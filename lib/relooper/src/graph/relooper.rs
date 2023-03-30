use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Display;

use crate::graph::cfg::{CfgEdge::*, CfgLabel};
use crate::graph::enrichments::EnrichedCfg;
use crate::graph::relooper::ReBlock::*;

#[derive(Debug)]
pub struct ReSeq<TLabel: CfgLabel>(pub Vec<ReBlock<TLabel>>);

/// describes relooped graph structure
/// consists of three "container" variants and several "actions" variants
/// containers define tree structure, actions denotes runtime control flow behaviour
#[derive(Debug)]
pub enum ReBlock<TLabel: CfgLabel> {
    Block(ReSeq<TLabel>),
    Loop(ReSeq<TLabel>),
    If(ReSeq<TLabel>, ReSeq<TLabel>),

    Actions(TLabel),
    Br(u32),
    TableJump(BTreeMap<usize, u32>),
    Return,
}

impl<TLabel: CfgLabel> ReBlock<TLabel> {
    pub(crate) fn cons(self, other: ReSeq<TLabel>) -> ReSeq<TLabel> {
        let mut blocks = vec![self];
        blocks.extend(other.0);
        ReSeq(blocks)
    }
}

impl<TLabel: CfgLabel> ReSeq<TLabel> {
    pub(crate) fn single(block: ReBlock<TLabel>) -> ReSeq<TLabel> {
        ReSeq(vec![block])
    }
}

#[derive(Clone, Copy, Debug)]
enum Context<TLabel: CfgLabel> {
    If,
    LoopHeadedBy(TLabel),
    BlockHeadedBy(TLabel),
}

impl<TLabel: CfgLabel> Context<TLabel> {
    fn label(&self) -> Option<&TLabel> {
        match self {
            Self::If => None,
            Self::LoopHeadedBy(l) | Self::BlockHeadedBy(l) => Some(l),
        }
    }
}

impl<TLabel: CfgLabel + Display> EnrichedCfg<TLabel> {
    /// returns set of immediately dominated nodes needed to be generated around the target node
    fn children(&self, label: TLabel) -> HashSet<TLabel> {
        self.domination.immediately_dominated_by(label)
    }

    /// either generates branch node or "fallthrough" next node
    fn do_branch(&self, from: TLabel, to: TLabel, context: &Vec<Context<TLabel>>) -> ReSeq<TLabel> {
        if self.node_ordering.is_backward(from, to) || self.merge_nodes.contains(&to) {
            let idx_coll = context
                .iter()
                .enumerate()
                .filter_map(|(i, c)| {
                    c.label().and_then(|&l| {
                        if l == to {
                            Some(
                                u32::try_from(context.len() - i - 1)
                                    .expect("unexpectedly far backwards jump"),
                            )
                        } else {
                            None
                        }
                    })
                })
                .collect::<Vec<_>>();

            assert_eq!(idx_coll.len(), 1);
            let &jump_idx = idx_coll
                .last()
                .expect("suitable jump target not found in context");
            ReSeq(vec![Br(jump_idx)])
        } else {
            self.do_tree(to, context)
        }
    }

    /// In case there is multiple nodes that should be generated around current node (outer nodes in tree structure), lays down them first.
    /// Otherwise, generates current node and branches to nodes generated on previous step (and above in tree structure).
    fn node_within(
        &self,
        node: TLabel,
        outer_nodes: &[TLabel],
        context: &Vec<Context<TLabel>>,
    ) -> ReSeq<TLabel> {
        if outer_nodes.is_empty() {
            let actions = Actions(node);
            let other = match self.cfg.edge(&node) {
                Uncond(u) => self.do_branch(node, *u, context),
                Cond(true_label, false_label) => {
                    let mut if_context = context.clone();
                    if_context.push(Context::If);

                    let true_branch = self.do_branch(node, *true_label, &if_context);
                    let false_branch = self.do_branch(node, *false_label, &if_context);

                    ReSeq(vec![If(true_branch, false_branch)])
                }
                Switch(v) => {
                    let context_len = context.len();
                    let context_map: HashMap<_, _> = context
                        .iter()
                        .enumerate()
                        .filter_map(|(idx, ctx)| {
                            ctx.label().map(|l| {
                                (
                                    l,
                                    u32::try_from(context_len - idx - 1)
                                        .expect("unexpectedly far backwards jump"),
                                )
                            })
                        })
                        .collect();
                    let cond_to_br: BTreeMap<_, _> = v
                        .iter()
                        .map(|&(cond, label)| {
                            let br_num = context_map.get(&label).unwrap();
                            (cond, *br_num)
                        })
                        .collect();
                    ReSeq(vec![TableJump(cond_to_br)])
                }
                Terminal => ReSeq(vec![Return]),
            };
            actions.cons(other)
        } else {
            let current_outer_node_id = outer_nodes.len() - 1;
            let current_outer_node = outer_nodes[current_outer_node_id];
            let mut new_ctx = context.clone();
            new_ctx.push(Context::BlockHeadedBy(current_outer_node));
            let inner = self.node_within(node, &outer_nodes[0..current_outer_node_id], &new_ctx);
            let merge_block = self.do_tree(current_outer_node, context);

            Block(inner).cons(merge_block)
        }
    }

    /// helper function for finding all the merge nodes depending on current node
    fn gen_node(&self, node: TLabel, context: &Vec<Context<TLabel>>) -> ReSeq<TLabel> {
        let merge_children: HashSet<TLabel> = self
            .children(node)
            .into_iter()
            .filter(|n| self.merge_nodes.contains(n))
            .collect();

        let context_labels: HashSet<_> = context.iter().filter_map(|ctx| ctx.label()).collect();
        let switch_children: Vec<TLabel> = if let Switch(v) = self.cfg.edge(&node) {
            v.iter()
                .map(|(_, l)| *l)
                .filter(|l| !context_labels.contains(l) && !merge_children.contains(l))
                .collect()
        } else {
            vec![]
        };

        let mut children = vec![];
        children.extend(merge_children);
        children.extend(switch_children);

        children.sort_by_key(|n| self.node_ordering.idx[n]);
        self.node_within(node, &children, context)
    }

    /// main function for node generating, handles loop nodes separately
    fn do_tree(&self, node: TLabel, context: &Vec<Context<TLabel>>) -> ReSeq<TLabel> {
        if self.loop_nodes.contains(&node) {
            let mut ctx = context.clone();
            ctx.push(Context::LoopHeadedBy(node));
            ReSeq::single(Loop(self.gen_node(node, &ctx)))
        } else {
            self.gen_node(node, context)
        }
    }

    pub fn reloop(&self) -> ReSeq<TLabel> {
        self.do_tree(self.cfg.entry, &Vec::new())
    }
}
