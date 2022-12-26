use crate::cfg::{Cfg, CfgEdge, CfgLabel};
use crate::relooper::{DomTree, NodeOrdering};
use crate::traversal::graph::bfs::Bfs;
use crate::traversal::graph::dfs::Dfs;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::format;
use std::vec::Vec;

pub struct EnrichedCfg {
    pub(crate) cfg: Cfg,
    entry: CfgLabel,
    back_edges: HashMap<CfgLabel, Vec<CfgLabel>>,
    node_ordering: NodeOrdering,
    domination: DomTree,
    merge_nodes: HashSet<CfgLabel>,
    loop_nodes: HashSet<CfgLabel>,
    if_nodes: HashSet<CfgLabel>,
}

impl EnrichedCfg {
    pub fn new(cfg: Cfg, entry: CfgLabel) -> Self {
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
                    loop_nodes.insert(c);
                }
            }

            if let CfgEdge::Cond(_, _) = cfg.out_edges.get(&n).unwrap() {
                if_nodes.insert(n);
            }
        }

        let domination_map = Self::domination_tree(&cfg, &node_ordering, entry);
        let domination_vec = Vec::from_iter(domination_map);
        let domination = DomTree::from(domination_vec);

        Self {
            cfg,
            entry,
            back_edges,
            node_ordering,
            domination,
            merge_nodes,
            loop_nodes,
            if_nodes,
        }
    }

    pub fn domination_tree(
        cfg: &Cfg,
        node_ordering: &NodeOrdering,
        begin: CfgLabel,
    ) -> HashMap<CfgLabel, CfgLabel> /* map points from node id to id of its dominator */ {
        let mut result = HashMap::<CfgLabel, CfgLabel>::new();
        let mut bfs = VecDeque::<CfgLabel>::new();
        let mut visited = HashSet::<CfgLabel>::new();
        for &n in node_ordering.sequence() {
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
            Self::update_dominators(cfg, node_ordering, cur_id, begin, &mut result);
            for id in cfg.children(cur_id) {
                if !visited.contains(&id) {
                    bfs.push_back(id);
                }
            }
        }
        return result;
    }

    fn update_dominators(
        cfg: &Cfg,
        node_ordering: &NodeOrdering,
        cur_id: CfgLabel,
        origin: CfgLabel,
        result: &mut HashMap<CfgLabel, CfgLabel>,
    ) {
        let mut reachable_set = HashSet::<CfgLabel>::default();
        for &node in node_ordering.sequence() {
            reachable_set.insert(node);
        }

        let reached = Dfs::start_from(origin, |&n| {
            let mut ch = cfg.children(n);
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

    fn labels(&self, n: CfgLabel) -> String {
        let mut res = "".to_string();
        if self.loop_nodes.contains(&n) {
            res += "l";
        }
        if self.if_nodes.contains(&n) {
            res += "i";
        }
        if self.merge_nodes.contains(&n) {
            res += "m";
        }

        res
    }

    pub fn to_dot(&self) -> String {
        let mut lines: Vec<String> = Vec::new();
        lines.push("digraph res {".to_string());
        lines.push("subgraph cluster_cfg { label=\"cfg\";".to_string());
        lines.push("nstart[label=\"start\"]".to_string());
        lines.push("nend[label=\"end\"]".to_string());

        let mut edges: Vec<String> = Vec::new();
        for n in self.cfg.nodes() {
            lines.push(format!("n{n}[label=\"{n} {}\"];", self.labels(n)));
            match self.cfg.edge(n) {
                CfgEdge::Uncond(u) => {
                    edges.push(format!("n{n} -> n{u};"));
                }
                CfgEdge::Cond(t, f) => {
                    edges.push(format!("n{n} -> n{t}[style=\"dashed\"];"));
                    edges.push(format!("n{n} -> n{f};"));
                }
                CfgEdge::Terminal => {
                    edges.push(format!("n{n} -> nend;"));
                }
            }
        }
        lines.push(format!("nstart -> n{}", self.entry));
        lines.extend(edges);
        lines.push("}".to_string());
        lines.push(String::new());

        lines.push("subgraph cluster_dom { label=\"dom\"; edge [dir=\"back\"];".to_string());
        for n in self.cfg.nodes() {
            lines.push(format!("d{n}[label=\"{n}\"];"));
        }
        for (&n, &d) in &self.domination.dominated {
            lines.push(format!("d{d} -> d{n};"));
        }
        lines.push("}".to_string());
        lines.push(String::new());

        lines.push("}".to_string());
        lines.join("\n")
    }
}
