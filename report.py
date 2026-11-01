"""Classify search results and write summary.md.

Usage: report.py results/run1 [max_size]
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
    """Map to the unit searched in extensional mode. None (non-independent) if it has two or more I-like birds."""
    members = subset & I_CLASS
    if len(members) >= 2:
        return None
    return (subset - members) | ({"I"} if members else set())


def status_of(subset, records, mode):
    """Extensional mode looks up the result with I, I*, I** identified."""
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
    """Whether the set is provably incomplete. A non-independent set reduces to a subset with the same closure."""
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
        ("minimal", "Confirmed minimal"),
        ("candidate", "Minimal candidate"),
        ("incomplete", "Incomplete (proven)"),
        ("undecided", "Undecided"),
        ("non_minimal", "Non-minimal"),
    ]
    lines = [f"# Minimal complete bases (size {max_size} or less)", ""]
    lines += ["Minimal bases of size 5 or more are out of scope. Candidates and undecided sets come from a bounded search and are not proofs.", ""]
    lines += ["| Class | Weak | Extensional |", "|---|---:|---:|"]
    for key, label in labels:
        lines.append(f"| {label} | {len(weak[key])} | {len(ext[key])} |")
    lines.append(f"| Total | {sum(map(len, weak.values()))} | {sum(map(len, ext.values()))} |")
    for key, label in labels[:2]:
        for name, groups in (("weak", weak), ("extensional", ext)):
            lines += ["", f"## {label} ({name})", ""]
            lines.append(", ".join(fmt(c) for c in groups[key]) or "None")

    def classes(groups):
        return {frozenset(c): k for k, v in groups.items() for c in v}

    cw, ce = classes(weak), classes(ext)
    diff = [(c, cw[c], ce[c]) for c in cw if cw[c] != ce[c]]
    bad = [c for c, a, b in diff if a in ("minimal", "candidate") and b in ("incomplete", "undecided")]
    lines += ["", "## Differences between weak and extensional", "", f"Sets classified differently: {len(diff)}", ""]
    lines += ["Sets complete in weak mode but incomplete or undecided in extensional mode: " + str(len(bad)), ""]
    interesting = [d for d in diff if {d[1], d[2]} & {"minimal", "candidate"}]
    lines += ["Differences affecting completeness:", ""]
    lines += [f"- {fmt(sorted(c))}: weak={a} / extensional={b}" for c, a, b in sorted(interesting, key=lambda d: (len(d[0]), sorted(d[0])))[:300]]
    text = "\n".join(lines) + "\n"
    (out / "summary.md").write_text(text, encoding="utf-8")
    print(text[:6000])


if __name__ == "__main__":
    main()
