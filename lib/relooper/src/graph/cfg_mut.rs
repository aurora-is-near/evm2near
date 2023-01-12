//todo move all cfg-related stuff to another package?

use crate::graph::cfg::{Cfg, CfgEdge, CfgLabel};

impl<TLabel: CfgLabel> Cfg<TLabel> {
    fn add_edge(&mut self, from: TLabel, edge: CfgEdge<TLabel>) {
        assert!(self.out_edges.insert(from, edge).is_none());
        for to in edge.to_vec() {
            assert!(self.in_edges.entry(to).or_default().insert(from));
        }
    }

    fn remove_edge(&mut self, from: TLabel) {
        let to = self.out_edges.remove(&from).unwrap().to_vec();
        for t in to {
            assert!(self.in_edges.get_mut(&t).unwrap().remove(&from))
        }
    }

    fn remove_node(&mut self, node: TLabel) {
        assert!(self.nodes().contains(&node));
        self.out_edges.remove(&node);
    }
}
