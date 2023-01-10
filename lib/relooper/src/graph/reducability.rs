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
    next_cfg_id : CfgLabel,
}

impl ColoredCfg {
    pub fn new(cfg : &Cfg) -> ColoredCfg {
        let mut colors : HashMap<CfgLabel, Color> = HashMap::default();
        let mut id : CfgLabel = 0;
        for (lbl, edge) in &cfg.out_edges {
            colors.insert(*lbl, *lbl);
            if id < *lbl {
                id = *lbl;
            }
        }
        return ColoredCfg { cfg: cfg.clone(), colors: colors, next_cfg_id: id};
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
            match self.splittable_colors() {
                Some((masters, slave)) => {
                    self.split(masters, slave);
                    continue;
                }
                None => {}
            }
            break;
        }
        let  mut different_colors : HashSet<Color> = HashSet::default();
        for (label, color) in &self.colors {
            different_colors.insert(*color);
        }
        assert_eq!(different_colors.len(), 1);
    }

    pub fn merge(&mut self, master : Color, slave : Color) -> () {
        println!("MERGE master = {}, slave = {}", master, slave);
        for (label, mut color) in &mut self.colors {
            if *color == slave {
                *color = master;
            }
        }
    }

    pub fn split(&mut self, mut masters : HashSet<Color>, slave : Color) -> () {
        println!("SPLIT slave = {}", slave);
        // first delete one random master
        let random = masters.iter().next().unwrap().clone();
        masters.remove(&random);
        // then consequently make a copy of slave for each master color
        for master in &masters {
            // find all nodes with color = slave
            let mut slaves : HashSet<CfgLabel> = HashSet::default();
            for (label, color) in &self.colors {
                if *color == slave {
                    slaves.insert(*label);
                }
            }
            // find all nodes with this master color
            let mut masternodes : HashSet<CfgLabel> = HashSet::default();
            for (label, color) in &self.colors {
                if *color == *master {
                    masternodes.insert(*label);
                }
            }
            // make a copy of all nodes with color = slave for this master
            // with all outedges
            let mut origin2clone : HashMap<CfgLabel, CfgLabel> = HashMap::default();
            for slave_node in &slaves {
                let copy_label = self.next_cfg_id; self.next_cfg_id += 1;
                origin2clone.insert(*slave_node, copy_label);
                self.colors.insert(copy_label, *master);
                let edge = self.cfg.out_edges.get(&slave_node).unwrap().clone();
                self.cfg.out_edges.insert(copy_label, edge);
            }
            // switch direction of inedges from this master to copyes
            for node in masternodes {
                let edge = self.cfg.out_edges.get_mut(&node).unwrap();
                match edge {
                    CfgEdge::Cond(cond, uncond) => {
                        let new_cond = if slaves.contains(cond) {*origin2clone.get(cond).unwrap()} else {*cond};
                        let new_uncond = if slaves.contains(uncond) {*origin2clone.get(uncond).unwrap()} else {*uncond};
                        *edge = CfgEdge::Cond(new_cond, new_uncond);
                    }
                    CfgEdge::Uncond(uncond) => {
                        let new_uncond = if slaves.contains(uncond) {*origin2clone.get(uncond).unwrap()} else {*uncond};
                        *edge = CfgEdge::Uncond(new_uncond);
                    }
                    CfgEdge::Terminal => {}
                }
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
                CfgEdge::Terminal => {}
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


    /// returns group of colors (masters, slave) if all precessors of all nodes with color = slave
    /// have color = slave or masters.contain(color)
    pub fn splittable_colors(&self) -> Option<(HashSet<Color>, Color)> {
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
                CfgEdge::Terminal => {}
            }
        }
        for (color, mut precolors) in precs {
            precolors.remove(&color);
            if precolors.len() > 1 {
                return Some((precolors, color));
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


#[test]
pub fn test_reducable() {
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
    cgraph.reduce_colors();
    let  mut different_colors : HashSet<Color> = HashSet::default();
    for (label, color) in cgraph.colors {
        different_colors.insert(color);
    }
    assert_eq!(different_colors.len(), 1);
}

#[test]
pub fn test_irreducable() {
    let graph = Cfg::from_edges(
        vec![
            (0, CfgEdge::Cond(1, 2)),
            (1, CfgEdge::Cond(2, 3)),
            (2, CfgEdge::Uncond(1)),
        ],
        0,
    ).unwrap();
    let mut cgraph = ColoredCfg::new(&graph);
    cgraph.reduce_colors();
    let reduced = cgraph.as_cfg();

    let e_graph = EnrichedCfg::new(reduced);

    let dot_lines: Vec<String> = vec![
        "digraph {".to_string(),
        e_graph.cfg_to_dot(),
        "}".to_string(),
    ];
    std::fs::write("relooped.dot", dot_lines.join("\n")).expect("fs error");
}