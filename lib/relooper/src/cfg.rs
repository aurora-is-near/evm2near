use std::collections::{BTreeMap, HashSet};
use std::iter::once;

pub type CfgLabel = usize;
pub struct Cfg {
    out_edges: BTreeMap<CfgLabel, Vec<CfgLabel>>,
    in_edges: BTreeMap<CfgLabel, Vec<CfgLabel>>,
}

impl From<Vec<(CfgLabel, CfgLabel)>> for Cfg {
    fn from(edges: Vec<(CfgLabel, CfgLabel)>) -> Self {
        let mut out_edges: BTreeMap<CfgLabel, Vec<CfgLabel>> = BTreeMap::new();
        let mut in_edges: BTreeMap<CfgLabel, Vec<CfgLabel>> = BTreeMap::new();
        for (from, to) in edges {
            out_edges.entry(from).or_default().push(to);
            in_edges.entry(to).or_default().push(from);
        }

        Cfg {
            out_edges,
            in_edges,
        }
    }
}

impl Cfg {
    pub fn nodes(&self) -> HashSet<CfgLabel> {
        let m = &self
            .out_edges
            .iter()
            .flat_map(|(&from, to)| to.iter().copied().chain(once(from)))
            .collect::<HashSet<_>>();
        m.to_owned()
    }

    pub fn edges(&self) -> Vec<(CfgLabel, CfgLabel)> {
        self.out_edges
            .iter()
            .flat_map(|(&from, to)| to.iter().map(move |&t| (from, t)))
            .collect::<Vec<_>>()
    }

    pub fn children(&self, label: CfgLabel) -> HashSet<CfgLabel> {
        self.out_edges
            .get(&label)
            .into_iter()
            .flatten()
            .copied()
            .collect()
    }

    pub fn parents(&self, label: CfgLabel) -> HashSet<CfgLabel> {
        self.in_edges
            .get(&label)
            .into_iter()
            .flatten()
            .copied()
            .collect()
    }
}
