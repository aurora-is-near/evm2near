use crate::graph::cfg::CfgEdge::{Cond, Terminal, Uncond};
use crate::graph::cfg::{Cfg, CfgEdge, CfgLabel};
use crate::graph::supergraph::NodeAction::{MergeInto, SplitFor};
use crate::traversal::graph::dfs::dfs_post_ord;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt::{Debug, Display, Formatter};

type SVersion = usize;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub struct SLabel<TLabel: CfgLabel> {
    origin: TLabel,
    version: SVersion,
}

impl<TLabel: CfgLabel> Display for SLabel<TLabel> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}_{}", self.origin, self.version)
    }
}

impl<TLabel: CfgLabel> Debug for SLabel<TLabel> {
    // why debug isnt automatically derived from display?
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}_{}", self.origin, self.version)
    }
}

impl<TLabel: CfgLabel> CfgLabel for SLabel<TLabel> {}

impl<TLabel: CfgLabel> From<TLabel> for SLabel<TLabel> {
    fn from(origin: TLabel) -> Self {
        Self {
            origin, //todo rename to origin
            version: 0,
        }
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

pub struct SuperGraph<TLabel: CfgLabel> {
    entry: SLabel<TLabel>,
    labels: BTreeMap<TLabel, Vec<SLabel<TLabel>>>,
    nodes: BTreeMap<SLabel<TLabel>, SNode<TLabel>>,
    in_edges: HashMap<SLabel<TLabel>, HashSet<SLabel<TLabel>>>,
    pub(crate) out_edges: HashMap<SLabel<TLabel>, HashSet<SLabel<TLabel>>>,
    label_location: BTreeMap<SLabel<TLabel>, SLabel<TLabel>>,
}

type SplitInto<TLabel> = Vec<SLabel<TLabel>>;

enum NodeAction<TLabel: CfgLabel> {
    MergeInto(SLabel<TLabel>),
    SplitFor(SplitInto<TLabel>),
}

impl<TLabel: CfgLabel> SuperGraph<TLabel> {
    pub(crate) fn new(cfg: &Cfg<TLabel>) -> Self {
        let nodes: BTreeMap<SLabel<TLabel>, SNode<TLabel>> = cfg
            .nodes()
            .iter()
            .map(|&n| {
                let label = SLabel::from(n);
                (label, SNode::from(label))
            })
            .collect();

        let label_location: BTreeMap<SLabel<TLabel>, SLabel<TLabel>> =
            nodes.iter().map(|(&l, _n)| (l, l)).collect();

        let labels: BTreeMap<TLabel, Vec<SLabel<TLabel>>> = label_location
            .iter()
            .map(|(&l, _)| (l.origin, vec![l]))
            .collect();

        let out_edges = cfg
            .out_edges
            .iter()
            .map(|(&f, v_t)| {
                (
                    SLabel::from(f),
                    v_t.to_vec().iter().map(|&l| SLabel::from(l)).collect(),
                )
            })
            .collect();

        let in_edges = cfg
            .in_edges()
            .iter()
            .map(|(&t, v_f)| {
                (
                    SLabel::from(t),
                    v_f.iter().map(|&l| SLabel::from(l)).collect(),
                )
            })
            .collect();

        Self {
            entry: cfg.entry.into(),
            labels,
            nodes,
            out_edges,
            in_edges,
            label_location,
        }
    }

    fn add_edge(&mut self, from: SLabel<TLabel>, to: SLabel<TLabel>) {
        self.out_edges.entry(from).or_default().insert(to);
        self.in_edges.entry(to).or_default().insert(from);
    }

    fn remove_edge(&mut self, from: SLabel<TLabel>, to: SLabel<TLabel>) {
        self.out_edges
            .entry(from)
            .and_modify(|set| assert!(set.remove(&to)));
        self.in_edges
            .entry(to)
            .and_modify(|set| assert!(set.remove(&from)));
    }

    fn remove_node(&mut self, node: SLabel<TLabel>) {
        let o_back = self
            .out_edges
            .get(&node)
            .into_iter()
            .flatten()
            .copied()
            .collect::<Vec<_>>();
        let i_back = self
            .in_edges
            .get(&node)
            .into_iter()
            .flatten()
            .copied()
            .collect::<Vec<_>>();
        for o in o_back {
            self.in_edges
                .entry(o)
                .and_modify(|set| assert!(set.remove(&node)));
        }
        for i in i_back {
            self.out_edges
                .entry(i)
                .and_modify(|set| assert!(set.remove(&node)));
        }

        self.out_edges.remove(&node);
        self.in_edges.remove(&node);
    }

    fn outgoing_edges(&self, node: &SNode<TLabel>) -> BTreeSet<&SNode<TLabel>> {
        node.contained
            .iter()
            .flat_map(|inner| {
                self.out_edges.get(inner).into_iter().flatten().map(|l| {
                    self.label_location
                        .get(l)
                        .and_then(|snode_label| self.nodes.get(snode_label))
                        .unwrap()
                })
            })
            .filter(|x| *x != node)
            .collect()
    }

    // TODO unify with `outgoing_edges`?
    fn incoming_edges(&self, node: &SNode<TLabel>) -> BTreeSet<&SNode<TLabel>> {
        node.contained
            .iter()
            .flat_map(|inner| {
                self.in_edges.get(inner).into_iter().flatten().map(|l| {
                    self.label_location
                        .get(l)
                        .and_then(|snode_label| self.nodes.get(snode_label))
                        .unwrap()
                })
            })
            .filter(|x| *x != node)
            .collect()
    }

    fn edges_between(
        &self,
        snode_from: &SNode<TLabel>,
        snode_to: &SNode<TLabel>,
    ) -> BTreeMap<SLabel<TLabel>, SLabel<TLabel>> {
        snode_from
            .contained
            .iter()
            .filter_map(|&inner| {
                self.out_edges.get(&inner).and_then(|to_set| {
                    let to: Vec<SLabel<TLabel>> = to_set
                        .into_iter()
                        .filter(|points_to| snode_to.contained.contains(points_to))
                        .copied()
                        .collect();

                    assert!(to.len() <= 1); // there can be no edges or only one edge to head of given `snode_to`
                    if to.len() == 1 {
                        let &single = to.get(0).unwrap();
                        Some((inner, single))
                    } else {
                        None
                    }
                })
            })
            .collect()
    }

    fn snode_order(&self) -> Vec<SLabel<TLabel>> {
        let start = self.nodes.get(&self.entry).unwrap().clone();
        let res: Vec<_> = dfs_post_ord(start.head, &mut |slabel| {
            let snode = self.nodes.get(slabel).unwrap();
            self.outgoing_edges(snode)
                .iter()
                .map(|n| n.head)
                .collect::<Vec<_>>()
        })
        .into_iter()
        .collect();
        res
    }

    fn node_action(&self, node: &SNode<TLabel>) -> Option<NodeAction<TLabel>> {
        let incoming = self.incoming_edges(node);
        // TODO hate there is no pattern-match adapters for simple collections, or is there?
        match incoming.len() {
            0 => None,
            1 => Some(MergeInto(incoming.first().unwrap().head)),
            _ => Some(SplitFor(incoming.into_iter().map(|s| s.head).collect())),
        }
    }

    fn merge(&mut self, from_label: SLabel<TLabel>, to_label: SLabel<TLabel>) {
        let from = self.nodes.remove(&from_label).unwrap();
        let to = self.nodes.get_mut(&to_label).unwrap();
        to.contained.extend(from.contained.iter());
        for inner in from.contained {
            assert_eq!(self.label_location.insert(inner, to.head), Some(from.head));
        }
    }

    fn split(&mut self, node_label: SLabel<TLabel>, split: &SplitInto<TLabel>) {
        let split_snode = self.nodes.remove(&node_label).unwrap();

        let internal_edges = self.edges_between(&split_snode, &split_snode);
        let outgoing_edges: Vec<_> = split_snode
            .contained
            .iter()
            .flat_map(|inner| {
                self.out_edges
                    .get(inner)
                    .into_iter()
                    .flatten()
                    .filter(|&to| {
                        self.label_location.get(to).unwrap().to_owned().to_owned() != node_label
                    })
                    .map(|&to| (*inner, to))
                    .collect::<Vec<_>>()
            })
            .collect();

        //duplicate every label in that supernode
        for split_for_l in split {
            let split_for = self.nodes.get(split_for_l).unwrap();
            let mut versions_mapping: HashMap<SLabel<TLabel>, SLabel<TLabel>> = Default::default();
            for &inner in split_snode.contained.iter() {
                self.labels.entry(inner.origin).and_modify(|versions| {
                    let new_ver = SLabel::new(inner.origin, versions.len());
                    assert!(versions_mapping.insert(inner, new_ver).is_none());
                    versions.push(new_ver)
                });
            }

            // split incoming edges
            for (s_from, i_to) in self.edges_between(split_for, &split_snode) {
                let n_to = versions_mapping.get(&i_to).unwrap().to_owned();
                self.remove_edge(s_from, i_to);
                self.add_edge(s_from, n_to);
            }

            // copy internal edges
            for (i_from, i_to) in internal_edges.iter() {
                let new_from = versions_mapping.get(i_from).unwrap().to_owned();
                let new_to = versions_mapping.get(i_to).unwrap().to_owned();
                self.add_edge(new_from, new_to);
            }

            // copy outgoing edges
            for &(i_from, o_to) in outgoing_edges.iter() {
                let new_from = versions_mapping.get(&i_from).unwrap().to_owned();
                self.add_edge(new_from, o_to);
            }

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

        for (i_from, i_to) in internal_edges {
            self.remove_node(i_from);
            self.remove_node(i_to);
        }
        for (i_from, o_to) in outgoing_edges {
            self.remove_node(i_from);
        }
    }

    pub fn reduce(&mut self) {
        'outer: loop {
            let order: Vec<SLabel<TLabel>> = self.snode_order();

            let mut splits: Vec<(SLabel<TLabel>, SplitInto<TLabel>)> = Vec::new();
            // TODO switch to maps and flattens to get rid of `Option`?
            for snode_label in order {
                let n = self.nodes.get(&snode_label).unwrap();
                match self.node_action(&n) {
                    None => {}
                    Some(SplitFor(split)) => splits.push((n.head, split)),
                    Some(MergeInto(to)) => {
                        self.merge(n.head, to);
                        continue 'outer;
                    }
                }
            }

            let mut split_len: BTreeMap<usize, Vec<(SLabel<TLabel>, SplitInto<TLabel>)>> =
                BTreeMap::new();

            for n_split in splits {
                split_len.entry(n_split.1.len()).or_default().push(n_split);
            }

            if let Some((_, biggest_splits)) = split_len.last_key_value() {
                let (n, split) = biggest_splits.first().unwrap(); // TODO select by internal node count?

                self.split(*n, split);

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
    let super_out_edges = super_graph.out_edges;
    let out_edges: Vec<(SLabel<TLabel>, CfgEdge<SLabel<TLabel>>)> = super_out_edges
        .into_iter()
        .filter_map(|(slabel, points_to)| {
            let edge_opt = match cfg.out_edges.get(&slabel.origin).unwrap().to_owned() {
                Uncond(label) => {
                    assert_eq!(points_to.len(), 1);
                    let to = points_to.into_iter().next().unwrap();
                    assert_eq!(to.origin, label);
                    Some(Uncond(to))
                }
                Cond(t_l, f_l) => {
                    assert_eq!(points_to.len(), 2);
                    let (first, second) = {
                        let mut iter = points_to.into_iter();
                        (iter.next().unwrap(), iter.next().unwrap())
                    };

                    if (t_l, f_l) == (first.origin, second.origin) {
                        Some(Cond(first, second))
                    } else if (t_l, f_l) == (second.origin, first.origin) {
                        Some(Cond(second, first))
                    } else {
                        panic!("wtf") //todo
                    }
                }
                Terminal => {
                    assert_eq!(points_to.len(), 0);
                    None
                }
            };
            edge_opt.map(|edge| (slabel, edge))
        })
        .collect();

    Cfg::from_edges(out_edges, super_graph.entry).unwrap()
}
