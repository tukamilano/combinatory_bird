"""birds.csv を検証し、固有の鳥だけを birds.json に出力する。"""
import csv
import json
from pathlib import Path

ROOT = Path(__file__).parent


def parse_body(body, params):
    """束縛変数のみからなる左結合の適用式を、int と [l, r] の入れ子に変換する。"""
    pos = 0

    def atom():
        nonlocal pos
        if pos >= len(body):
            raise ValueError(f"式が途中で終わっています: {body}")
        ch = body[pos]
        if ch == "(":
            pos += 1
            inner = app()
            if pos >= len(body) or body[pos] != ")":
                raise ValueError(f"括弧が閉じていません: {body}")
            pos += 1
            return inner
        if ch not in params:
            raise ValueError(f"束縛されていない記号 {ch}: {body}")
        pos += 1
        return params.index(ch)

    def app():
        term = atom()
        while pos < len(body) and body[pos] != ")":
            term = [term, atom()]
        return term

    term = app()
    if pos != len(body):
        raise ValueError(f"余分な文字があります: {body}")
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
                raise ValueError(f"{row['symbol']}: arity と束縛変数の数が不一致")
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
        raise SystemExit(f"47 羽のはずが {len(birds)} 羽です")
    (ROOT / "birds.json").write_text(
        json.dumps(birds, ensure_ascii=False, indent=1), encoding="utf-8"
    )
    print(f"{len(birds)} 羽を birds.json に出力しました")


if __name__ == "__main__":
    main()
