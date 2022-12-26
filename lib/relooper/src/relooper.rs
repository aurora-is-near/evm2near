use crate::cfg::CfgEdge::*;
use crate::cfg::{Cfg, CfgLabel};
use crate::relooper::ReBlock::*;
use crate::traversal::graph;
use crate::traversal::graph::dfs::dfs_post;
use std::collections::{HashMap, HashSet};

pub struct ReSeq(pub Vec<ReBlock>);

pub enum ReBlock {
    Block(ReSeq),
    Loop(ReSeq),
    If(ReSeq, ReSeq),

    Actions(CfgLabel),
    Br(usize),
    Return,
}

impl ReBlock {
    pub(crate) fn concat(self, other: ReSeq) -> ReSeq {
        let mut blocks = vec![self];
        blocks.extend(other.0);
        ReSeq(blocks)
    }
}

impl ReSeq {
    pub(crate) fn single(block: ReBlock) -> ReSeq {
        ReSeq(vec![block])
    }
}

#[derive(Default)]
pub struct DomTree {
    dominates: HashMap<CfgLabel, HashSet<CfgLabel>>,
    pub(crate) dominated: HashMap<CfgLabel, CfgLabel>,
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

pub struct NodeOrdering {
    idx: HashMap<CfgLabel, usize>,
    vec: Vec<CfgLabel>,
}

impl NodeOrdering {
    pub fn new(cfg: &Cfg, entry: CfgLabel) -> Self {
        let vec = dfs_post(entry, &mut |x| cfg.children(*x));
        let idx: HashMap<CfgLabel, usize> = vec.iter().enumerate().map(|(i, &n)| (n, i)).collect();
        Self { vec, idx }
    }

    pub fn is_backward(&self, from: CfgLabel, to: CfgLabel) -> bool {
        self.idx
            .get(&from)
            .zip(self.idx.get(&to))
            .map(|(&f, &t)| f > t)
            .unwrap()
    }

    pub fn is_forward(&self, from: CfgLabel, to: CfgLabel) -> bool {
        !self.is_backward(from, to)
    }

    pub fn sequence(&self) -> &Vec<CfgLabel> {
        &self.vec
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
    // reachability: HashMap<CfgLabel, HashSet<CfgLabel>>,
    ordering: NodeOrdering,
    domitation: DomTree,
    ifs: HashSet<CfgLabel>,
    loops: HashSet<CfgLabel>,
    merges: HashSet<CfgLabel>,
}

impl<'a> Relooper<'a> {
    fn children(&self, label: CfgLabel) -> Vec<CfgLabel> {
        let mut res = self
            .domitation
            .immediately_dominated_by(label)
            .into_iter()
            .collect::<Vec<_>>();
        res.sort_by_key(|n| {
            self.ordering
                .idx
                .get(n)
                .expect("every node should have postorder numbering")
        });
        res
    }

    fn do_branch(&mut self, from: CfgLabel, to: CfgLabel, context: &Vec<Context>) -> ReSeq {
        if self.ordering.is_backward(from, to) || self.merges.contains(&to) {
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
            ReSeq(vec![Br(jump_idx)]) //TODO is seq really necessary there?
        } else {
            self.do_tree(to, context)
        }
    }

    fn node_within(
        &mut self,
        node: CfgLabel,
        merges: &Vec<CfgLabel>,
        context: &Vec<Context>,
    ) -> ReSeq {
        let mut current_merges = merges.clone();
        match current_merges.pop() {
            Some(merge) => {
                let mut new_ctx = context.clone();
                new_ctx.push(Context::BlockHeadedBy(merge));
                let inner = self.node_within(node, &current_merges, &new_ctx);
                let merge_block = self.do_tree(merge, context);

                Block(inner).concat(merge_block)
            }
            None => {
                let actions = Actions(node);
                match *self.cfg.edge(node) {
                    Uncond(u) => actions.concat(self.do_branch(node, u, context)),
                    Cond(true_label, false_label) => {
                        let mut if_context = context.clone();
                        if_context.push(Context::If);

                        let true_branch = self.do_branch(node, true_label, &if_context);
                        let false_branch = self.do_branch(node, false_label, &if_context);

                        ReSeq(vec![If(true_branch, false_branch)])
                    }
                    Terminal => ReSeq(vec![Return]),
                }
            }
        }
    }

    fn gen_node(&mut self, node: CfgLabel, context: &Vec<Context>) -> ReSeq {
        let merge_children: Vec<CfgLabel> = self
            .children(node)
            .into_iter()
            .filter(|n| self.merges.contains(n))
            .collect();
        self.node_within(node, &merge_children, context)
    }

    fn do_tree(&mut self, node: CfgLabel, context: &Vec<Context>) -> ReSeq {
        if self.loops.contains(&node) {
            let mut ctx = context.clone();
            ctx.push(Context::LoopHeadedBy(node));
            ReSeq::single(Loop(self.gen_node(node, &ctx)))
        } else {
            self.gen_node(node, context)
        }
    }
}

pub fn reloop(cfg: &Cfg, entry: CfgLabel) -> ReSeq {
    let nodes = cfg.nodes();

    let reachability: HashMap<CfgLabel, HashSet<CfgLabel>> = nodes
        .into_iter()
        .map(|l| {
            let reachable: HashSet<_> =
                graph::bfs::Bfs::start_from_except(l, |&l| cfg.children(l).into_iter()).collect();
            (l, reachable)
        })
        .collect();

    let postorder_rev = dfs_post(entry, &mut |x| cfg.children(*x))
        .into_iter()
        .enumerate()
        .map(|(i, n)| (n, i))
        .collect::<HashMap<_, _>>();

    let mut relooper = Relooper {
        cfg,
        entry,
        ordering: NodeOrdering::new(cfg, entry),
        domitation: todo!(), //TODO
        ifs: Default::default(),
        loops: Default::default(),
        merges: Default::default(),
    };

    relooper.do_tree(entry, &Vec::new())
}
