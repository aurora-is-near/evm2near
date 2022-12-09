use crate::cfg::{Cfg, CfgLabel};
use crate::re_graph::ReLabel::{FromCfg, Generated};
use crate::re_graph::{ReBlock, ReGraph, ReLabel};
use crate::traversal::graph;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::iter::once;

#[derive()]
struct DomTree {
    pub dominates: HashMap<CfgLabel, HashSet<CfgLabel>>,
    pub dominated: HashMap<CfgLabel, CfgLabel>,
}

impl From<Vec<(CfgLabel, CfgLabel)>> for DomTree {
    fn from(edges: Vec<(CfgLabel, CfgLabel)>) -> Self {
        let dominated = HashMap::from_iter(edges.iter().copied());
        let mut dominates: HashMap<CfgLabel, HashSet<CfgLabel>> = HashMap::new();

        for (dominated, dominator) in edges {
            dominates.entry(dominator).or_default().insert(dominated);
        }

        DomTree {
            dominates,
            dominated,
        }
    }
}

enum Context {
    If,
    LoopHeadedBy(CfgLabel),
    BlockHeadedBy(CfgLabel),
}

struct ReBuilder<'a> {
    cfg: &'a Cfg,
    entry: CfgLabel,
    blocks: HashMap<ReLabel, Option<ReBlock>>,
    reachability: HashMap<CfgLabel, HashSet<CfgLabel>>,
    postorder_rev: Vec<CfgLabel>,
    domitation: DomTree,
    last_generated_label: isize,
}

impl<'a> ReBuilder<'a> {
    fn generate_label(&mut self) -> ReLabel {
        self.last_generated_label += 1;
        Generated(self.last_generated_label)
    }

    fn reachable(&self, l: &CfgLabel) -> &HashSet<CfgLabel> {
        self.reachability
            .get(l)
            .expect("that label should be in the initial cfg")
    }

    fn processed_cfg_labels(&self) -> HashSet<CfgLabel> {
        self.blocks
            .keys()
            .filter_map(|&b| match b {
                FromCfg(l) => Some(l),
                _ => None,
            })
            .collect()
    }

    pub fn create(cfg: &Cfg, entry: CfgLabel) -> ReBuilder {
        let nodes = cfg.nodes();

        let reachability: HashMap<CfgLabel, HashSet<CfgLabel>> = nodes
            .into_iter()
            .map(|l| {
                let reachable: HashSet<_> =
                    graph::bfs::Bfs::start_from_except(l, |&l| cfg.children(l).into_iter())
                        .collect();
                (l, reachable)
            })
            .collect();
        println!("n{:?}", reachability);

        ReBuilder {
            cfg,
            entry,
            blocks: HashMap::new(),
            reachability,
            postorder_rev: todo!(),
            domitation: todo!(),
            last_generated_label: 0,
        }
    }

    fn do_branch(&mut self, from: CfgLabel, to: CfgLabel) -> ReBlock {
        todo!()
    }

    fn node_within(&mut self, node: CfgLabel, merges: Vec<CfgLabel>) -> ReBlock {
        todo!()
    }

    fn do_tree(&mut self, node: CfgLabel, context: Vec<Context>) -> ReBlock {
        todo!()
    }

    fn dummy() -> ReGraph {
        let mut m = BTreeMap::new();
        m.insert(FromCfg(0), ReBlock::iff(FromCfg(0), FromCfg(1), FromCfg(2)));
        m.insert(FromCfg(1), ReBlock::block(FromCfg(1), FromCfg(3)));
        m.insert(FromCfg(2), ReBlock::block(FromCfg(2), FromCfg(3)));
        m.insert(FromCfg(3), ReBlock::block(FromCfg(3), FromCfg(4)));
        m.insert(FromCfg(4), ReBlock::looop(FromCfg(4), FromCfg(0)));

        ReGraph(m)
    }

    pub fn reloop(mut self) -> ReGraph {
        let reachable_from_start = self.reachable(&self.entry).to_owned();

        let re_entry = self.do_tree(self.entry, Vec::new());

        Self::dummy()
    }
}
