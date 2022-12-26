use crate::graph::cfg::{Cfg, CfgLabel};
use crate::traversal::graph::dfs::dfs_post;
use std::collections::{HashMap, HashSet};

#[derive(Default)]
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
