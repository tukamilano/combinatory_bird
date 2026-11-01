//! Invariants that certify incompleteness.
use crate::term::{Bird, Birds};

fn counts(b: &Bird) -> Vec<u32> {
    let mut c = vec![0; b.arity];
    b.rhs.count_vars(&mut c);
    c
}

pub fn is_erasing(b: &Bird) -> bool {
    counts(b).iter().any(|&n| n == 0)
}

pub fn is_duplicating(b: &Bird) -> bool {
    counts(b).iter().any(|&n| n > 1)
}

/// Return the reason if the set is provably incomplete.
pub fn certificate(birds: &Birds, mask: u64) -> Option<&'static str> {
    let members = || {
        birds
            .list
            .iter()
            .enumerate()
            .filter(|(i, _)| mask >> i & 1 == 1)
            .map(|(_, b)| b)
    };
    if !members().any(is_erasing) {
        return Some("non_erasing");
    }
    if !members().any(is_duplicating) {
        return Some("non_duplicating");
    }
    None
}
