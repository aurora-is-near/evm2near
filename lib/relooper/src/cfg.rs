use std::collections::{BTreeMap, HashSet};
use std::iter::once;

pub type CfgLabel = isize;
pub struct Cfg(pub(crate) BTreeMap<CfgLabel, Vec<CfgLabel>>);

impl From<Vec<(CfgLabel, CfgLabel)>> for Cfg {
    fn from(edges: Vec<(CfgLabel, CfgLabel)>) -> Self {
        let mut m: BTreeMap<CfgLabel, Vec<CfgLabel>> = BTreeMap::new();
        for (from, to) in edges {
            m.entry(from).or_default().push(to);
        }
        Cfg(m)
    }
}

impl Cfg {
    pub fn nodes(&self) -> HashSet<CfgLabel> {
        let m = &self
            .0
            .iter()
            .flat_map(|(&from, to)| to.iter().copied().chain(once(from)))
            .collect::<HashSet<_>>();
        m.to_owned()
    }

    pub fn children(&self, label: CfgLabel) -> HashSet<CfgLabel> {
        self.0.get(&label).into_iter().flatten().copied().collect()
    }
}
