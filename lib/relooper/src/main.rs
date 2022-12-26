mod graph;
mod traversal;

use crate::graph::cfg::Cfg;
use crate::graph::EnrichedCfg;

pub fn main() {
    let graph = Cfg::from(vec![
        (0, 1, true),
        (0, 2, false),
        (1, 3, true),
        (2, 3, false),
        (3, 4, false),
        (1, 5, false),
        (5, 6, true),
        (5, 7, false),
        (6, 8, false),
        (7, 8, false),
        (4, 9, false),
        (8, 9, true),
        (8, 5, false),
    ]);
    // let graph = Cfg::from(vec![(0, 1), (0, 2), (1, 3), (1, 4), (1, 5), (2, 6), (6, 7)]);

    let e_graph = EnrichedCfg::new(graph, 0);
    std::fs::write("out.dot", e_graph.to_dot()).expect("fs error");

    // reloop(&graph, 0);
}

// extern crate queues;
// use crate::graph::read_graph;
// use crate::graph::Graph;
// use std::fs;
// mod graph;
// use std::borrow::Borrow;
//
// pub fn test_graph(mut g: Graph, name: &str) {
//     let res = g.reverse_postorder(0);
//     println!("Reverse postorder:");
//     for n in res {
//         print!("{}, ", n);
//     }
//     println!("\nDomination tree: ");
//     let dt = g.domination_tree(0);
//     for p in dt {
//         println!("k : {}, v : {}", p.0, p.1);
//     }
//     println!("Labels:");
//     g.put_labels(0);
//     g.print_labels();
//     g.gen_dot(name.strip_suffix(".txt").unwrap());
//     let mut reducable_name = "reducable_".to_owned();
//     reducable_name.push_str(name);
//     g.reducable()
//         .gen_dot(reducable_name.strip_suffix(".txt").unwrap())
// }
//
// fn main() {
//     let paths = fs::read_dir("test/").unwrap();
//     for entry in paths {
//         let path = entry.unwrap().path();
//         // let filename = ;
//         test_graph(
//             read_graph(path.file_name().unwrap().to_string_lossy().borrow()),
//             path.file_name().unwrap().to_string_lossy().borrow(),
//         );
//     }
// }
