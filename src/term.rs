//! Hash-consed terms and bird definitions.
use rustc_hash::FxHashMap;
use serde_json::Value;

pub type Id = u32;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Node {
    Bird(u8),
    Var(u8),
    App(Id, Id),
}

#[derive(Clone, Debug)]
pub enum Rhs {
    Var(usize),
    App(Box<Rhs>, Box<Rhs>),
}

impl Rhs {
    fn from_json(v: &Value) -> Rhs {
        match v {
            Value::Number(n) => Rhs::Var(n.as_u64().expect("variable index") as usize),
            Value::Array(a) => Rhs::App(
                Box::new(Rhs::from_json(&a[0])),
                Box::new(Rhs::from_json(&a[1])),
            ),
            _ => panic!("invalid rhs: {v}"),
        }
    }

    /// Count the occurrences of each variable.
    pub fn count_vars(&self, counts: &mut [u32]) {
        match self {
            Rhs::Var(i) => counts[*i] += 1,
            Rhs::App(l, r) => {
                l.count_vars(counts);
                r.count_vars(counts);
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Bird {
    pub symbol: String,
    pub arity: usize,
    pub rhs: Rhs,
}

#[derive(Clone, Debug)]
pub struct Birds {
    pub list: Vec<Bird>,
}

impl Birds {
    pub fn load(path: &str) -> Birds {
        let text = std::fs::read_to_string(path).expect("cannot read birds.json");
        let v: Value = serde_json::from_str(&text).expect("malformed birds.json");
        let list = v
            .as_array()
            .expect("array")
            .iter()
            .map(|b| Bird {
                symbol: b["symbol"].as_str().unwrap().to_string(),
                arity: b["arity"].as_u64().unwrap() as usize,
                rhs: Rhs::from_json(&b["rhs"]),
            })
            .collect();
        Birds { list }
    }

    pub fn index(&self, symbol: &str) -> usize {
        self.list
            .iter()
            .position(|b| b.symbol == symbol)
            .unwrap_or_else(|| panic!("unknown bird: {symbol}"))
    }

    pub fn mask(&self, symbols: &[&str]) -> u64 {
        symbols.iter().fold(0, |m, s| m | 1u64 << self.index(s))
    }

    pub fn symbols(&self, mask: u64) -> Vec<String> {
        (0..self.list.len())
            .filter(|i| mask >> i & 1 == 1)
            .map(|i| self.list[i].symbol.clone())
            .collect()
    }
}

pub struct Arena {
    pub nodes: Vec<Node>,
    sizes: Vec<u32>,
    map: FxHashMap<Node, Id>,
    pub(crate) memo: FxHashMap<Id, Id>,
    pub(crate) failed: rustc_hash::FxHashSet<Id>,
}

impl Default for Arena {
    fn default() -> Self {
        Self::new()
    }
}

impl Arena {
    pub fn new() -> Arena {
        Arena {
            nodes: Vec::new(),
            sizes: Vec::new(),
            map: FxHashMap::default(),
            memo: FxHashMap::default(),
            failed: Default::default(),
        }
    }

    fn intern(&mut self, n: Node, size: u32) -> Id {
        if let Some(&id) = self.map.get(&n) {
            return id;
        }
        let id = self.nodes.len() as Id;
        self.nodes.push(n);
        self.sizes.push(size);
        self.map.insert(n, id);
        id
    }

    pub fn bird(&mut self, b: usize) -> Id {
        self.intern(Node::Bird(b as u8), 1)
    }

    pub fn var(&mut self, i: usize) -> Id {
        self.intern(Node::Var(i as u8), 1)
    }

    pub fn app(&mut self, l: Id, r: Id) -> Id {
        let size = self.sizes[l as usize]
            .saturating_add(self.sizes[r as usize])
            .saturating_add(1);
        self.intern(Node::App(l, r), size)
    }

    /// Node count as a tree (saturating).
    pub fn tree_size(&self, t: Id) -> u32 {
        self.sizes[t as usize]
    }

    pub fn instantiate(&mut self, rhs: &Rhs, args: &[Id]) -> Id {
        match rhs {
            Rhs::Var(i) => args[*i],
            Rhs::App(l, r) => {
                let a = self.instantiate(l, args);
                let b = self.instantiate(r, args);
                self.app(a, b)
            }
        }
    }

    pub fn show(&self, t: Id, birds: &Birds) -> String {
        let mut out = String::new();
        self.write(t, birds, false, &mut out);
        out
    }

    fn write(&self, t: Id, birds: &Birds, paren: bool, out: &mut String) {
        match self.nodes[t as usize] {
            Node::Bird(b) => out.push_str(&birds.list[b as usize].symbol),
            Node::Var(i) => out.push_str(&format!("x{i}")),
            Node::App(l, r) => {
                if paren {
                    out.push('(');
                }
                self.write(l, birds, false, out);
                out.push(' ');
                self.write(r, birds, true, out);
                if paren {
                    out.push(')');
                }
            }
        }
    }

    /// Parse an expression in `show` format (used to read known derivations).
    pub fn parse(&mut self, text: &str, birds: &Birds) -> Id {
        let spaced = text.replace('(', " ( ").replace(')', " ) ");
        let tokens: Vec<&str> = spaced.split_whitespace().collect();
        let mut pos = 0;
        let t = self.parse_app(&tokens, &mut pos, birds);
        assert_eq!(pos, tokens.len(), "invalid end of expression: {text}");
        t
    }

    fn parse_app(&mut self, tokens: &[&str], pos: &mut usize, birds: &Birds) -> Id {
        let mut term = None;
        while *pos < tokens.len() && tokens[*pos] != ")" {
            let atom = if tokens[*pos] == "(" {
                *pos += 1;
                let inner = self.parse_app(tokens, pos, birds);
                assert_eq!(tokens[*pos], ")");
                *pos += 1;
                inner
            } else {
                let b = birds.index(tokens[*pos]);
                *pos += 1;
                self.bird(b)
            };
            term = Some(match term {
                None => atom,
                Some(l) => self.app(l, atom),
            });
        }
        term.expect("empty expression")
    }
}
