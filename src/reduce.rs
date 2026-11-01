//! 最左最外戦略の弱簡約器。ステップ数と木のサイズの上限つき。
use crate::term::{Arena, Birds, Id, Node};

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub max_steps: u32,
    pub max_nodes: u32,
}

pub const DEFAULT_LIMITS: Limits = Limits {
    max_steps: 200,
    max_nodes: 10_000,
};

impl Arena {
    /// 正規形を返す。上限を超えたら `None`。
    pub fn nf(&mut self, t: Id, birds: &Birds, lim: Limits) -> Option<Id> {
        let mut steps = 0;
        self.nf_inner(t, birds, lim, &mut steps)
    }

    fn nf_inner(&mut self, t: Id, birds: &Birds, lim: Limits, steps: &mut u32) -> Option<Id> {
        let mut visited = Vec::new();
        let result = self.nf_loop(t, birds, lim, steps, &mut visited);
        match result {
            Some(r) => {
                for v in visited {
                    self.memo.insert(v, r);
                }
            }
            // 上限を超えた項は覚えておき、同じ計算を繰り返さない。
            None => self.failed.extend(visited),
        }
        result
    }

    fn nf_loop(
        &mut self,
        t: Id,
        birds: &Birds,
        lim: Limits,
        steps: &mut u32,
        visited: &mut Vec<Id>,
    ) -> Option<Id> {
        let mut cur = t;
        loop {
            if let Some(&r) = self.memo.get(&cur) {
                return Some(r);
            }
            if self.failed.contains(&cur) {
                return None;
            }
            visited.push(cur);
            if self.tree_size(cur) > lim.max_nodes {
                return None;
            }
            // args は最後の引数が先頭に来る。
            let mut args = Vec::new();
            let mut head = cur;
            while let Node::App(l, r) = self.nodes[head as usize] {
                args.push(r);
                head = l;
            }
            if let Node::Bird(b) = self.nodes[head as usize] {
                let bird = &birds.list[b as usize];
                if args.len() >= bird.arity {
                    *steps += 1;
                    if *steps > lim.max_steps {
                        return None;
                    }
                    args.reverse();
                    let mut next = self.instantiate(&bird.rhs, &args[..bird.arity]);
                    for &a in &args[bird.arity..] {
                        next = self.app(next, a);
                    }
                    cur = next;
                    continue;
                }
            }
            let mut res = head;
            for &a in args.iter().rev() {
                let n = self.nf_inner(a, birds, lim, steps)?;
                res = self.app(res, n);
            }
            self.memo.insert(res, res);
            return Some(res);
        }
    }
}
