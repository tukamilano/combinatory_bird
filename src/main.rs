use combinatory_bird::closure::{Budget, Mode};
use combinatory_bird::search::{load_records, run, Config};
use combinatory_bird::term::Birds;
use std::path::PathBuf;

fn main() {
    let mut max_subset = 4;
    let mut mode = "both".to_string();
    let mut budget = 100_000u64;
    let mut out_dir = PathBuf::from("results");
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut val = || args.next().expect("値がありません");
        match a.as_str() {
            "--max-subset" => max_subset = val().parse().unwrap(),
            "--mode" => mode = val(),
            "--budget" => budget = val().parse().unwrap(),
            "--out" => out_dir = PathBuf::from(val()),
            _ => panic!("不明な引数: {a}"),
        }
    }
    rayon::ThreadPoolBuilder::new()
        .stack_size(64 * 1024 * 1024)
        .build_global()
        .unwrap();
    std::fs::create_dir_all(&out_dir).unwrap();
    let birds = Birds::load("birds.json");
    let budget = Budget::new(budget);
    let weak_path = out_dir.join("weak.jsonl");
    if mode == "weak" || mode == "both" {
        run(&Config {
            birds: &birds,
            mode: Mode::Weak,
            max_subset,
            budget,
            out: &weak_path,
            inherit: Vec::new(),
        });
    }
    if mode == "ext" || mode == "both" {
        run(&Config {
            birds: &birds,
            mode: Mode::Ext,
            max_subset,
            budget,
            out: &out_dir.join("ext.jsonl"),
            inherit: load_records(&weak_path, &birds),
        });
    }
}
