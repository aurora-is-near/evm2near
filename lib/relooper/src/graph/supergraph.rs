/// simple but working algorithm for graph reduction
/// produces too much copied nodes for fairly simple examples, which complicates every later stage
/// was replaced by more efficient algorithm (`relooper::graph::reduction`) for that reasons
///
/// The algorithm: firstly, let's define a supernode.
/// A supernode is a group of nodes of the initial CFG.
/// Initially, we put each CFG node in a separate supernode.
/// Then, we perform two operations -- merge and split.
/// Merge: If all in-edges of all CFG nodes of some supernode A have the origin in supernode A and/or in exactly one other supernode B,
/// we can merge these supernodes -- just assign that now all CFG nodes of supernode A are in supernode B.
/// Split: Now, let's say we have a supernode A and a set of supernodes {B0, B1, ... Bn}
/// such that all in-edges of CFG nodes in supernode A have the origin in supernode A or in one of supernodes Bi.
/// Then, we can perform a split -- duplicate node A n times, now we have supernodes {A0, A1, ... An} with the same code inside.
/// And for each of these supernodes, we will cut all in-edges that are not from Ai or from Bi.
/// For example, for node A3, we will cut all in-edges that are not from A3 or B3. Then we will perform n merges (Ai with Bi).
/// We perform these operations until there will be exactly one supernode.
/// After it, we just return the graph contained in this supernode.
/// In each step, there is a variety of operations we can do -- we can do some splits and some merges, but we need to choose one.
/// These choices affect the execution time and, what is more important, the size of the resulting CFG.
/// But, we didn't find the best way to make these decisions, and currently, we use a greedy strategy.
/// Some words about correctness: it is easy to see that if we have more than one supernode and CFG is connected, we can perform merge or split.
/// Also, both merge and split reduce the number of supernodes by one, so after `size(CFG)` iterations, the algorithm will be finished.
/// The proof that each irreducible loop will be broken by a split is quite big, and we left it for the reader.
use super::reduction::SLabel;
use super::{GEdgeColl, GEdgeCollMappable, Graph, GraphMut};
use crate::graph::cfg::{Cfg, CfgLabel};
use crate::graph::supergraph::NodeAction::{MergeInto, SplitFor};
use crate::traversal::graph::dfs::PrePostOrder;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt::Debug;

#[derive(Clone, PartialEq, Eq, Ord, PartialOrd, Debug)]
struct SNode<TLabel: CfgLabel> {
    head: SLabel<TLabel>,
    contained: BTreeSet<SLabel<TLabel>>,
}

impl<TLabel: CfgLabel> From<SLabel<TLabel>> for SNode<TLabel> {
    fn from(label: SLabel<TLabel>) -> Self {
        Self {
            head: label,
            contained: BTreeSet::from([label]),
        }
    }
}

impl<TLabel: CfgLabel> SNode<TLabel> {
    fn new(head: SLabel<TLabel>, contained: BTreeSet<SLabel<TLabel>>) -> Self {
        Self { head, contained }
    }
}

type SNodeLabel<TLabel> = SLabel<TLabel>;

pub struct SuperGraph<TLabel: CfgLabel> {
    cfg: Cfg<SLabel<TLabel>>,
    versions: BTreeMap<TLabel, usize>,
    nodes: BTreeMap<SNodeLabel<TLabel>, SNode<TLabel>>,
    label_location: BTreeMap<SLabel<TLabel>, SNodeLabel<TLabel>>,
}

type SplitInto<TLabel> = Vec<SLabel<TLabel>>;

#[derive(Debug)]
enum NodeAction<TLabel: CfgLabel> {
    MergeInto(SLabel<TLabel>),
    SplitFor(SplitInto<TLabel>),
}

impl<TLabel: CfgLabel> SuperGraph<TLabel> {
    pub(crate) fn new(cfg: &Cfg<TLabel>) -> Self {
        let cfg = cfg.map_label(|&l| SLabel::from(l));

        let nodes: BTreeMap<SLabel<TLabel>, SNode<TLabel>> =
            cfg.nodes().iter().map(|&&l| (l, SNode::from(l))).collect();

        let label_location: BTreeMap<SLabel<TLabel>, SLabel<TLabel>> =
            nodes.iter().map(|(&l, _n)| (l, l)).collect();

        let versions: BTreeMap<TLabel, usize> =
            label_location.iter().map(|(&l, _)| (l.origin, 0)).collect();

        Self {
            cfg,
            versions,
            nodes,
            label_location,
        }
    }

    /// We are using that order for traversing supernodes graph for choosing between merge/split actions
    fn snode_order(&self) -> Vec<&SLabel<TLabel>> {
        let start = self.nodes.get(&self.cfg.entry).unwrap();
        let mut postorder: Vec<_> = PrePostOrder::start_from(&start.head, |slabel| {
            let snode = self.nodes.get(slabel).unwrap();
            snode.contained.iter().flat_map(|l| {
                self.cfg
                    .children(l)
                    .into_iter()
                    .map(|to| self.label_location.get(to).unwrap())
            })
        })
        .postorder()
        .collect();
        postorder.reverse();
        postorder
    }

    /// finding out applicable action for given supernode
    fn node_action(
        &self,
        node: &SNode<TLabel>,
        cfg_in_edges: &HashMap<SLabel<TLabel>, HashSet<SLabel<TLabel>>>,
    ) -> Option<NodeAction<TLabel>> {
        let mut incoming: BTreeSet<&SLabel<TLabel>> = cfg_in_edges // getting all nodes that point to current node's head
            .get(&node.head)
            .into_iter()
            .flatten()
            .map(|l| self.label_location.get(l).unwrap())
            .map(|snode_label| &self.nodes.get(snode_label).unwrap().head)
            .collect();

        incoming.remove(&node.head); // if given node have internal edges ending in its head, it will be seen as incoming supernode

        match incoming.len() {
            0 => None,
            1 => Some(MergeInto(**incoming.first().unwrap())),
            _ => Some(SplitFor(incoming.into_iter().copied().collect())),
        }
    }

    /// merging `from_label` into `to_label`, entirely removing `from` supernode from the supergraph
    fn merge(&mut self, from_label: SLabel<TLabel>, to_label: SLabel<TLabel>) {
        let from = self.nodes.remove(&from_label).unwrap();
        let to = self.nodes.get_mut(&to_label).unwrap();
        to.contained.extend(from.contained.iter());
        for inner in from.contained {
            assert_eq!(self.label_location.insert(inner, to.head), Some(from.head));
        }
    }

    /// splitting given supernode for each of `split` nodes
    /// duplicates every node residing that supernode
    fn split(&mut self, node_label: SLabel<TLabel>, split: &SplitInto<TLabel>) {
        let split_snode = self.nodes.get(&node_label).unwrap().to_owned();

        let outgoing_edges: HashMap<_, _> = split_snode
            .contained
            .iter()
            .copied()
            .map(|inner| (inner, self.cfg.edge(&inner).clone()))
            .collect();

        //duplicate every label in that supernode (for each split except the first one, bc original version can be reused)
        for split_for_l in &split[1..] {
            let split_for = self.nodes.get(split_for_l).unwrap();
            let mut versions_mapping: HashMap<SLabel<TLabel>, SLabel<TLabel>> = Default::default();
            for &inner in split_snode.contained.iter() {
                self.versions.entry(inner.origin).and_modify(|version| {
                    *version += 1;
                    let a = *version;
                    let new_ver = SLabel::new(inner.origin, a);
                    versions_mapping.insert(inner, new_ver);
                });
            }

            // copy internal & outgoing edges
            for (o_from, edge) in &outgoing_edges {
                let curr_from = versions_mapping[o_from];
                // in case of internal edge, we should redirect that edge to new copy of internal node
                let maybe_redirected_edge = edge.map(|l| *versions_mapping.get(l).unwrap_or(l));
                self.cfg.add_edge(curr_from, maybe_redirected_edge);
            }

            for &f in &split_for.contained {
                let e = self.cfg.edge_mut(&f);
                if e.iter().any(|&to| to == split_snode.head) {
                    e.apply(|to| *versions_mapping.get(to).unwrap_or(to))
                }
            }

            // populate supernode graph with new node's new version (for each split)
            let splitted_head = versions_mapping.get(&split_snode.head).unwrap().to_owned();
            let contained: Vec<_> = versions_mapping.values().copied().collect();
            self.nodes.insert(
                splitted_head,
                SNode::new(
                    splitted_head,
                    BTreeSet::from_iter(contained.iter().copied()),
                ),
            );
            let node_ref = self.nodes.get(&splitted_head).unwrap();
            for c in contained {
                self.label_location.insert(c, node_ref.head);
            }
        }
    }

    /// iterates over supergraph (using `snode_order` on each iteration) until there is only one node left
    /// on each iteration we either:
    /// * remove one supernode (by merging it into another one)
    /// * or duplicate one (by splitting, one dup for each "parent")
    /// in the end, there is only one supernode, which contains all the nodes and whose head is "entry" node
    fn reduce(&mut self) {
        let mut in_edges: Option<HashMap<SLabel<TLabel>, HashSet<SLabel<TLabel>>>> = None;
        'outer: loop {
            let order: Vec<&SLabel<TLabel>> = self.snode_order();

            let mut splits: HashMap<SLabel<TLabel>, SplitInto<TLabel>> = Default::default();

            for snode_label in order {
                let n = self.nodes.get(snode_label).unwrap();
                let in_edges = in_edges.get_or_insert_with(|| {
                    let mut hm: HashMap<SLabel<TLabel>, HashSet<SLabel<TLabel>>> =  // ugly copying to avoid mutual borrowing
                        Default::default();
                    for (a, b) in self.cfg.in_edges().into_iter() {
                        hm.insert(*a, b.into_iter().copied().collect());
                    }
                    hm
                }); // recalculated after every node splitting
                match self.node_action(n, in_edges) {
                    None => {}
                    Some(SplitFor(split)) => {
                        splits.insert(n.head, split);
                    }
                    Some(MergeInto(to)) => {
                        self.merge(n.head, to);
                        continue 'outer;
                    }
                }
            }

            let mut split_len: BTreeMap<usize, Vec<&SLabel<TLabel>>> = BTreeMap::new();

            for (split_node, split_for) in splits.iter() {
                split_len
                    .entry(split_for.len())
                    .or_default()
                    .push(split_node);
            }

            if let Some((_, biggest_splits)) = split_len.last_key_value() {
                // TODO select by internal node count?
                let split_node = biggest_splits.first().unwrap();
                let split_for = splits.get(split_node).unwrap();

                self.split(**split_node, split_for);
                in_edges = None;

                continue;
            } else {
                break;
            }
        }
        assert_eq!(self.nodes.len(), 1);
    }
}

fn check_reduction<TLabel: CfgLabel>(
    origin_cfg: &Cfg<TLabel>,
    reduced_cfg: &Cfg<SLabel<TLabel>>,
) -> bool {
    let reduced_nodes = reduced_cfg.nodes();
    let mut origin_mapping: HashMap<TLabel, HashSet<SLabel<TLabel>>> = Default::default();
    for &x in reduced_nodes.iter() {
        origin_mapping.entry(x.origin).or_default().insert(*x);
    }

    origin_cfg.edges().iter().all(|(from, e)| {
        origin_mapping
            .get(from)
            .unwrap()
            .iter()
            .all(|&r_from| &reduced_cfg.edge(&r_from).map(|x| x.origin) == e)
    })
}

#[deprecated]
pub fn reduce<TLabel: CfgLabel>(cfg: &Cfg<TLabel>) -> Cfg<SLabel<TLabel>> {
    let mut super_graph = SuperGraph::new(cfg);
    super_graph.reduce();
    assert!(check_reduction(cfg, &super_graph.cfg));
    super_graph.cfg
}

#[cfg(test)]
#[allow(deprecated)]
mod test {
    use crate::graph::cfg::Cfg;
    use crate::graph::cfg::CfgEdge::{Cond, Terminal, Uncond};
    use crate::graph::supergraph::{check_reduction, reduce};

    #[test]
    fn simplest() {
        let cfg = Cfg::from_edges(
            0,
            vec![(0, Cond(1, 2)), (1, Uncond(2)), (2, Cond(3, 1))]
                .into_iter()
                .collect(),
        );
        let reduced = reduce(&cfg);

        assert!(check_reduction(&cfg, &reduced));
    }

    #[test]
    fn irreducible() {
        let cfg = Cfg::from_edges(
            0,
            vec![
                (0, Cond(1, 2)),
                (1, Uncond(4)),
                (4, Uncond(2)),
                (2, Cond(3, 1)),
            ]
            .into_iter()
            .collect(),
        );
        let reduced = reduce(&cfg);

        assert!(check_reduction(&cfg, &reduced));
    }

    #[test]
    fn moderate() {
        let cfg = Cfg::from_edges(
            0,
            vec![
                (0, Cond(1, 2)),
                (1, Cond(3, 4)),
                (2, Cond(3, 5)),
                (3, Uncond(4)),
                (4, Cond(2, 5)),
            ]
            .into_iter()
            .collect(),
        );
        let reduced = reduce(&cfg);

        assert!(check_reduction(&cfg, &reduced));
    }

    #[test]
    fn new() {
        let cfg = Cfg::from_edges(
            0,
            vec![
                (0, Cond(1, 3)),
                (1, Uncond(2)),
                (2, Cond(5, 1)),
                (3, Uncond(4)),
                (4, Cond(5, 3)),
                (5, Cond(6, 7)),
                (6, Terminal),
                (7, Cond(1, 3)),
            ]
            .into_iter()
            .collect(),
        );
        let reduced = reduce(&cfg);

        std::fs::write(
            "irr_new_cfg.dot",
            format!("digraph {{{}}}", cfg.cfg_to_dot("irr_new_cfg")),
        )
        .expect("fs error");

        std::fs::write(
            "irr_new_cfg_reduced.dot",
            format!("digraph {{{}}}", reduced.cfg_to_dot("irr_new_cfg_reduced")),
        )
        .expect("fs error");

        assert!(check_reduction(&cfg, &reduced));
    }
}
