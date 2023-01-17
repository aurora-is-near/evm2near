extern crate core;

mod graph;
mod traversal;

use crate::graph::cfg::{Cfg, CfgLabel};
use crate::graph::{supergraph, EnrichedCfg};
use std::env;
use std::fmt::{Debug, Display, Formatter};
use std::path::Path;

impl CfgLabel for &String {}

pub fn main() {
    let args: Vec<String> = env::args().collect();

    assert_eq!(args.len(), 2);

    let input_path = Path::new(args.get(1).unwrap());
    let output_path = input_path.with_extension("dot");

    let input = std::fs::read_to_string(input_path).expect("unable to read input file");
    let lines: Vec<String> = input.split("\n").map(|x| x.to_string()).collect();

    let cfg_descr: Cfg<String> = Cfg::try_from(&lines).expect("invalud input formatting");
    let cfg = cfg_descr.to_borrowed();
    // let cfg: Cfg<usize> = Cfg::try_from(&lines).expect("invalud input formatting");

    let reduced_graph = supergraph::reduce(&cfg);

    let e_graph = EnrichedCfg::new(reduced_graph);
    let relooped = e_graph.reloop();

    let dot_lines: Vec<String> = vec![
        "digraph {".to_string(),
        cfg.cfg_to_dot("cfg"),
        String::new(),
        e_graph.cfg_to_dot("reduced"),
        String::new(),
        e_graph.dom_to_dot(),
        String::new(),
        relooped.to_dot(),
        "}".to_string(),
    ];

    std::fs::write(output_path, dot_lines.join("\n")).expect("fs error");
}
