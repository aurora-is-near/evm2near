use std::collections::{BTreeMap, HashSet};
use std::iter::once;

pub type CfgLabel = usize;
pub type CfgEdge = (CfgLabel, bool);
pub struct Cfg {
    out_edges: BTreeMap<CfgLabel, Vec<CfgEdge>>,
}

impl From<Vec<(CfgLabel, CfgLabel, bool)>> for Cfg {
    fn from(edges: Vec<(CfgLabel, CfgLabel, bool)>) -> Self {
        let mut out_edges: BTreeMap<CfgLabel, Vec<CfgEdge>> = BTreeMap::new();
        for (from, to, is_conditional) in edges {
            out_edges
                .entry(from)
                .or_default()
                .push((to, is_conditional));
        }

        Cfg { out_edges }
    }
}

impl Cfg {
    pub fn edges_raw(&self) -> HashSet<(CfgLabel, CfgEdge)> {
        self.out_edges
            .iter()
            .flat_map(|(&from, to)| to.iter().map(move |&t| (from, t)))
            .collect()
    }

    pub fn edges(&self) -> HashSet<(CfgLabel, CfgLabel, bool)> {
        self.edges_raw()
            .iter()
            .map(|&(from, (to, is_cond))| (from, to, is_cond))
            .collect()
    }

    pub fn nodes(&self) -> HashSet<CfgLabel> {
        self.edges()
            .iter()
            .flat_map(|&(f, t, is_cond)| vec![f, t])
            .collect()
    }

    pub fn children(&self, label: CfgLabel) -> HashSet<CfgLabel> {
        self.out_edges
            .get(&label)
            .into_iter()
            .flatten()
            .map(|&(to, is_cond)| to)
            .collect()
    }
}
