use crate::graph::cfg::{Cfg, CfgLabel};
use crate::graph::supergraph::NodeAction::{MergeInto, SplitFor};
use crate::traversal::graph::dfs::{DfsPost, DfsPostReverseInstantiator};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt::{Debug, Display, Formatter};

type SVersion = usize;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub struct SLabel<TLabel: CfgLabel> {
    pub origin: TLabel,
    version: SVersion,
}

impl<TLabel: CfgLabel + Display> Display for SLabel<TLabel> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}_{}", self.origin, self.version)
    }
}

impl<TLabel: CfgLabel> Debug for SLabel<TLabel> {
    // why debug isnt automatically derived from display?
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}_{}", self.origin, self.version)
    }
}

impl<TLabel: CfgLabel> From<TLabel> for SLabel<TLabel> {
    fn from(origin: TLabel) -> Self {
        Self { origin, version: 0 }
    }
}

impl<TLabel: CfgLabel> SLabel<TLabel> {
    fn new(origin: TLabel, version: SVersion) -> Self {
        Self { origin, version }
    }
}

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
        let new_cfg = cfg.map_label(|&l| SLabel::from(l));

        let nodes: BTreeMap<SLabel<TLabel>, SNode<TLabel>> = new_cfg
            .nodes()
            .iter()
            .copied()
            .map(|&l| (l, SNode::from(l)))
            .collect();

        let label_location: BTreeMap<SLabel<TLabel>, SLabel<TLabel>> =
            nodes.iter().map(|(&l, _n)| (l, l)).collect();

        let versions: BTreeMap<TLabel, usize> =
            label_location.iter().map(|(&l, _)| (l.origin, 0)).collect();

        Self {
            cfg: new_cfg,
            versions,
            nodes,
            label_location,
        }
    }

    /// We are using that order for traversing supernodes graph for choosing between merge/split actions
    fn snode_order(&self) -> Vec<SLabel<TLabel>> {
        let start = self.nodes.get(&self.cfg.entry).unwrap().clone();
        DfsPost::<_, _, HashSet<_>>::reverse(start.head, |slabel| {
            let snode = self.nodes.get(slabel).unwrap();
            snode.contained.iter().flat_map(|l| {
                self.cfg
                    .children(l)
                    .into_iter()
                    .map(|to| *self.label_location.get(to).unwrap())
            })
        })
    }

    /// finding out applicable action for given supernode
    fn node_action(
        &self,
        node: &SNode<TLabel>,
        cfg_in_edges: &HashMap<SLabel<TLabel>, HashSet<SLabel<TLabel>>>,
    ) -> Option<NodeAction<TLabel>> {
        let mut incoming: BTreeSet<&SNode<TLabel>> = cfg_in_edges
            .get(&node.head)
            .into_iter()
            .flatten()
            .map(|l| self.label_location.get(l).unwrap())
            .map(|snode_label| self.nodes.get(snode_label).unwrap())
            .collect();
        // if given node have internal edges ending in its head, it will be seen as incoming supernode, which isn't useful
        incoming.remove(node);

        match incoming.len() {
            0 => None,
            1 => Some(MergeInto(incoming.first().unwrap().head)),
            _ => Some(SplitFor(incoming.into_iter().map(|s| s.head).collect())),
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
            let order: Vec<SLabel<TLabel>> = self.snode_order();

            let mut splits: HashMap<SLabel<TLabel>, SplitInto<TLabel>> = Default::default();

            for snode_label in order {
                let n = self.nodes.get(&snode_label).unwrap();
                let in_edges = in_edges.get_or_insert_with(|| self.cfg.in_edges());
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

pub fn reduce<TLabel: CfgLabel>(cfg: &Cfg<TLabel>) -> Cfg<SLabel<TLabel>> {
    let mut super_graph = SuperGraph::new(cfg);
    super_graph.reduce();
    super_graph.cfg
}

#[cfg(test)]
mod test {
    use crate::graph::cfg::CfgEdge::{Cond, Uncond};
    use crate::graph::cfg::{Cfg, CfgLabel};
    use crate::graph::supergraph::{reduce, SLabel};
    use std::collections::{HashMap, HashSet};

    fn test_reduce<TLabel: CfgLabel>(
        origin_cfg: Cfg<TLabel>,
        reduced_cfg: Cfg<SLabel<TLabel>>,
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

    #[test]
    fn simplest() {
        let cfg = Cfg::from_edges(
            0,
            vec![(0, Cond(1, 2)), (1, Uncond(2)), (2, Cond(3, 1))]
                .into_iter()
                .collect(),
        );
        let reduced = reduce(&cfg);

        assert!(test_reduce(cfg, reduced));
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

        assert!(test_reduce(cfg, reduced));
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

        assert!(test_reduce(cfg, reduced));
    }
}
