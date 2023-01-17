//todo move all cfg-related stuff to another package?

use crate::graph::cfg::{Cfg, CfgEdge, CfgLabel};
use std::fmt::Debug;

impl<TLabel: CfgLabel + Debug> Cfg<TLabel> {
    pub fn add_edge(&mut self, from: TLabel, edge: CfgEdge<TLabel>) {
        assert!(self.out_edges.insert(from, edge).is_none());
    }

    pub fn remove_edge(&mut self, from: TLabel, edge: CfgEdge<TLabel>) {
        assert_eq!(self.out_edges.remove(&from), Some(edge));
    }
}
