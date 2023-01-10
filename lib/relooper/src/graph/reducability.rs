use crate::graph::cfg::CfgEdge;
use crate::graph::cfg::CfgLabel;
use crate::Cfg;
use crate::EnrichedCfg;
use std::collections::{BTreeMap, BTreeSet};
pub type Color = usize;

/// This is struct helper to convert from irreducable graph to
/// equivalent reducable. Reducable graphs are graphs with single loopheader.
/// Main idea of algorithm:
///   1) Lets firstly paint each cfg node to
pub struct ColoredCfg {
    cfg: Cfg,
    colors: BTreeMap<CfgLabel, Color>,
    next_cfg_id: CfgLabel,
    clone2origin: BTreeMap<CfgLabel, CfgLabel>,
}

impl ColoredCfg {
    /// Constructor from Cfg
    pub fn new(cfg: &Cfg) -> ColoredCfg {
        let mut colors: BTreeMap<CfgLabel, Color> = BTreeMap::default();
        let mut id: CfgLabel = 0;
        for (lbl, _edge) in &cfg.out_edges {
            colors.insert(*lbl, *lbl);
            if id < *lbl {
                id = *lbl;
            }
        }
        id += 1;
        return ColoredCfg {
            cfg: cfg.clone(),
            colors: colors,
            next_cfg_id: id,
            clone2origin: BTreeMap::default(),
        };
    }

    /// This function returns cfg stored by colored graph. (maybe this cfg was previously modified by colored graph)
    pub fn as_cfg(&self) -> Cfg {
        return self.cfg.clone();
    }

    /// This is main function that make graph reducing. Now it works greedy: infinite loop, if can merge -- do merge and to next
    /// iteration. If can't merge but can split -- do split and to next iteration. If can't both -- return.
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
        // Just debug assertation to check algorithms invariant. Can be removed.
        let mut different_colors: BTreeSet<Color> = BTreeSet::default();
        for (_label, color) in &self.colors {
            different_colors.insert(*color);
        }
        assert_eq!(different_colors.len(), 1);
    }

    /// This function merges two colors. It simply recolor all nodes with color = slave to master color.
    pub fn merge(&mut self, master: Color, slave: Color) -> () {
        for (_label, mut color) in &mut self.colors {
            if *color == slave {
                *color = master;
            }
        }
    }

    /// Most difficult for understading function in this struct.
    /// Main idea: lets go throw master colors. For each master color we create
    /// copyes of all slave nodes. (All slave nodes are cloned with their outedges).
    /// Than, if there was edge between two slave nodes, now we have edge from clone-slave to origin-slave.
    /// So, next step is switch such edges to clones-slaves.
    /// Last step is switch edges of current master nodes from origin-slaves to clones-slaves.
    ///
    /// And a nice trick. If we do it for all masters we will need to delete original slaves. Better is just skip
    /// this operation for one master. This master will have outedges to original slaves (and only this master).
    pub fn split(&mut self, mut masters: BTreeSet<Color>, slave: Color) -> () {
        // first delete one random master
        let random = masters.iter().next().unwrap().clone();
        masters.remove(&random);
        // then consequently make a copy of slave for each master color
        for master in &masters {
            // find all nodes with color = slave
            let mut slaves: BTreeSet<CfgLabel> = BTreeSet::default();
            for (label, color) in &self.colors {
                if *color == slave {
                    slaves.insert(*label);
                }
            }
            // find all nodes with this master color
            let mut masternodes: BTreeSet<CfgLabel> = BTreeSet::default();
            for (label, color) in &self.colors {
                if *color == *master {
                    masternodes.insert(*label);
                }
            }
            // make a copy of all nodes with color = slave for this master
            // with all outedges
            let mut origin2clone: BTreeMap<CfgLabel, CfgLabel> = BTreeMap::default();
            let mut clones: BTreeSet<CfgLabel> = BTreeSet::default();
            for slave_node in &slaves {
                let copy_label = self.next_cfg_id;
                self.next_cfg_id += 1;
                origin2clone.insert(*slave_node, copy_label);
                self.clone2origin.insert(copy_label, *slave_node);
                clones.insert(copy_label);
                self.colors.insert(copy_label, *master);
                let edge = self.cfg.out_edges.get(&slave_node).unwrap().clone();
                self.cfg.out_edges.insert(copy_label, edge);
            }
            // fix edges between slave nodes
            for node in &clones {
                let edge = self.cfg.out_edges.get_mut(&node).unwrap();
                match edge {
                    CfgEdge::Cond(cond, uncond) => {
                        let new_cond = if slaves.contains(cond) {
                            *origin2clone.get(cond).unwrap()
                        } else {
                            *cond
                        };
                        let new_uncond = if slaves.contains(uncond) {
                            *origin2clone.get(uncond).unwrap()
                        } else {
                            *uncond
                        };
                        *edge = CfgEdge::Cond(new_cond, new_uncond);
                    }
                    CfgEdge::Uncond(uncond) => {
                        let new_uncond = if slaves.contains(uncond) {
                            *origin2clone.get(uncond).unwrap()
                        } else {
                            *uncond
                        };
                        *edge = CfgEdge::Uncond(new_uncond);
                    }
                    CfgEdge::Terminal => {}
                }
            }
            // switch direction of inedges from this master to copyes
            for node in masternodes {
                let edge = self.cfg.out_edges.get_mut(&node).unwrap();
                match edge {
                    CfgEdge::Cond(cond, uncond) => {
                        let new_cond = if slaves.contains(cond) {
                            *origin2clone.get(cond).unwrap()
                        } else {
                            *cond
                        };
                        let new_uncond = if slaves.contains(uncond) {
                            *origin2clone.get(uncond).unwrap()
                        } else {
                            *uncond
                        };
                        *edge = CfgEdge::Cond(new_cond, new_uncond);
                    }
                    CfgEdge::Uncond(uncond) => {
                        let new_uncond = if slaves.contains(uncond) {
                            *origin2clone.get(uncond).unwrap()
                        } else {
                            *uncond
                        };
                        *edge = CfgEdge::Uncond(new_uncond);
                    }
                    CfgEdge::Terminal => {}
                }
            }
        }
    }

    /// This function returns pair of colors (master, slave) if all precessors of all nodes with color = slave
    /// have color = slave or color = master. If there is no such nodes function returns None.
    pub fn mergeble_colors(&self) -> Option<(Color, Color)> {
        let mut precs: BTreeMap<Color, BTreeSet<Color>> = BTreeMap::default();
        for (node, edge) in &self.cfg.out_edges {
            match edge {
                CfgEdge::Cond(cond, uncond) => {
                    precs
                        .entry(*self.colors.get(cond).unwrap())
                        .or_default()
                        .insert(*self.colors.get(node).unwrap());
                    precs
                        .entry(*self.colors.get(uncond).unwrap())
                        .or_default()
                        .insert(*self.colors.get(node).unwrap());
                }
                CfgEdge::Uncond(uncond) => {
                    precs
                        .entry(*self.colors.get(uncond).unwrap())
                        .or_default()
                        .insert(*self.colors.get(node).unwrap());
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

    /// This function returns group of colors (masters, slave) if all precessors of all nodes with color = slave
    /// have color = slave or masters.contain(color). If there is no such nodes it return None.
    pub fn splittable_colors(&self) -> Option<(BTreeSet<Color>, Color)> {
        let mut precs: BTreeMap<Color, BTreeSet<Color>> = BTreeMap::default();
        for (node, edge) in &self.cfg.out_edges {
            match edge {
                CfgEdge::Cond(cond, uncond) => {
                    precs
                        .entry(*self.colors.get(cond).unwrap())
                        .or_default()
                        .insert(*self.colors.get(node).unwrap());
                    precs
                        .entry(*self.colors.get(uncond).unwrap())
                        .or_default()
                        .insert(*self.colors.get(node).unwrap());
                }
                CfgEdge::Uncond(uncond) => {
                    precs
                        .entry(*self.colors.get(uncond).unwrap())
                        .or_default()
                        .insert(*self.colors.get(node).unwrap());
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
    )
    .unwrap();
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
    )
    .unwrap();
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
    )
    .unwrap();
    let mut cgraph = ColoredCfg::new(&graph);
    cgraph.reduce_colors();
    let mut different_colors: BTreeSet<Color> = BTreeSet::default();
    for (_label, color) in cgraph.colors {
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
    )
    .unwrap();
    let mut cgraph = ColoredCfg::new(&graph);
    cgraph.reduce_colors();
    let reduced = cgraph.as_cfg();
    let e_graph = EnrichedCfg::new(reduced);
    let dot_lines: Vec<String> = vec![
        "digraph {".to_string(),
        e_graph.cfg_to_dot(),
        "}".to_string(),
    ];
    std::fs::write("reduced.dot", dot_lines.join("\n")).expect("fs error");
}

#[test]
pub fn test_irreducable2() {
    let graph = Cfg::from_edges(
        vec![
            (0, CfgEdge::Uncond(1)),
            (1, CfgEdge::Cond(2, 3)),
            (2, CfgEdge::Cond(4, 3)),
            (3, CfgEdge::Cond(2, 5)),
            (4, CfgEdge::Cond(6, 5)),
            (5, CfgEdge::Cond(4, 7)),
        ],
        0,
    )
    .unwrap();
    let mut cgraph = ColoredCfg::new(&graph);
    cgraph.reduce_colors();
    let reduced = cgraph.as_cfg();
    let e_graph = EnrichedCfg::new(reduced);
    let dot_lines: Vec<String> = vec![
        "digraph {".to_string(),
        e_graph.cfg_to_dot(),
        "}".to_string(),
    ];
    std::fs::write("reduced2.dot", dot_lines.join("\n")).expect("fs error");
}

#[test]
pub fn test_irreducable3() {
    let graph = Cfg::from_edges(
        vec![
            (0, CfgEdge::Cond(1, 2)),
            (1, CfgEdge::Cond(3, 4)),
            (2, CfgEdge::Cond(4, 5)),
            (3, CfgEdge::Cond(4, 6)),
            (4, CfgEdge::Cond(3, 5)),
            (5, CfgEdge::Cond(4, 7)),
        ],
        0,
    )
    .unwrap();
    let mut cgraph = ColoredCfg::new(&graph);
    cgraph.reduce_colors();
    let reduced = cgraph.as_cfg();
    let e_graph = EnrichedCfg::new(reduced);
    let dot_lines: Vec<String> = vec![
        "digraph {".to_string(),
        e_graph.cfg_to_dot(),
        "}".to_string(),
    ];
    std::fs::write("reduced3.dot", dot_lines.join("\n")).expect("fs error");
}
