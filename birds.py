"""Validate birds.csv and write only the proper birds to birds.json."""
import csv
import json
from pathlib import Path

ROOT = Path(__file__).parent


def parse_body(body, params):
    """Convert a left-associative application of bound variables into nested ints and [l, r] pairs."""
    pos = 0

    def atom():
        nonlocal pos
        if pos >= len(body):
            raise ValueError(f"unexpected end of expression: {body}")
        ch = body[pos]
        if ch == "(":
            pos += 1
            inner = app()
            if pos >= len(body) or body[pos] != ")":
                raise ValueError(f"unclosed parenthesis: {body}")
            pos += 1
            return inner
        if ch not in params:
            raise ValueError(f"unbound symbol {ch}: {body}")
        pos += 1
        return params.index(ch)

    def app():
        term = atom()
        while pos < len(body) and body[pos] != ")":
            term = [term, atom()]
        return term

    term = app()
    if pos != len(body):
        raise ValueError(f"trailing characters: {body}")
    return term


def load(path=ROOT / "birds.csv", in_book_only=False):
    birds = []
    with open(path, encoding="utf-8") as f:
        for row in csv.DictReader(f):
            if row["proper"] != "yes":
                continue
            if in_book_only and row["in_book"] != "yes":
                continue
            head, body = row["lambda"].removeprefix("λ").split(".", 1)
            if len(head) != int(row["arity"]):
                raise ValueError(f"{row['symbol']}: arity does not match the number of bound variables")
            birds.append(
                {
                    "symbol": row["symbol"],
                    "name": row["name"],
                    "arity": len(head),
                    "rhs": parse_body(body, head),
                }
            )
    return birds


def main():
    birds = load()
    if len(birds) != 47:
        raise SystemExit(f"expected 47 birds, found {len(birds)}")
    (ROOT / "birds.json").write_text(
        json.dumps(birds, ensure_ascii=False, indent=1), encoding="utf-8"
    )
    print(f"wrote {len(birds)} birds to birds.json")


if __name__ == "__main__":
    main()
