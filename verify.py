"""証拠項の独立検算。Rust のコードを共有せず、木の素朴な簡約で確かめる。

使い方: verify.py results/weak.jsonl results/ext.jsonl
"""
import json
import re
import sys
from pathlib import Path

sys.setrecursionlimit(100000)
ROOT = Path(__file__).parent
BIRDS = {b["symbol"]: b for b in json.loads((ROOT / "birds.json").read_text(encoding="utf-8"))}
MAX_STEPS = 20000
MAX_SIZE = 200000
MAX_EXTRA = 8
EXPAND_LIMIT = 200000


class Exceeded(Exception):
    pass


def parse(text):
    toks = re.findall(r"[()]|[^\s()]+", text)
    pos = 0

    def app():
        nonlocal pos
        term = None
        while pos < len(toks) and toks[pos] != ")":
            if toks[pos] == "(":
                pos += 1
                atom = app()
                pos += 1
            else:
                atom = toks[pos]
                if atom not in BIRDS:
                    raise ValueError(f"未知の鳥 {atom}")
                pos += 1
            term = atom if term is None else (term, atom)
        return term

    return app()


def subst(rhs, args):
    if isinstance(rhs, int):
        return args[rhs]
    return (subst(rhs[0], args), subst(rhs[1], args))


def size(t):
    total, stack = 0, [t]
    while stack:
        x = stack.pop()
        total += 1
        if isinstance(x, tuple):
            stack.extend(x)
    return total


def normalize(t, budget):
    while True:
        args = []
        h = t
        while isinstance(h, tuple):
            args.append(h[1])
            h = h[0]
        args.reverse()
        if isinstance(h, str) and h in BIRDS and len(args) >= BIRDS[h]["arity"]:
            budget[0] -= 1
            if budget[0] < 0:
                raise Exceeded
            n = BIRDS[h]["arity"]
            t = subst(BIRDS[h]["rhs"], args[:n])
            for a in args[n:]:
                t = (t, a)
            continue
        res = h
        for a in args:
            res = (res, normalize(a, budget))
        return res


def variable(i):
    return ("var", i)


def var_rhs(rhs):
    if isinstance(rhs, int):
        return variable(rhs)
    return (var_rhs(rhs[0]), var_rhs(rhs[1]))


def apply_vars(t, start, stop):
    for i in range(start, stop):
        t = (t, variable(i))
    return t


def normalize_vars(t, budget):
    """("var", i) を原子として扱う正規化。"""
    return normalize(encode(t), budget)


def encode(t):
    if isinstance(t, tuple) and t[0] == "var":
        return f"#{t[1]}"
    if isinstance(t, tuple):
        return (encode(t[0]), encode(t[1]))
    return t


def check(witness, bird, mode):
    """証拠項 `witness` が鳥 `bird` を定義することを確かめる。"""
    arity = BIRDS[bird]["arity"]
    rhs = var_rhs(BIRDS[bird]["rhs"])
    extras = range(0, 1) if mode == "weak" else range(0, MAX_EXTRA + 1)
    for extra in extras:
        m = arity + extra
        try:
            got = normalize_vars(apply_vars(witness, 0, m), [MAX_STEPS])
        except (Exceeded, RecursionError):
            continue
        if got == encode(apply_vars(rhs, arity, m)):
            return True
    return False


def expand(t, defs):
    if isinstance(t, tuple):
        return (expand(t[0], defs), expand(t[1], defs))
    return defs.get(t, t)


def verify_record(rec, mode):
    known = set(rec["birds"])
    defs = {}
    errors = []
    for step in rec["chain"]:
        w = parse(step["witness"])
        atoms = set(re.findall(r"[^\s()]+", step["witness"]))
        if not atoms <= known:
            errors.append(f"{step['bird']}: 未定義の原子 {sorted(atoms - known)}")
        if not check(w, step["bird"], mode):
            errors.append(f"{step['bird']} = {step['witness']}: 検算失敗")
        known.add(step["bird"])
        if size(w) < EXPAND_LIMIT:
            defs[step["bird"]] = expand(w, defs)
    for goal in ("S", "K"):
        if goal not in known:
            errors.append(f"{goal} の導出がありません")
    expanded = skipped = 0
    if mode == "weak" and not errors:
        for goal in ("S", "K"):
            if goal in defs:
                if size(defs[goal]) > EXPAND_LIMIT:
                    skipped += 1
                    continue
                try:
                    ok = check(defs[goal], goal, "weak")
                except Exceeded:
                    ok = None
                if ok:
                    expanded += 1
                else:
                    skipped += 1
    return errors, expanded, skipped


def main():
    failed = 0
    for path in sys.argv[1:]:
        mode = "weak" if "weak" in Path(path).name else "ext"
        checked = expanded = skipped = 0
        with open(path, encoding="utf-8") as f:
            for line in f:
                rec = json.loads(line)
                if rec["status"] != "complete":
                    continue
                errors, e, s = verify_record(rec, mode)
                checked += 1
                expanded += e
                skipped += s
                if errors:
                    failed += 1
                    print("NG", rec["birds"], errors[:3])
        print(f"{path}: 完全な組 {checked} 件を検算、展開検算 成功 {expanded} / 未確認 {skipped}")
    print("不一致:", failed)
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
