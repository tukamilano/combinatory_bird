use combinatory_bird::closure::{base_rules, closure, Budget, Mode};
use combinatory_bird::invariant::certificate;
use combinatory_bird::reduce::DEFAULT_LIMITS;
use combinatory_bird::search::universe;
use combinatory_bird::term::{Arena, Birds};

fn birds() -> Birds {
    Birds::load("birds.json")
}

fn run(b: &Birds, set: &[&str], mode: Mode) -> u64 {
    closure(b, mode, b.mask(set), universe(b, mode), Budget::new(2_000_000), &base_rules(b)).closure
}

#[test]
fn reducer_basic() {
    let b = birds();
    let mut a = Arena::new();
    let t = a.parse("S K K I", &b);
    let i = a.parse("I", &b);
    assert_eq!(a.nf(t, &b, DEFAULT_LIMITS), Some(i));
    let omega = a.parse("M M", &b);
    assert_eq!(a.nf(omega, &b, DEFAULT_LIMITS), None);
    // 消去される発散項があっても正規形に到達する。
    let t = a.parse("K I (M M)", &b);
    assert_eq!(a.nf(t, &b, DEFAULT_LIMITS), Some(i));
}

#[test]
fn known_complete_sets() {
    let b = birds();
    let goal = b.mask(&["S", "K"]);
    for set in [
        vec!["S", "K"],
        vec!["B", "C", "W", "K"],
        vec!["B", "T", "M", "K"],
        vec!["B", "W", "T", "K"],
    ] {
        for mode in [Mode::Weak, Mode::Ext] {
            assert_eq!(run(&b, &set, mode) & goal, goal, "{set:?} {mode:?}");
        }
    }
}

#[test]
fn known_incomplete_sets_have_certificates() {
    let b = birds();
    for (set, cert) in [
        (vec!["B", "C", "W", "I"], "non_erasing"),
        (vec!["I", "J"], "non_erasing"),
        (vec!["B", "C", "K", "I"], "non_duplicating"),
        (vec!["G", "K"], "non_duplicating"),
        (vec!["G", "I"], "non_erasing"),
        (vec!["B", "T", "K"], "non_duplicating"),
    ] {
        assert_eq!(certificate(&b, b.mask(&set)), Some(cert), "{set:?}");
    }
}

#[test]
fn invariants_hold_on_closures() {
    let b = birds();
    let (s, k, w) = (b.mask(&["S"]), b.mask(&["K"]), b.mask(&["W"]));
    for mode in [Mode::Weak, Mode::Ext] {
        for set in [vec!["B", "C", "W", "I"], vec!["I", "J"]] {
            assert_eq!(run(&b, &set, mode) & k, 0, "{set:?}");
        }
        for set in [vec!["B", "C", "K", "I"], vec!["G", "K"], vec!["B", "T", "K"]] {
            assert_eq!(run(&b, &set, mode) & (s | w), 0, "{set:?}");
        }
    }
}

#[test]
fn weak_and_ext_differ_on_identity() {
    let b = birds();
    let i = b.mask(&["I"]);
    // 外延版の照合対象に I を含め、I* を原子に持たせる。
    let all = (1u64 << b.list.len()) - 1;
    let budget = Budget::new(200_000);
    let seed = b.mask(&["I*"]);
    assert_eq!(closure(&b, Mode::Weak, seed, all, budget, &[]).closure & i, 0);
    assert_eq!(closure(&b, Mode::Ext, seed, all, budget, &[]).closure & i, i);
}

#[test]
fn search_finds_starling_without_rules() {
    let b = birds();
    let set = b.mask(&["B", "C", "W"]);
    let s = b.mask(&["S"]);
    for mode in [Mode::Weak, Mode::Ext] {
        let uni = (1u64 << b.list.len()) - 1;
        let out = closure(&b, mode, set, uni, Budget::new(2_000_000), &[]);
        assert_eq!(out.closure & s, s, "{mode:?}");
    }
}
