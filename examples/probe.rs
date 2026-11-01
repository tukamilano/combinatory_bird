use combinatory_bird::closure::{base_rules,closure, Budget, Mode};
use combinatory_bird::term::Birds;
fn main(){
 let b=Birds::load("birds.json");
 let a:Vec<String>=std::env::args().skip(1).collect();
 let cand:u64=a[0].parse().unwrap();
 let set:Vec<&str>=a[1..].iter().map(|s|s.as_str()).collect();
 let t=std::time::Instant::now();
 let o=closure(&b,Mode::Weak,b.mask(&set),(1u64<<47)-1,Budget::new(cand),&base_rules(&b));
 println!("{:?} trunc={} cand={} {:.1}s",b.symbols(o.closure),o.truncated,o.candidates,t.elapsed().as_secs_f64());
 for d in &o.chain{println!("{} = {}",b.list[d.bird].symbol,d.witness);}
}
