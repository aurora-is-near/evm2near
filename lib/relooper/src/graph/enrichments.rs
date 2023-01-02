use crate::graph::cfg::{Cfg, CfgLabel};
use crate::traversal::graph::dfs::dfs_post;
use std::collections::{HashMap, HashSet};

#[derive(Default)]
/// Node A dominate node B if you can't reach B without visiting A. For example, entry node dominates all nodes.
/// Each node have set of dominators. If B_set is set of node B dominators, node A will called Immediate Dominator of B
/// if it is in B_set AND NOT dominate any other nodes from B_set.
/// Each node have exactly one immediate dominator. Each node can be immediate dominator for any amount of nodes.
///  
/// Domination tree is a graph with nodes of CFG, but edges only from dominator to dominated nodes.
/// Domination tree uniquely specified by given CFG
///
/// We build domination tree next way:
/// 1) make an array of results (hash_map (dominated -> dominator)) and initialize it with entry node as dominator for every node.
/// 2) Than we iterate in nodes in reverse postorder(?) and make next operation for each node:
///   2.1) remove this node and all its edges from graph, go throw graph with dfs, and find all nodes unreachable without this nodes
///   2.2) update immediate dominator for all unreachable nodes
///
/// Thanks to reverse postorder we will find immediate dominator for all nodes.
///
pub struct DomTree {
    dominates: HashMap<CfgLabel, HashSet<CfgLabel>>,
    pub(crate) dominated: HashMap<CfgLabel, CfgLabel>,
}

impl From<Vec<(CfgLabel, CfgLabel)>> for DomTree {
    fn from(edges: Vec<(CfgLabel, CfgLabel)>) -> Self {
        let dominated = HashMap::from_iter(edges.iter().copied());
        let mut dominates: HashMap<CfgLabel, HashSet<CfgLabel>> = HashMap::new();

        for (dominated, dominator) in edges {
            dominates.entry(dominator).or_default().insert(dominated);
        }

        DomTree {
            dominates,
            dominated,
        }
    }
}

impl DomTree {
    pub(crate) fn immediately_dominated_by(&self, label: CfgLabel) -> HashSet<CfgLabel> {
        self.dominates
            .get(&label)
            .unwrap_or(&HashSet::new())
            .to_owned()
    }
}

pub struct NodeOrdering {
    pub(crate) idx: HashMap<CfgLabel, usize>,
    vec: Vec<CfgLabel>,
}

impl NodeOrdering {
    pub fn new(cfg: &Cfg, entry: CfgLabel) -> Self {
        let vec = dfs_post(entry, &mut |x| cfg.children(*x));
        let idx: HashMap<CfgLabel, usize> = vec.iter().enumerate().map(|(i, &n)| (n, i)).collect();
        Self { vec, idx }
    }

    pub fn is_backward(&self, from: CfgLabel, to: CfgLabel) -> bool {
        self.idx
            .get(&from)
            .zip(self.idx.get(&to))
            .map(|(&f, &t)| f > t)
            .unwrap()
    }

    pub fn is_forward(&self, from: CfgLabel, to: CfgLabel) -> bool {
        !self.is_backward(from, to)
    }

    pub fn sequence(&self) -> &Vec<CfgLabel> {
        &self.vec
    }
}
