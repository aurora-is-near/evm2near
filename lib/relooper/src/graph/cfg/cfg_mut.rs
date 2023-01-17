//todo move all cfg-related stuff to another package?

use crate::graph::cfg::{Cfg, CfgEdge, CfgLabel};

impl<TLabel: CfgLabel> Cfg<TLabel> {
    pub fn add_edge(&mut self, from: TLabel, edge: CfgEdge<TLabel>) {
        assert!(self.out_edges.insert(from, edge).is_none());
        for to in edge.to_vec() {
            assert!(self.in_edges.entry(to).or_default().insert(from));
        }
    }
}
