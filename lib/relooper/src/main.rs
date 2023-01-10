mod graph;
mod traversal;

use crate::graph::cfg::Cfg;
use crate::graph::reducability::test_irreducable;
use crate::graph::EnrichedCfg;
use std::env;
use std::path::Path;

// pub fn main() {
//     let args: Vec<String> = env::args().collect();

//     assert_eq!(args.len(), 2);

//     let input_path = Path::new(args.get(1).unwrap());
//     let output_path = input_path.with_extension("dot");

//     let input = std::fs::read_to_string(input_path).expect("unable to read input file");
//     let lines = input.split("\n").map(|x| x.to_string()).collect();

//     let graph = Cfg::from_strings(lines).expect("invalid input formatting");

//     let e_graph = EnrichedCfg::new(graph);
//     let relooped = e_graph.reloop();

//     let dot_lines: Vec<String> = vec![
//         "digraph {".to_string(),
//         e_graph.cfg_to_dot(),
//         String::new(),
//         e_graph.dom_to_dot(),
//         String::new(),
//         relooped.to_dot(),
//         "}".to_string(),
//     ];

//     std::fs::write(output_path, dot_lines.join("\n")).expect("fs error");
// }

pub fn main() {
    test_irreducable();
}
