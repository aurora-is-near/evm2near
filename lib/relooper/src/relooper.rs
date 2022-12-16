use crate::cfg::{Cfg, CfgLabel};
use crate::re_graph::ReBlockType::{Block, If, Loop};
use crate::re_graph::ReLabel::{FromCfg, Generated};
use crate::re_graph::{ReBlock, ReGraph, ReLabel};
use crate::traversal::graph;
use std::collections::{BTreeMap, HashMap, HashSet};

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

impl DomTree {
    fn immediately_dominated_by(&self, label: CfgLabel) -> HashSet<CfgLabel> {
        self.dominates
            .get(&label)
            .unwrap_or(&HashSet::new())
            .to_owned()
    }
}

#[derive(Clone, Copy)]
enum Context {
    If,
    LoopHeadedBy(CfgLabel),
    BlockHeadedBy(CfgLabel),
}

struct Relooper<'a> {
    cfg: &'a Cfg,
    entry: CfgLabel,
    blocks: HashMap<ReLabel, Option<ReBlock>>,
    reachability: HashMap<CfgLabel, HashSet<CfgLabel>>,
    postorder_rev: HashMap<CfgLabel, usize>,
    domitation: DomTree,
    last_generated_label: usize,
    ifs: HashSet<CfgLabel>,
    loops: HashSet<CfgLabel>,
    merges: HashSet<CfgLabel>,
}

impl<'a> Relooper<'a> {
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

    pub fn create(cfg: &Cfg, entry: CfgLabel) -> Relooper {
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

        Relooper {
            cfg,
            entry,
            blocks: HashMap::new(),
            reachability,
            postorder_rev: todo!(),
            domitation: todo!(),
            last_generated_label: 0,
            ifs: Default::default(),
            loops: Default::default(),
            merges: Default::default(),
        }
    }

    fn children(&self, label: CfgLabel) -> Vec<CfgLabel> {
        let mut res = self
            .domitation
            .immediately_dominated_by(label)
            .into_iter()
            .collect::<Vec<_>>();
        res.sort_by_key(|n| {
            self.postorder_rev
                .get(n)
                .expect("every node should have postorder numbering")
        });
        res
    }

    fn is_backward(&self, from: CfgLabel, to: CfgLabel) -> bool {
        self.postorder_rev
            .get(&from)
            .and_then(|&f| self.postorder_rev.get(&to).map(|&t| f < t))
            .unwrap()
    }

    fn is_forward(&self, from: CfgLabel, to: CfgLabel) -> bool {
        !self.is_backward(from, to)
    }

    fn do_branch(&self, from: CfgLabel, to: CfgLabel, context: Vec<Context>) -> Option<usize> {
        if self.is_backward(from, to) || self.merges.contains(&to) {
            let idx_coll = context
                .iter()
                .enumerate()
                .filter_map(|(i, c)| match c {
                    Context::LoopHeadedBy(label) | Context::BlockHeadedBy(label)
                        if *label == to =>
                    {
                        Some(context.len() - i - 1)
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();

            assert_eq!(idx_coll.len(), 1);
            let &jump_idx = idx_coll
                .first()
                .expect("suitable jump target not found in context");
            Some(jump_idx)
        } else {
            None
        }
    }

    fn node_within(
        &mut self,
        node: CfgLabel,
        merges: Vec<CfgLabel>,
        context: Vec<Context>,
    ) -> ReBlock {
        let mut current_merges = merges.clone();
        current_merges.pop().map_or_else(
            || todo!(),
            |y| {
                let mut new_ctx = context.clone();
                new_ctx.push(Context::BlockHeadedBy(y));
                let inner = self.node_within(node, current_merges, new_ctx);
                let curr = self.do_tree(y, context);
                todo!("concat inner & curr, so returning value needed to be changed to smth")
            },
        )
    }

    fn gen_node(&mut self, node: CfgLabel, context: Vec<Context>) -> ReBlock {
        let merge_children: Vec<CfgLabel> = self
            .children(node)
            .into_iter()
            .filter(|n| self.merges.contains(n))
            .collect();
        self.node_within(node, merge_children, context)
    }

    fn do_tree(&mut self, node: CfgLabel, context: Vec<Context>) -> ReBlock {
        if self.loops.contains(&node) {
            let mut ctx = context.clone();
            ctx.push(Context::LoopHeadedBy(node));
            let next_block = self.gen_node(node, context);
            ReBlock::new(Loop, FromCfg(node), next_block.label())
        } else {
            self.gen_node(node, context)
        }
    }

    fn dummy() -> ReGraph {
        let mut m = BTreeMap::new();
        m.insert(
            FromCfg(0),
            ReBlock::new(If(FromCfg(1)), FromCfg(0), FromCfg(2)),
        );
        m.insert(FromCfg(1), ReBlock::new(Block, FromCfg(1), FromCfg(3)));
        m.insert(FromCfg(2), ReBlock::new(Block, FromCfg(2), FromCfg(3)));
        m.insert(FromCfg(3), ReBlock::new(Block, FromCfg(3), FromCfg(4)));
        m.insert(FromCfg(4), ReBlock::new(Loop, FromCfg(4), FromCfg(0)));

        ReGraph(m)
    }

    pub fn reloop(mut self) -> ReGraph {
        let reachable_from_start = self.reachable(&self.entry).to_owned();

        let re_entry = self.do_tree(self.entry, Vec::new());

        Self::dummy()
    }
}
