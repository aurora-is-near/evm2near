use crate::cfg::{Cfg, CfgEdge, CfgLabel};
use crate::relooper::NodeOrdering;
use crate::traversal::graph::bfs::Bfs;
use crate::traversal::graph::dfs::Dfs;
use std::collections::{HashMap, HashSet, VecDeque};
use std::vec::Vec;

pub struct EnrichedCfg {
    pub(crate) cfg: Cfg,
    entry: CfgLabel,
    back_edges: HashMap<CfgLabel, Vec<CfgLabel>>,
    node_ordering: NodeOrdering,
    pub(crate) merge_nodes: HashSet<CfgLabel>,
    pub(crate) loop_nodes: HashSet<CfgLabel>,
    pub(crate) if_nodes: HashSet<CfgLabel>,
}

impl EnrichedCfg {
    fn new(cfg: Cfg, entry: CfgLabel) -> Self {
        let mut back_edges: HashMap<CfgLabel, Vec<CfgLabel>> = HashMap::default();

        for (&from, &to_edge) in &cfg.out_edges {
            for to in to_edge.to_vec() {
                back_edges.entry(to).or_default().push(from);
            }
        }

        let node_ordering = NodeOrdering::new(&cfg, entry);

        let mut merge_nodes: HashSet<CfgLabel> = HashSet::new();
        let mut loop_nodes: HashSet<CfgLabel> = HashSet::new();
        let mut if_nodes: HashSet<CfgLabel> = HashSet::new();

        for n in cfg.nodes() {
            let back_edges_count = back_edges.get(&n).map_or(0, |v| v.len());
            if back_edges_count > 1 {
                merge_nodes.insert(n);
            }

            let reachable: HashSet<_> =
                Bfs::start_from_except(n, |&l| cfg.children(l).into_iter()).collect();
            for c in cfg.children(n).into_iter() {
                if node_ordering.is_backward(n, c) && reachable.contains(&c) {
                    loop_nodes.insert(n);
                }
            }

            if let CfgEdge::Cond(_, _) = cfg.out_edges.get(&n).unwrap() {
                if_nodes.insert(n);
            }
        }

        Self {
            cfg,
            entry,
            back_edges,
            node_ordering,
            merge_nodes,
            loop_nodes,
            if_nodes,
        }
    }

    pub fn domination_tree(&self, begin: CfgLabel) -> HashMap<CfgLabel, CfgLabel> /* map points from node id to id of its dominator */
    {
        let mut result = HashMap::<CfgLabel, CfgLabel>::new();
        let mut bfs = VecDeque::<CfgLabel>::new();
        let mut visited = HashSet::<CfgLabel>::new();
        for &n in self.node_ordering.sequence() {
            result.insert(n, begin);
        }
        bfs.push_back(begin); // should be next. upd: i dont think so
        visited.insert(begin);
        loop {
            if bfs.len() == 0 {
                break;
            }
            let &cur_id = bfs.front().unwrap();
            visited.insert(cur_id);
            bfs.pop_front().unwrap();
            self.update_dominators(cur_id, begin, &mut result);
            for id in &self.cfg.children(cur_id) {
                if !visited.contains(id) {
                    bfs.push_back(*id);
                }
            }
        }
        return result;
    }

    fn update_dominators(
        &self,
        cur_id: CfgLabel,
        origin: CfgLabel,
        result: &mut HashMap<CfgLabel, CfgLabel>,
    ) -> () {
        let mut reachable_set = HashSet::<CfgLabel>::default();
        for &node in self.node_ordering.sequence() {
            reachable_set.insert(node);
        }

        let reached = Dfs::start_from(origin, |&n| {
            let mut ch = self.cfg.children(n);
            ch.remove(&cur_id);
            ch
        });

        // let mut reached = Vec::<CfgLabel>::default();
        // let mut visited = HashSet::<CfgLabel>::default();
        // visited.insert(cur_id);
        // self.dfs(origin, &mut reached, &mut visited);

        for id in reached {
            reachable_set.remove(&id);
        }
        reachable_set.remove(&cur_id);
        for id in reachable_set {
            result.insert(id, cur_id);
        }
    }

    // pub fn gen_dot(&self, graphname: &str) -> () {
    //     let mut res = format!("digraph {graphname} {{ \n");
    //     for i in self.cfg.nodes() {
    //         let s = format!("    N{i}[label=\"N{i}\"];\n");
    //         res.push_str(&s);
    //     }
    //     for (from, node) in &self.id2node {
    //         for to in &node.succ {
    //             let s = format!("    N{from} -> N{to}[label=\"\"];\n");
    //             res.push_str(&s);
    //         }
    //     }
    //     res.push_str("}\n");
    //     let mut file = File::create(format!("dots/{graphname}.dot")).unwrap();
    //     file.write_all(res.as_bytes()).unwrap();
    // }
}

// pub fn read_graph(filepath: &str) -> EnrichedCfg {
//     println!("{}:", filepath);
//     let mut fullpath = "test/".to_owned();
//     fullpath.push_str(filepath);
//     let data = fs::read_to_string(fullpath).unwrap();
//     let lines = data.split("\n").collect::<Vec<&str>>();
//     let mut result = EnrichedCfg::new();
//     let size = lines[0].parse::<CfgLabel>().unwrap();
//     for _ in 0..size {
//         result.add_vertex(Block::new());
//     }
//     for line in lines {
//         if line.contains(" ") {
//             let nums = line.split(" ").collect::<Vec<&str>>();
//             result.add_edge(
//                 nums[0].parse::<CfgLabel>().unwrap(),
//                 nums[1].parse::<CfgLabel>().unwrap(),
//             )
//         }
//     }
//     return result;
// }
