//! 不完全性の証明書となる不変量。
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

/// 組が不完全だと証明できれば、その理由を返す。
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
