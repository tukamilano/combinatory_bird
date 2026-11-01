import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))
import birds as B


def by_symbol():
    return {b["symbol"]: b for b in B.load()}


def test_count_and_excluded():
    d = by_symbol()
    assert len(d) == 47
    assert not {"Y", "ω", "Θ"} & set(d)


def test_vstar_is_corrected():
    # λabcd.adbc = ((a d) b) c
    assert by_symbol()["V*"]["rhs"] == [[[0, 3], 1], 2]


def test_parse_examples():
    d = by_symbol()
    assert d["S"]["rhs"] == [[0, 2], [1, 2]]
    assert d["B"]["rhs"] == [0, [1, 2]]
    assert d["I"]["rhs"] == 0
