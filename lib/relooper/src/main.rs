mod cfg;
mod re_graph;
mod relooper;
mod traversal;

use crate::cfg::{Cfg, CfgLabel};
use crate::relooper::reloop;
use crate::traversal::graph::dfs::Dfs;

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

    // let mut f_cfg = File::create("cfg.dot").unwrap();

    let dfs: Vec<_> = Dfs::start_from(0 as CfgLabel, |&n| graph.children(n).into_iter()).collect();
    println!("dfs: {:?}", dfs);

    reloop(&graph, 0);
}

// extern crate queues;
// use crate::graph::Block;
// use crate::graph::Graph;
// use crate::graph::ReadGraph;
// use std::fs;
// mod graph;
// use std::borrow::Borrow;
//
// pub fn TestGraph(mut g: Graph, name: &str) {
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
// }
//
// fn main() {
//     let paths = fs::read_dir("test/").unwrap();
//     for entry in paths {
//         let path = entry.unwrap().path();
//         // let filename = ;
//         TestGraph(
//             ReadGraph(path.file_name().unwrap().to_string_lossy().borrow()),
//             path.file_name().unwrap().to_string_lossy().borrow(),
//         );
//     }
// }
