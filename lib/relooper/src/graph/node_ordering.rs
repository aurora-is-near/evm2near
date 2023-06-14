use std::collections::{HashMap, HashSet};

use crate::traversal::graph::dfs::{DfsPost, DfsPostReverseInstantiator};

use super::{
    cfg::{Cfg, CfgLabel},
    Graph,
};

pub struct NodeOrdering<TLabel: CfgLabel> {
    pub(crate) idx: HashMap<TLabel, usize>,
    vec: Vec<TLabel>,
}

impl<TLabel: CfgLabel> NodeOrdering<TLabel> {
    pub fn new(cfg: &Cfg<TLabel>, entry: TLabel) -> Self {
        let vec: Vec<TLabel> = DfsPost::<_, _, HashSet<_>>::reverse(&entry, |x| cfg.children(x))
            .into_iter()
            .copied()
            .collect();
        let idx: HashMap<TLabel, usize> = vec.iter().enumerate().map(|(i, &n)| (n, i)).collect();
        Self { vec, idx }
    }

    pub fn is_backward(&self, from: &TLabel, to: &TLabel) -> bool {
        self.idx
            .get(from)
            .zip(self.idx.get(to))
            .map(|(&f, &t)| f > t)
            .unwrap()
    }

    pub fn is_forward(&self, from: &TLabel, to: &TLabel) -> bool {
        !self.is_backward(from, to)
    }

    pub fn sequence(&self) -> &Vec<TLabel> {
        &self.vec
    }
}
