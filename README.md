# combinatory_bird

This project enumerates the minimal combinatorially complete sets of the birds (combinators) from Raymond Smullyan's *To Mock a Mockingbird*.

A set is combinatorially complete if it can define S and K. Definability is decided in two ways, weak reduction and extensional equality, and the results are compared.

## Birds

The search covers the 47 proper birds in `birds.csv`. Y, ω and Θ are excluded because they are not proper. `birds.py` validates the CSV and generates `birds.json`.

The list of birds is based on [Combinator Birds by Chris Rathman (Web Archive)](https://web.archive.org/web/20100307150810/http://www.angelfire.com/tx4/cus/combinator/birds.html).

## Setup

You need Rust (edition 2021) and Python 3.

```sh
# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
. ~/.cargo/env

# Python (only the tests need pytest; everything else uses the standard library)
python3 -m venv .venv
.venv/bin/pip install pytest

# Build
cargo build --release
```

## Usage

Run commands from the repository root, because the search reads `birds.json` by relative path.

```sh
# 1. Search
./target/release/combinatory_bird --max-subset 4 --out results/run1

# 2. Check the witnesses
.venv/bin/python verify.py results/run1/weak.jsonl results/run1/ext.jsonl

# 3. Summarize (writes results/run1/summary.md)
.venv/bin/python report.py results/run1 4
```

A run up to size 4 has not been tested and its running time is unknown. It covers about 200,000 sets, compared with about 1,100 up to size 2, so try `--max-subset 3` first.

### Search options

| Option | Default | Description |
|---|---|---|
| `--max-subset` | `4` | Largest set size to examine |
| `--mode` | `both` | `weak`, `ext` or `both` |
| `--budget` | `100000` | Maximum candidate terms per closure computation |
| `--out` | `results` | Output directory |

- `report.py` needs both `weak.jsonl` and `ext.jsonl`.
- The search uses all cores. Set `RAYON_NUM_THREADS` to limit the thread count.
- Rerunning with the same `--out` reuses every fully written size without recomputing it. To compare `--budget` values, use a different directory for each.

### Tests

```sh
cargo test --release
.venv/bin/python -m pytest -q
```

## How the search works

1. **Invariants:** If every bird in a set is non-erasing, it cannot define K. If every bird is non-duplicating, it cannot define S. Either case proves the set incomplete.
2. **Closure:** Terms are enumerated by size and weakly reduced, then the normal forms after applying variables are matched against the bird definitions. Each newly defined bird becomes an atom, and this repeats until no more birds appear.
3. **Matching:** Weak mode requires the number of arguments to match exactly. Extensional mode also accepts a match with extra trailing arguments.
4. **Order of sets:** Sets are processed by increasing size, each starting from the closures of its smaller subsets. Supersets of complete sets are not searched. Derivations that are found become rules for later sizes.
5. **Independent check:** `verify.py` confirms the witnesses with naive tree reduction and shares no code with the Rust search.

Extensional mode identifies `I`, `I*` and `I**`.

### Search limits

| Limit | Value | Defined in |
|---|---|---|
| Term size | Grows through 4, 6, 8, 10 | `SIZE_STEPS` in `src/closure.rs` |
| Arguments applied in extensional mode | 8 | `MAX_ARGS` in `src/closure.rs` |
| Reduction steps | 200 | `DEFAULT_LIMITS` in `src/reduce.rs` |
| Term nodes during reduction | 10,000 | `DEFAULT_LIMITS` in `src/reduce.rs` |
| Normal form nodes for a representative | 128 | `MAX_REP_NODES` in `src/closure.rs` |

## Output

`weak.jsonl` and `ext.jsonl` contain one record per set, one per line.

```json
{"birds":["KM"],"cert":null,"chain":[{"bird":"M","witness":"KM KM"}],"closure":["M","KM"],"status":"undecided","truncated":false}
```

| Field | Description |
|---|---|
| `birds` | The set examined |
| `status` | The verdict (see below) |
| `cert` | The invariant proving incompleteness (`non_erasing` or `non_duplicating`) |
| `closure` | The birds that could be defined |
| `chain` | The derivations, with each `witness` term |
| `truncated` | Whether the search stopped early, for example because of the candidate budget |

| `status` | Meaning |
|---|---|
| `complete` | S and K were defined |
| `incomplete` | Proven incomplete by an invariant |
| `undecided` | Neither could be established |
| `superset` | A superset of a complete set |
| `non_independent` | A bird in the set can be defined from the others |

`summary.md` groups these as Confirmed minimal, Minimal candidate, Incomplete (proven), Undecided and Non-minimal. A set is confirmed minimal if it is complete and every set with one bird removed is proven incomplete. If any of those smaller sets is not proven incomplete, the set is a minimal candidate.

## Interpreting the results

- The search is bounded, so undecided does not mean incomplete, and a minimal candidate is not proven minimal.
- Minimal bases of size 5 or more are out of scope.
- The current invariants cannot refute sets such as those containing `KM` or `CKM`, which both erase and duplicate, so these remain undecided.

## Repository layout

| Path | Description |
|---|---|
| `birds.csv`, `birds.json` | Bird definitions and the generated data |
| `birds.py` | Validates the CSV and generates the JSON |
| `src/term.rs`, `src/reduce.rs` | Term representation and weak reduction |
| `src/invariant.rs` | Incompleteness invariants |
| `src/closure.rs` | Closure search |
| `src/search.rs` | Set enumeration, reuse of results and resuming |
| `src/main.rs` | Command line |
| `verify.py` | Independent witness check |
| `report.py` | Classification and `summary.md` generation |
| `examples/probe.rs` | Helper that inspects the closure of one set |
| `tests/` | Rust and Python tests |
