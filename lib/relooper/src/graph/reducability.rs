use crate::graph::cfg::CfgEdge;
use crate::graph::cfg::CfgLabel;
use crate::Cfg;
use crate::EnrichedCfg;
use std::collections::{HashMap, HashSet};
use std::default;
pub type Color = usize;


pub struct ColoredCfg {
    cfg : Cfg,
    colors : HashMap<CfgLabel, Color>,
}

impl ColoredCfg {
    pub fn new(cfg : &Cfg) -> ColoredCfg {
        let mut colors : HashMap<CfgLabel, Color> = HashMap::default();
        for (lbl, edge) in &cfg.out_edges {
            colors.insert(*lbl, *lbl);
        }
        return ColoredCfg { cfg: cfg.clone(), colors: colors };
    }

    pub fn as_cfg(&self) -> Cfg {
        return self.cfg.clone();
    }


    pub fn reduce_colors(&mut self) -> () {
        loop {
            match self.mergeble_colors() {
                Some((master, slave)) => {
                    self.merge(master, slave);
                    continue;
                }
                None => {}
            }
            break;
        }
    }

    pub fn merge(&mut self, master : CfgLabel, slave : CfgLabel) -> () {
        for (label, mut color) in &mut self.colors {
            if *color == slave {
                *color = master;
            }
        }
    }


    /// returns pair of colors (master, slave) if all precessors of all nodes with color = slave
    /// have color = slave or color = master
    pub fn mergeble_colors(&self) -> Option<(Color, Color)> {
        let mut precs : HashMap<Color, HashSet<Color>> = HashMap::default();
        for (node, edge) in &self.cfg.out_edges {
            match edge {
                CfgEdge::Cond(cond, uncond) => {
                    precs.entry(*self.colors.get(cond).unwrap()).or_default().insert(*self.colors.get(node).unwrap());
                    precs.entry(*self.colors.get(uncond).unwrap()).or_default().insert(*self.colors.get(node).unwrap());
                }
                CfgEdge::Uncond(uncond) => {
                    precs.entry(*self.colors.get(uncond).unwrap()).or_default().insert(*self.colors.get(node).unwrap());
                }
                CfgEdge::Terminal => {panic!("Here should not be terminal!");}
            }
        }
        for (color, mut precolors) in precs {
            precolors.remove(&color);
            if precolors.len() == 1 {
                return Some((precolors.into_iter().next().unwrap(), color));
            }
        }
        return None;
    }
}


#[test]
pub fn test_create() {
    let graph = Cfg::from_edges(
        vec![
            (0, CfgEdge::Cond(1, 2)),
            (1, CfgEdge::Cond(3, 5)),
            (2, CfgEdge::Uncond(3)),
            (3, CfgEdge::Uncond(4)),
            (5, CfgEdge::Cond(6, 7)),
            (6, CfgEdge::Uncond(8)),
            (7, CfgEdge::Uncond(8)),
            (4, CfgEdge::Uncond(9)),
            (8, CfgEdge::Cond(9, 5)),
        ],
        0,
    ).unwrap();
    let cgraph = ColoredCfg::new(&graph);
    let graph2 = cgraph.as_cfg();
    assert_eq!(graph.out_edges, graph2.out_edges);
    assert_eq!(graph.entry, graph2.entry);
}

#[test]
pub fn test_merge() {
    let graph = Cfg::from_edges(
        vec![
            (0, CfgEdge::Cond(1, 2)),
            (1, CfgEdge::Cond(3, 5)),
            (2, CfgEdge::Uncond(3)),
            (3, CfgEdge::Uncond(4)),
            (5, CfgEdge::Cond(6, 7)),
            (6, CfgEdge::Uncond(8)),
            (7, CfgEdge::Uncond(8)),
            (4, CfgEdge::Uncond(9)),
            (8, CfgEdge::Cond(9, 5)),
        ],
        0,
    ).unwrap();
    let mut cgraph = ColoredCfg::new(&graph);
    cgraph.merge(6, 7);
    assert_eq!(*cgraph.colors.get(&7).unwrap(), 6);
    cgraph.merge(1, 2);
    assert_eq!(*cgraph.colors.get(&2).unwrap(), 1);
}
