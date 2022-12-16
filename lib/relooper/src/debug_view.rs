use crate::cfg::CfgEdge;
use crate::re_graph::ReBlockType;
use crate::{Cfg, CfgLabel, ReBlock, ReGraph, ReLabel};
use dot::{Edges, Id, Nodes, Style};
use std::borrow::Cow;

impl<'a> dot::Labeller<'a, CfgLabel, (CfgLabel, CfgEdge)> for Cfg {
    fn graph_id(&'a self) -> Id<'a> {
        Id::new("cfg").unwrap()
    }

    fn node_id(&'a self, n: &CfgLabel) -> Id<'a> {
        Id::new(format!("n{}", n)).unwrap()
    }

    fn edge_style(&'a self, (_f, (_t, is_cond)): &(CfgLabel, CfgEdge)) -> Style {
        if *is_cond {
            Style::Dashed
        } else {
            Style::None
        }
    }
}

impl<'a> dot::GraphWalk<'a, CfgLabel, (CfgLabel, CfgEdge)> for Cfg {
    fn nodes(&'a self) -> Nodes<'a, CfgLabel> {
        let nodes = self.nodes();
        let v = nodes.into_iter().collect::<Vec<CfgLabel>>();
        Cow::Owned(v)
    }

    fn edges(&'a self) -> Edges<'a, (CfgLabel, CfgEdge)> {
        Cow::Owned(self.edges_raw().into_iter().collect())
    }

    fn source(&'a self, &(from, _to): &(CfgLabel, CfgEdge)) -> CfgLabel {
        from
    }

    fn target(&'a self, &(_from, (to, is_cond)): &(CfgLabel, CfgEdge)) -> CfgLabel {
        to
    }
}

// impl<'a> dot::Labeller<'a, ReBlock, (ReLabel, ReLabel)> for ReGraph {
//     fn graph_id(&'a self) -> Id<'a> {
//         Id::new("relooped").unwrap()
//     }
//
//     fn node_id(&'a self, b: &ReBlock) -> Id<'a> {
//         Id::new(format!("{:?}{:?}", b.block_type, b.curr)).unwrap()
//     }
// }
//
// impl<'a> dot::GraphWalk<'a, ReBlock, (ReLabel, ReLabel)> for ReGraph {
//     fn nodes(&'a self) -> Nodes<'a, ReBlock> {
//         let v: Vec<ReBlock> = self.0.iter().map(|(_l, block)| *block).collect();
//         Cow::Owned(v)
//     }
//
//     fn edges(&'a self) -> Edges<'a, (ReLabel, ReLabel)> {
//         Cow::Owned(
//             self.0
//                 .clone()
//                 .into_values()
//                 .flat_map(|b| match b.block_type {
//                     ReBlockType::If => vec![(b.curr, b.inner), (b.curr, b.next)],
//                     _ => vec![(b.curr, b.next)],
//                 })
//                 .collect(),
//         ) //TODO
//     }
//
//     fn source(&'a self, (from, _to): &(ReLabel, ReLabel)) -> ReBlock {
//         *self.0.get(from).unwrap()
//     }
//
//     fn target(&'a self, (_from, to): &(ReLabel, ReLabel)) -> ReBlock {
//         *self.0.get(to).unwrap()
//     }
// }
