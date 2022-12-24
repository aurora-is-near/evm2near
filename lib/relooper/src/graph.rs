use crate::cfg::{Cfg, CfgEdge, CfgLabel};
use crate::traversal::graph::dfs::dfs_post;
use queues::IsQueue;
use queues::Queue;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::vec::Vec;

#[derive(Default, Clone)]
pub struct Node {
    pub id: CfgLabel,
    pub succ: HashSet<CfgLabel>,
    pub prec: HashSet<CfgLabel>,
}

impl Node {
    pub fn new(id_: CfgLabel) -> Node {
        return Node {
            id: id_,
            succ: HashSet::default(),
            prec: HashSet::default(),
        };
    }
}

pub struct EnrichedCfg {
    cfg: Cfg,
    entry: CfgLabel,
    back_edges: HashMap<CfgLabel, Vec<CfgLabel>>,
    postorder_rev: HashMap<CfgLabel, usize>,
    pub(crate) id2node: HashMap<CfgLabel, Node>,
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

        let postorder_rev = dfs_post(entry, &mut |x| cfg.children(*x))
            .into_iter()
            .enumerate()
            .map(|(i, n)| (n, i))
            .collect::<HashMap<_, _>>();

        let mut merge_nodes: HashSet<CfgLabel> = HashSet::new();
        let mut loop_nodes: HashSet<CfgLabel> = HashSet::new();
        let mut if_nodes: HashSet<CfgLabel> = HashSet::new();

        for n in cfg.nodes() {
            match cfg.out_edges.get(&n).unwrap() {
                CfgEdge::Cond(_, _) => {
                    if_nodes.insert(n);
                }
                _ => {}
            }
        }

        Self {
            cfg,
            entry,
            back_edges,
            postorder_rev,
            id2node: Default::default(),
            merge_nodes: Default::default(),
            loop_nodes: Default::default(),
            if_nodes: Default::default(),
        }
    }
}

impl EnrichedCfg {
    // TODO cache result
    pub fn reverse_postorder(&self, begin: CfgLabel) -> Vec<CfgLabel> {
        let mut res = Vec::<CfgLabel>::default();
        let mut visited = HashSet::<CfgLabel>::default();
        self.dfs(begin, &mut res, &mut visited);
        res.reverse();
        return res;
    }

    fn dfs(
        &self,
        current_node: CfgLabel,
        res: &mut Vec<CfgLabel>,
        visited: &mut HashSet<CfgLabel>,
    ) -> () {
        for id in &self.cfg.children(current_node) {
            if !visited.contains(&id) {
                visited.insert(*id);
                self.dfs(*id, res, visited);
            }
        }
        res.push(current_node);
    }

    pub fn domination_tree(&self, begin: CfgLabel) -> HashMap<CfgLabel, CfgLabel> /* map points from node id to id of its dominator */
    {
        let mut result = HashMap::<CfgLabel, CfgLabel>::new();
        let mut bfs = Queue::<CfgLabel>::new();
        let mut visited = HashSet::<CfgLabel>::new();
        let nodes = self.reverse_postorder(begin);
        for n in nodes {
            result.insert(n, begin);
        }
        bfs.add(begin).unwrap(); // should be next. upd: i dont think so
        visited.insert(begin);
        loop {
            if bfs.size() == 0 {
                break;
            }
            let cur_id = bfs.peek().unwrap();
            visited.insert(cur_id);
            bfs.remove().unwrap();
            self.update_dominators(cur_id, begin, &mut result);
            for id in &self.cfg.children(cur_id) {
                if !visited.contains(id) {
                    bfs.add(*id).unwrap();
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
        let reachable = self.reverse_postorder(origin);
        let mut reachable_set = HashSet::<CfgLabel>::default();
        for node in reachable {
            reachable_set.insert(node);
        }
        let mut reached = Vec::<CfgLabel>::default();
        let mut visited = HashSet::<CfgLabel>::default();
        visited.insert(cur_id);
        self.dfs(origin, &mut reached, &mut visited);
        for id in reached {
            reachable_set.remove(&id);
        }
        reachable_set.remove(&cur_id);
        for id in reachable_set {
            result.insert(id, cur_id);
        }
    }

    fn is_backward(&self, from: CfgLabel, to: CfgLabel) -> bool {
        self.postorder_rev
            .get(&from)
            .and_then(|&f| self.postorder_rev.get(&to).map(|&t| f < t))
            .unwrap()
    }

    fn is_forward(&self, from: CfgLabel, to: CfgLabel) -> bool {
        !self.is_backward(from, to)
    }

    // // TODO: check if edge exist
    // fn is_forward(&self, begin: CfgLabel, from: CfgLabel, to: CfgLabel) -> bool {
    //     let order = self.reverse_postorder(begin);
    //     for id in order {
    //         if id == from {
    //             return true;
    //         }
    //         if id == to {
    //             return false;
    //         }
    //     }
    //     return false;
    // }
    //
    // fn is_backward(&self, begin: CfgLabel, from: CfgLabel, to: CfgLabel) -> bool {
    //     return !self.is_forward(begin, from, to);
    // }

    fn put_merge_labels(&mut self) {
        for (id, node) in &self.id2node {
            let mut forward_inedjes = 0;
            for origin in &node.prec {
                if self.is_forward(*origin, *id) {
                    forward_inedjes += 1;
                }
            }
            if forward_inedjes >= 2 {
                self.merge_nodes.insert(*id);
            }
        }
    }

    fn put_loop_labels(&mut self, begin: CfgLabel) -> () {
        for (id, node) in &self.id2node {
            for origin in &node.prec {
                if self.is_backward(*origin, *id) {
                    self.loop_nodes.insert(*id);
                    break;
                }
            }
        }
    }

    fn put_if_labels(&mut self, _begin: CfgLabel) -> () {
        for (id, node) in &self.id2node {
            if node.succ.len() > 1 {
                self.if_nodes.insert(*id);
            }
        }
    }

    pub fn gen_dot(&self, graphname: &str) -> () {
        let mut res = format!("digraph {graphname} {{ \n");
        for i in self.cfg.nodes() {
            let s = format!("    N{i}[label=\"N{i}\"];\n");
            res.push_str(&s);
        }
        for (from, node) in &self.id2node {
            for to in &node.succ {
                let s = format!("    N{from} -> N{to}[label=\"\"];\n");
                res.push_str(&s);
            }
        }
        res.push_str("}\n");
        let mut file = File::create(format!("dots/{graphname}.dot")).unwrap();
        file.write_all(res.as_bytes()).unwrap();
    }
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
