//! 部分集合の全探索。サイズの昇順に処理し、下のレベルの閉包を初期値に使う。
use crate::closure::{base_rules, closure, Budget, ClosureOut, Deriv, Mode, Rule};
use crate::invariant::certificate;
use crate::term::Birds;
use rayon::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};
use serde_json::{json, Value};
use std::io::Write;
use std::path::Path;
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    Complete,
    Incomplete,
    Undecided,
    Superset,
    NonIndependent,
}

impl Status {
    pub fn name(self) -> &'static str {
        match self {
            Status::Complete => "complete",
            Status::Incomplete => "incomplete",
            Status::Undecided => "undecided",
            Status::Superset => "superset",
            Status::NonIndependent => "non_independent",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Rec {
    pub mask: u64,
    pub status: Status,
    pub cert: Option<String>,
    pub closure: u64,
    pub chain: Vec<Deriv>,
    pub truncated: bool,
}

pub fn combinations(items: &[usize], k: usize) -> Vec<u64> {
    fn go(items: &[usize], k: usize, start: usize, m: u64, out: &mut Vec<u64>) {
        if k == 0 {
            out.push(m);
            return;
        }
        for i in start..items.len() {
            go(items, k - 1, i + 1, m | 1u64 << items[i], out);
        }
    }
    let mut out = Vec::new();
    go(items, k, 0, 0, &mut out);
    out
}

fn bits(mask: u64) -> Vec<usize> {
    (0..64).filter(|i| mask >> i & 1 == 1).collect()
}

fn to_json(r: &Rec, birds: &Birds) -> Value {
    json!({
        "birds": birds.symbols(r.mask),
        "status": r.status.name(),
        "cert": r.cert,
        "closure": birds.symbols(r.closure),
        "chain": r.chain.iter().map(|d| json!({
            "bird": birds.list[d.bird].symbol, "witness": d.witness
        })).collect::<Vec<_>>(),
        "truncated": r.truncated,
    })
}

fn from_json(v: &Value, birds: &Birds) -> Rec {
    let mask_of = |a: &Value| {
        a.as_array()
            .unwrap()
            .iter()
            .fold(0u64, |m, s| m | 1u64 << birds.index(s.as_str().unwrap()))
    };
    let status = match v["status"].as_str().unwrap() {
        "complete" => Status::Complete,
        "incomplete" => Status::Incomplete,
        "undecided" => Status::Undecided,
        "superset" => Status::Superset,
        _ => Status::NonIndependent,
    };
    Rec {
        mask: mask_of(&v["birds"]),
        status,
        cert: v["cert"].as_str().map(str::to_string),
        closure: mask_of(&v["closure"]),
        chain: v["chain"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| Deriv {
                bird: birds.index(d["bird"].as_str().unwrap()),
                witness: d["witness"].as_str().unwrap().to_string(),
            })
            .collect(),
        truncated: v["truncated"].as_bool().unwrap(),
    }
}

/// 外延版の探索対象。I* と I** は I と外延的に等しいので除く。
pub fn universe(birds: &Birds, mode: Mode) -> u64 {
    let all = (1u64 << birds.list.len()) - 1;
    match mode {
        Mode::Weak => all,
        Mode::Ext => all & !birds.mask(&["I*", "I**"]),
    }
}

/// 弱簡約版で完全な組を、外延版の組に写して引き継ぐ。
fn inherited_complete(birds: &Birds, weak: &[Rec]) -> FxHashMap<u64, Vec<Deriv>> {
    let i_class = birds.mask(&["I*", "I**"]);
    let i = birds.index("I");
    let mut out = FxHashMap::default();
    for r in weak.iter().filter(|r| r.status == Status::Complete) {
        let mut mask = r.mask;
        let mut chain = Vec::new();
        for b in bits(mask & i_class) {
            mask &= !(1u64 << b);
            mask |= 1u64 << i;
            chain.push(Deriv {
                bird: b,
                witness: "I".to_string(),
            });
        }
        chain.extend(r.chain.iter().cloned());
        out.entry(mask).or_insert(chain);
    }
    out
}

pub fn load_records(path: &Path, birds: &Birds) -> Vec<Rec> {
    match std::fs::read_to_string(path) {
        Ok(text) => text
            .lines()
            .map(|l| from_json(&serde_json::from_str(l).unwrap(), birds))
            .collect(),
        Err(_) => Vec::new(),
    }
}

pub struct Config<'a> {
    pub birds: &'a Birds,
    pub mode: Mode,
    pub max_subset: usize,
    pub budget: Budget,
    pub out: &'a Path,
    pub inherit: Vec<Rec>,
}

/// 部分集合をレベル順に処理する。レベルが揃って書き込まれていれば、そこから再開する。
pub fn run(cfg: &Config) -> Vec<Rec> {
    let birds = cfg.birds;
    let uni = universe(birds, cfg.mode);
    let items = bits(uni);
    let goal = birds.mask(&["S", "K"]);
    let inherited = inherited_complete(birds, &cfg.inherit);

    let mut all: Vec<Rec> = Vec::new();
    let mut prev: FxHashMap<u64, Rec> = FxHashMap::default();
    let mut memo: FxHashMap<u64, Arc<ClosureOut>> = FxHashMap::default();
    let mut start = 1;
    let mut rules = base_rules(birds);

    // 再開:レベルが完全に揃っている分だけ読み込む。
    let old = load_records(cfg.out, birds);
    let mut by_level: FxHashMap<usize, Vec<Rec>> = FxHashMap::default();
    for r in old {
        by_level.entry(r.mask.count_ones() as usize).or_default().push(r);
    }
    let mut writer = {
        let mut kept = Vec::new();
        for k in 1..=cfg.max_subset {
            let expected = combinations(&items, k).len();
            match by_level.remove(&k) {
                Some(rs) if rs.len() == expected => {
                    prev = rs.iter().map(|r| (r.mask, r.clone())).collect();
                    kept.extend(rs);
                    start = k + 1;
                }
                _ => break,
            }
        }
        let mut f = std::io::BufWriter::new(std::fs::File::create(cfg.out).unwrap());
        for r in &kept {
            writeln!(f, "{}", to_json(r, birds)).unwrap();
        }
        f.flush().unwrap();
        all.extend(kept);
        f
    };
    if start > 1 {
        eprintln!("[{:?}] レベル {} までを再利用します", cfg.mode, start - 1);
    }

    for k in start..=cfg.max_subset {
        let t0 = std::time::Instant::now();
        let subsets = combinations(&items, k);

        // 1 段目:閉包を計算せずに決まるものを決め、残りの初期値を集める。
        struct Prep {
            mask: u64,
            cert: Option<&'static str>,
            verdict: Option<Status>,
            seed: u64,
            chain: Vec<Deriv>,
            needs_run: bool,
        }
        let is_last = k == cfg.max_subset;
        let preps: Vec<Prep> = subsets
            .par_iter()
            .map(|&mask| {
                let parents: Vec<u64> = bits(mask).iter().map(|b| mask & !(1u64 << b)).collect();
                let cert = certificate(birds, mask);
                let mut p = Prep {
                    mask,
                    cert,
                    verdict: None,
                    seed: mask,
                    chain: Vec::new(),
                    needs_run: false,
                };
                if k > 1 && parents.iter().any(|q| prev[q].status == Status::Complete
                    || prev[q].status == Status::Superset)
                {
                    p.verdict = Some(Status::Superset);
                    return p;
                }
                if let Some(chain) = inherited.get(&mask) {
                    p.verdict = Some(Status::Complete);
                    p.chain = chain.clone();
                    return p;
                }
                if cert.is_some() && is_last {
                    p.verdict = Some(Status::Incomplete);
                    return p;
                }
                let mut independent = true;
                if k > 1 {
                    for b in bits(mask) {
                        let q = &prev[&(mask & !(1u64 << b))];
                        if q.closure >> b & 1 == 1 {
                            independent = false;
                        }
                        p.seed |= q.closure;
                        for d in &q.chain {
                            if !p.chain.iter().any(|e| e.bird == d.bird) && mask >> d.bird & 1 == 0 {
                                p.chain.push(d.clone());
                            }
                        }
                    }
                }
                if !independent {
                    p.verdict = Some(if cert.is_some() {
                        Status::Incomplete
                    } else {
                        Status::NonIndependent
                    });
                    return p;
                }
                if p.seed & goal == goal {
                    p.verdict = Some(Status::Complete);
                    return p;
                }
                p.needs_run = true;
                p
            })
            .collect();

        // 2 段目:初期値が同じ組は 1 回だけ閉包を計算する。
        // 不変量で不完全な組は初期値を作るだけなので、予算を 1/10 にする。
        let full: FxHashSet<u64> = preps
            .iter()
            .filter(|p| p.needs_run && p.cert.is_none())
            .map(|p| p.seed)
            .collect();
        let seeds: Vec<u64> = preps
            .iter()
            .filter(|p| p.needs_run && !memo.contains_key(&p.seed))
            .map(|p| p.seed)
            .collect::<FxHashSet<_>>()
            .into_iter()
            .collect();
        let n_runs = seeds.len();
        let results: Vec<(u64, ClosureOut)> = seeds
            .par_iter()
            .map(|&seed| {
                let mut budget = cfg.budget;
                if !full.contains(&seed) {
                    budget.candidates /= 10;
                }
                (seed, closure(birds, cfg.mode, seed, uni, budget, &rules))
            })
            .collect();
        for (seed, out) in results {
            // 見つけた導出を規則に加える。同じ鳥をより少ない原子で作れる規則があれば加えない。
            for d in &out.chain {
                let rule = Rule::new(birds, d.bird, &d.witness);
                if !rules
                    .iter()
                    .any(|r| r.target == rule.target && r.mask & !rule.mask == 0)
                {
                    rules.push(rule);
                }
            }
            let fixed = out.closure;
            let truncated = out.truncated;
            memo.insert(seed, Arc::new(out));
            memo.entry(fixed).or_insert_with(|| {
                Arc::new(ClosureOut {
                    closure: fixed,
                    chain: Vec::new(),
                    truncated,
                    candidates: 0,
                })
            });
        }

        // 3 段目:記録を確定する。
        let mut level = Vec::with_capacity(preps.len());
        for p in preps {
            let mut rec = Rec {
                mask: p.mask,
                status: p.verdict.unwrap_or(Status::Undecided),
                cert: p.cert.map(str::to_string),
                closure: p.seed,
                chain: p.chain,
                truncated: false,
            };
            if p.needs_run {
                let out = &memo[&p.seed];
                rec.closure = out.closure;
                rec.truncated = out.truncated;
                for d in &out.chain {
                    if !rec.chain.iter().any(|e| e.bird == d.bird) {
                        rec.chain.push(d.clone());
                    }
                }
                rec.status = if out.closure & goal == goal {
                    Status::Complete
                } else if p.cert.is_some() {
                    Status::Incomplete
                } else {
                    Status::Undecided
                };
            }
            if p.cert.is_some() && rec.status == Status::Complete {
                panic!("不変量に反します: {:?}", birds.symbols(p.mask));
            }
            level.push(rec);
        }
        for r in &level {
            // 最終レベルの非完全な組は導出を残さない(ファイルサイズ削減)。
            if is_last && r.status != Status::Complete {
                let mut slim = r.clone();
                slim.chain.clear();
                writeln!(writer, "{}", to_json(&slim, birds)).unwrap();
            } else {
                writeln!(writer, "{}", to_json(r, birds)).unwrap();
            }
        }
        writer.flush().unwrap();

        let mut counts: FxHashMap<&str, usize> = FxHashMap::default();
        for r in &level {
            *counts.entry(r.status.name()).or_default() += 1;
        }
        let mut c: Vec<_> = counts.into_iter().collect();
        c.sort();
        eprintln!(
            "[{:?}] size={k} 組={} 閉包計算={n_runs} {:.1}s {:?}",
            cfg.mode,
            level.len(),
            t0.elapsed().as_secs_f64(),
            c
        );
        prev = level.iter().map(|r| (r.mask, r.clone())).collect();
        all.extend(level);
    }
    all
}
