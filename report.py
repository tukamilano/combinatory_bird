"""探索結果を分類し、summary.md を出力する。

使い方: report.py results/run1
"""
import itertools
import json
import sys
from pathlib import Path

ROOT = Path(__file__).parent
SYMBOLS = [b["symbol"] for b in json.loads((ROOT / "birds.json").read_text(encoding="utf-8"))]
I_CLASS = {"I", "I*", "I**"}
TOTAL = 195708


def load(path):
    records = {}
    with open(path, encoding="utf-8") as f:
        for line in f:
            r = json.loads(line)
            records[frozenset(r["birds"])] = r
    return records


def canonical(subset):
    """外延版の探索単位に写す。I の仲間が 2 羽以上なら None(非独立)。"""
    members = subset & I_CLASS
    if len(members) >= 2:
        return None
    return (subset - members) | ({"I"} if members else set())


def status_of(subset, records, mode):
    """外延版は I, I*, I** を同一視した結果を引く。"""
    if mode == "weak":
        return records[subset]["status"]
    c = canonical(subset)
    if c is None:
        return "non_independent"
    return records[frozenset(c)]["status"]


def closure_of(subset, records, mode):
    key = subset if mode == "weak" else frozenset(canonical(subset) or subset)
    return set(records[key]["closure"]) if key in records else set()


def proven_incomplete(subset, records, mode, memo):
    """組が不完全だと証明できるか。非独立の組は、閉包が等しい部分集合に帰着する。"""
    if subset in memo:
        return memo[subset]
    status = status_of(subset, records, mode)
    result = False
    if status == "incomplete":
        result = True
    elif status == "non_independent" and len(subset) > 1:
        for x in subset:
            rest = subset - {x}
            if x in closure_of(rest, records, mode) and proven_incomplete(rest, records, mode, memo):
                result = True
                break
    memo[subset] = result
    return result


def classify(records, mode, max_size):
    groups = {"minimal": [], "candidate": [], "incomplete": [], "undecided": [], "non_minimal": []}
    memo = {}
    for k in range(1, max_size + 1):
        for combo in itertools.combinations(SYMBOLS, k):
            subset = frozenset(combo)
            status = status_of(subset, records, mode)
            if status == "complete":
                parents = [subset - {x} for x in subset] if k > 1 else []
                if all(proven_incomplete(p, records, mode, memo) for p in parents):
                    groups["minimal"].append(combo)
                else:
                    groups["candidate"].append(combo)
            elif status == "incomplete":
                groups["incomplete"].append(combo)
            elif status == "undecided":
                groups["undecided"].append(combo)
            else:
                groups["non_minimal"].append(combo)
    return groups


def fmt(combo):
    return "{" + ",".join(combo) + "}"


def main():
    out = Path(sys.argv[1])
    max_size = int(sys.argv[2]) if len(sys.argv) > 2 else 4
    weak = classify(load(out / "weak.jsonl"), "weak", max_size)
    ext = classify(load(out / "ext.jsonl"), "ext", max_size)
    labels = [
        ("minimal", "極小確定"),
        ("candidate", "極小候補"),
        ("incomplete", "不完全(証明あり)"),
        ("undecided", "未決定"),
        ("non_minimal", "非極小"),
    ]
    lines = [f"# 極小完全基底の探索結果(サイズ {max_size} 以下)", ""]
    lines += ["サイズ 5 以上の極小基底は範囲外。候補と未決定は上限つきの探索の結果であり、証明ではない。", ""]
    lines += ["| 分類 | 弱簡約 | 外延 |", "|---|---:|---:|"]
    for key, label in labels:
        lines.append(f"| {label} | {len(weak[key])} | {len(ext[key])} |")
    lines.append(f"| 合計 | {sum(map(len, weak.values()))} | {sum(map(len, ext.values()))} |")
    for key, label in labels[:2]:
        for name, groups in (("弱簡約", weak), ("外延", ext)):
            lines += ["", f"## {label}({name})", ""]
            lines.append(", ".join(fmt(c) for c in groups[key]) or "なし")

    def classes(groups):
        return {frozenset(c): k for k, v in groups.items() for c in v}

    cw, ce = classes(weak), classes(ext)
    diff = [(c, cw[c], ce[c]) for c in cw if cw[c] != ce[c]]
    bad = [c for c, a, b in diff if a in ("minimal", "candidate") and b in ("incomplete", "undecided")]
    lines += ["", "## 弱簡約と外延の差分", "", f"分類が異なる組: {len(diff)}", ""]
    lines += ["弱簡約で完全なのに、外延で不完全または未決定の組: " + str(len(bad)), ""]
    interesting = [d for d in diff if {d[1], d[2]} & {"minimal", "candidate"}]
    lines += ["完全性に関わる差分:", ""]
    lines += [f"- {fmt(sorted(c))}: 弱簡約={a} / 外延={b}" for c, a, b in sorted(interesting, key=lambda d: (len(d[0]), sorted(d[0])))[:300]]
    text = "\n".join(lines) + "\n"
    (out / "summary.md").write_text(text, encoding="utf-8")
    print(text[:6000])


if __name__ == "__main__":
    main()
