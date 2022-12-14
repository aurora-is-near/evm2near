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
//
// use crate::graph::Graph;
// use crate::graph::Block;
// mod graph;
//
//
//
// fn main() {
//     let mut g = Graph::new();
//     for _ in 0..6 {
//         g.add_vertex(Block::new());
//     }
//     g.add_edge(0, 1);
//     g.add_edge(0, 5);
//     g.add_edge(1, 2);
//     g.add_edge(1, 3);
//     g.add_edge(5, 3);
//     g.add_edge(2, 4);
//     g.add_edge(3, 4);
//     g.add_edge(4, 1);
//     let res = g.reverse_postorder(0);
//     println!("Reverse postorder:");
//     for n in res {
//         print!("{}, ", n);
//     }
//     println!("\nDomination tree: ");
//     let dt = g.domination_tree(0);
//     for p in dt {
//         println!("k : {}, v : {}", p.0 , p.1);
//     }
//     println!("Labels:");
//     g.put_labels(0);
//     g.print_labels();
// }
