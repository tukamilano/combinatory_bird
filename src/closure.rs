//! 不動点まで反復する閉包探索。
use crate::reduce::{Limits, DEFAULT_LIMITS};
use crate::term::{Arena, Birds, Id};
use rustc_hash::FxHashMap;

pub const MAX_ARGS: usize = 8;
pub const SIZE_STEPS: [usize; 4] = [4, 6, 8, 10];
const MAX_ARENA_NODES: usize = 5_000_000;
/// これより大きい正規形を持つ項は代表にしない(未決定が増えるだけで誤判定はしない)。
pub const MAX_REP_NODES: u32 = 128;

/// 既知の導出(Rathman の表など)。どれも照合を通して確かめるので、誤りがあっても無視されるだけ。
const KNOWN: [(&str, &str); 46] = [
    ("S", "B (B (B W) C) (B B)"), ("I", "W K"), ("I", "S K K"), ("B", "S (K S) K"),
    ("C", "R R R"), ("C", "B (T (B B T)) (B B T)"), ("C", "S (B B S) (K K)"),
    ("R", "B B T"), ("R", "C C"), ("T", "C I"), ("W", "C (B M R)"), ("W", "S S (K I)"),
    ("M", "S I I"), ("M", "W I"), ("B1", "B B B"), ("B2", "B (B B B) B"), ("B3", "B (B B) B"),
    ("D", "B B"), ("D1", "B (B B)"), ("D2", "B B (B B)"), ("E", "B (B B B)"),
    ("Ê", "B (B B B) (B (B B B))"), ("F", "E T T E T"), ("G", "B B C"),
    ("H", "B W (B C)"), ("J", "B (B C) (W (B C (B (B B B))))"), ("L", "C B M"),
    ("M2", "B M"), ("O", "S I"), ("Q", "C B"), ("Q1", "B C B"), ("Q2", "C (B C B)"),
    ("Q3", "B T"), ("U", "L O"), ("V", "B C T"), ("W1", "C W"), ("W*", "B W"),
    ("W**", "B (B W)"), ("C*", "B C"), ("R*", "C* C*"), ("F*", "B C* R*"), ("V*", "C* F*"),
    ("C**", "B C*"), ("R**", "B R*"), ("F**", "B F*"), ("V**", "B V*"),
];

/// 「原子 `mask` が揃えば `expr` で `target` を定義できるはず」という規則。
#[derive(Clone, Debug)]
pub struct Rule {
    pub target: usize,
    pub mask: u64,
    pub expr: String,
}

fn tokens(expr: &str) -> impl Iterator<Item = &str> {
    expr.split(|c: char| c == ' ' || c == '(' || c == ')')
        .filter(|s| !s.is_empty())
}

impl Rule {
    pub fn new(birds: &Birds, target: usize, expr: &str) -> Rule {
        let mask = tokens(expr).fold(0, |m, t| m | 1u64 << birds.index(t));
        Rule {
            target,
            mask,
            expr: expr.to_string(),
        }
    }
}

pub fn base_rules(birds: &Birds) -> Vec<Rule> {
    KNOWN
        .iter()
        .map(|(t, e)| Rule::new(birds, birds.index(t), e))
        .collect()
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Weak,
    Ext,
}

#[derive(Clone, Copy, Debug)]
pub struct Budget {
    pub candidates: u64,
    pub limits: Limits,
    pub max_size: usize,
}

impl Budget {
    pub fn new(candidates: u64) -> Budget {
        Budget {
            candidates,
            limits: DEFAULT_LIMITS,
            max_size: 10,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Deriv {
    pub bird: usize,
    pub witness: String,
}

#[derive(Clone, Debug, Default)]
pub struct ClosureOut {
    pub closure: u64,
    pub chain: Vec<Deriv>,
    pub truncated: bool,
    pub candidates: u64,
}

struct Rep {
    size: usize,
    val: Id,
    wit: Id,
    gen: u32,
}

struct Search<'a> {
    birds: &'a Birds,
    mode: Mode,
    universe: u64,
    budget: Budget,
    arena: Arena,
    vars: Vec<Id>,
    reps: Vec<Rep>,
    buckets: Vec<Vec<u32>>,
    keys: FxHashMap<(u8, Id), u32>,
    /// 弱簡約版は k ごと、外延版は k = MAX_ARGS のみを使う。
    targets: Vec<FxHashMap<Id, Vec<usize>>>,
    atoms: u64,
    pending: Vec<(usize, Id)>,
    pending_mask: u64,
    gen: u32,
    done: u64,
    goal: u64,
    rules: &'a [Rule],
}

impl Search<'_> {
    fn unresolved(&self, bird: usize) -> bool {
        (self.atoms | self.pending_mask) >> bird & 1 == 0 && self.universe >> bird & 1 == 1
    }

    /// k 個の変数を順に適用した正規形。
    fn nf_args(&mut self, start: Id, from: usize, to: usize) -> Option<Id> {
        let mut cur = start;
        let mut last = None;
        for i in from..to {
            cur = self.arena.app(cur, self.vars[i]);
            last = self.arena.nf(cur, self.birds, self.budget.limits);
            match last {
                Some(n) => cur = n,
                None => return None,
            }
        }
        last
    }

    fn check_targets(&mut self, val: Id, nf8: Option<Id>, wit: Id) {
        match self.mode {
            Mode::Ext => {
                let Some(n) = nf8 else { return };
                let Some(hits) = self.targets[MAX_ARGS].get(&n).cloned() else {
                    return;
                };
                for b in hits {
                    self.resolve(b, wit);
                }
            }
            Mode::Weak => {
                let mut max_k = 0;
                for (i, b) in self.birds.list.iter().enumerate() {
                    if self.unresolved(i) {
                        max_k = max_k.max(b.arity);
                    }
                }
                let mut cur = val;
                for k in 1..=max_k {
                    cur = self.arena.app(cur, self.vars[k - 1]);
                    let Some(n) = self.arena.nf(cur, self.birds, self.budget.limits) else {
                        return;
                    };
                    cur = n;
                    if let Some(hits) = self.targets[k].get(&n).cloned() {
                        for b in hits {
                            self.resolve(b, wit);
                        }
                    }
                }
            }
        }
    }

    fn resolve(&mut self, bird: usize, wit: Id) {
        if self.unresolved(bird) {
            self.pending.push((bird, wit));
            self.pending_mask |= 1u64 << bird;
        }
    }

    fn insert_rep(&mut self, size: usize, val: Id, wit: Id) -> u32 {
        let idx = self.reps.len() as u32;
        self.reps.push(Rep {
            size,
            val,
            wit,
            gen: self.gen,
        });
        self.buckets[size].push(idx);
        idx
    }

    fn improve(&mut self, idx: u32, size: usize, val: Id, wit: Id) {
        let gen = self.gen;
        let r = &mut self.reps[idx as usize];
        if size < r.size {
            *r = Rep {
                size,
                val,
                wit,
                gen,
            };
            self.buckets[size].push(idx);
        }
    }

    /// 項を 1 つ処理する。`raw` は値どうしの適用、`wit` は原子上の証拠項。
    /// 正規形がない項と、正規形が大きすぎる項は代表にしない。
    fn process(&mut self, raw: Id, wit: Id, size: usize) {
        let Some(val) = self.arena.nf(raw, self.birds, self.budget.limits) else {
            return;
        };
        if self.arena.tree_size(val) > MAX_REP_NODES {
            return;
        }
        if let Some(&idx) = self.keys.get(&(0, val)) {
            self.improve(idx, size, val, wit);
            return;
        }
        // 弱簡約版は k = 0 の正規形だけがキー。外延版は k = MAX_ARGS も使う。
        let nf8 = match self.mode {
            Mode::Weak => None,
            Mode::Ext => self.nf_args(val, 0, MAX_ARGS),
        };
        if let Some(n) = nf8 {
            if let Some(&idx) = self.keys.get(&(MAX_ARGS as u8, n)) {
                self.keys.insert((0, val), idx);
                self.improve(idx, size, val, wit);
                return;
            }
        }
        let idx = self.insert_rep(size, val, wit);
        self.keys.insert((0, val), idx);
        if let Some(n) = nf8 {
            self.keys.insert((MAX_ARGS as u8, n), idx);
        }
        self.check_targets(val, nf8, wit);
    }

    fn apply_rules(&mut self) {
        for rule in self.rules {
            if self.unresolved(rule.target) && rule.mask & !self.atoms == 0 {
                let t = self.arena.parse(&rule.expr, self.birds);
                let size = tokens(&rule.expr).count().min(self.buckets.len() - 1);
                self.process(t, t, size);
            }
        }
    }

    /// 定義できた鳥を原子に加える。新しい鳥があれば `true`。
    fn commit(&mut self, out: &mut ClosureOut) -> bool {
        let new = std::mem::take(&mut self.pending);
        self.pending_mask = 0;
        for &(b, wit) in &new {
            out.chain.push(Deriv {
                bird: b,
                witness: self.arena.show(wit, self.birds),
            });
            self.atoms |= 1u64 << b;
        }
        for &(b, _) in &new {
            let t = self.arena.bird(b);
            self.process(t, t, 1);
        }
        !new.is_empty()
    }

    fn out_of_budget(&self) -> bool {
        self.done >= self.budget.candidates || self.arena.nodes.len() > MAX_ARENA_NODES
    }

    /// サイズ上限 `limit` で 1 周する。`false` なら予算切れ。
    fn pass(&mut self, limit: usize, prev_limit: usize) -> bool {
        for n in 2..=limit {
            for i in 1..n {
                let j = n - i;
                let (la, lb) = (self.buckets[i].len(), self.buckets[j].len());
                for x in 0..la {
                    let ra = self.buckets[i][x] as usize;
                    if self.reps[ra].size != i {
                        continue;
                    }
                    let (va, wa, ga) = {
                        let r = &self.reps[ra];
                        (r.val, r.wit, r.gen)
                    };
                    for y in 0..lb {
                        let rb = self.buckets[j][y] as usize;
                        let r = &self.reps[rb];
                        if r.size != j || (ga < self.gen && r.gen < self.gen && n <= prev_limit) {
                            continue;
                        }
                        let (vb, wb) = (r.val, r.wit);
                        if self.out_of_budget() {
                            return false;
                        }
                        self.done += 1;
                        let raw = self.arena.app(va, vb);
                        let wit = self.arena.app(wa, wb);
                        self.process(raw, wit, n);
                        if self.atoms & self.goal == self.goal {
                            return true;
                        }
                    }
                }
            }
        }
        true
    }
}

/// `seed` を原子として閉包を計算する。`universe` にない鳥は照合しない。
pub fn closure(
    birds: &Birds,
    mode: Mode,
    seed: u64,
    universe: u64,
    budget: Budget,
    rules: &[Rule],
) -> ClosureOut {
    let goal = birds.mask(&["S", "K"]);
    let mut s = Search {
        birds,
        mode,
        universe,
        budget,
        arena: Arena::new(),
        vars: Vec::new(),
        reps: Vec::new(),
        buckets: vec![Vec::new(); budget.max_size + 16],
        keys: FxHashMap::default(),
        targets: vec![FxHashMap::default(); MAX_ARGS + 1],
        atoms: 0,
        pending: Vec::new(),
        pending_mask: 0,
        gen: 1,
        done: 0,
        goal,
        rules,
    };
    s.vars = (0..MAX_ARGS).map(|i| s.arena.var(i)).collect();
    for (i, b) in birds.list.iter().enumerate() {
        let mut t = s.arena.instantiate(&b.rhs, &s.vars[..b.arity]);
        let k = match mode {
            Mode::Weak => b.arity,
            Mode::Ext => {
                for v in b.arity..MAX_ARGS {
                    t = s.arena.app(t, s.vars[v]);
                }
                MAX_ARGS
            }
        };
        s.targets[k].entry(t).or_default().push(i);
    }
    let mut out = ClosureOut::default();
    // 先に全原子を登録し、シードどうしが互いの「導出」として記録されないようにする。
    s.atoms = seed;
    for b in 0..birds.list.len() {
        if seed >> b & 1 == 1 {
            let t = s.arena.bird(b);
            s.process(t, t, 1);
        }
    }
    let mut level = 0;
    let mut prev_limit = 0;
    let steps: Vec<usize> = SIZE_STEPS
        .iter()
        .copied()
        .filter(|&x| x <= budget.max_size)
        .collect();
    loop {
        // 規則だけで増える分を先に使い切る。
        loop {
            s.apply_rules();
            if !s.commit(&mut out) {
                break;
            }
        }
        if s.atoms & goal == goal {
            break;
        }
        let limit = steps[level];
        let finished = s.pass(limit, prev_limit);
        prev_limit = limit;
        s.gen += 1;
        let grew = s.commit(&mut out);
        if !finished {
            out.truncated = true;
            break;
        }
        if s.atoms & goal == goal {
            break;
        }
        if !grew {
            level += 1;
            if level == steps.len() {
                break;
            }
        }
    }
    out.closure = s.atoms;
    out.candidates = s.done;
    out
}
