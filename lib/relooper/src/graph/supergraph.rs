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

impl<TLabel: CfgLabel + Display> Display for SLabel<TLabel> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}_{}", self.origin, self.version)
    }
}

impl<TLabel: CfgLabel + Debug> Debug for SLabel<TLabel> {
    // why debug isnt automatically derived from display?
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}_{}", self.origin, self.version)
    }
}

impl<TLabel: CfgLabel> CfgLabel for SLabel<TLabel> {}

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

pub struct SuperGraph<TLabel: CfgLabel + Debug> {
    cfg: Cfg<SLabel<TLabel>>,
    labels: BTreeMap<TLabel, Vec<SLabel<TLabel>>>,
    nodes: BTreeMap<SNodeLabel<TLabel>, SNode<TLabel>>,
    label_location: BTreeMap<SLabel<TLabel>, SNodeLabel<TLabel>>,
}

type SplitInto<TLabel> = Vec<SLabel<TLabel>>;

#[derive(Debug)]
enum NodeAction<TLabel: CfgLabel> {
    MergeInto(SLabel<TLabel>),
    SplitFor(SplitInto<TLabel>),
}

impl<TLabel: CfgLabel> CfgEdge<TLabel> {
    fn redirect<F>(&self, redirection: F) -> CfgEdge<TLabel>
    where
        F: Fn(TLabel) -> TLabel,
    {
        match self {
            Uncond(to) => Uncond(redirection(*to)),
            Cond(t, f) => Cond(redirection(*t), redirection(*f)),
            Terminal => Terminal,
        }
    }
}

impl<TLabel: CfgLabel + Debug> SuperGraph<TLabel> {
    pub(crate) fn new(cfg: &Cfg<TLabel>) -> Self {
        let new_cfg = cfg.map_label(|&l| SLabel::from(l));

        let nodes: BTreeMap<SLabel<TLabel>, SNode<TLabel>> = new_cfg
            .nodes()
            .iter()
            .map(|&l| (l, SNode::from(l)))
            .collect();

        let label_location: BTreeMap<SLabel<TLabel>, SLabel<TLabel>> =
            nodes.iter().map(|(&l, _n)| (l, l)).collect();

        let labels: BTreeMap<TLabel, Vec<SLabel<TLabel>>> = label_location
            .iter()
            .map(|(&l, _)| (l.origin, vec![l]))
            .collect();

        Self {
            cfg: new_cfg,
            labels,
            nodes,
            label_location,
        }
    }

    fn snode_order(&self) -> Vec<SLabel<TLabel>> {
        let start = self.nodes.get(&self.cfg.entry).unwrap().clone();
        let res: Vec<_> = dfs_post_ord(start.head, &mut |slabel| {
            let snode = self.nodes.get(slabel).unwrap();
            snode.contained.iter().flat_map(|&l| {
                self.cfg
                    .children(l)
                    .into_iter()
                    .map(|to| *self.label_location.get(&to).unwrap())
            })
        })
        .into_iter()
        .collect();
        res
    }

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
        incoming.remove(node);
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
        let split_snode = self.nodes.get(&node_label).unwrap().to_owned();
        println!(
            "label: {:?}, node: {:?}, nodes: {:?}",
            node_label, split_snode, self.nodes
        );

        let outgoing_edges: HashMap<_, _> = split_snode
            .contained
            .iter()
            .copied()
            .map(|inner| (inner, *self.cfg.edge(inner)))
            .collect();

        //duplicate every label in that supernode
        for split_for_l in &split[1..] {
            let split_for = self.nodes.get(split_for_l).unwrap();
            let mut versions_mapping: HashMap<SLabel<TLabel>, SLabel<TLabel>> = Default::default();
            for &inner in split_snode.contained.iter() {
                self.labels.entry(inner.origin).and_modify(|versions| {
                    let new_ver = SLabel::new(inner.origin, versions.len());
                    versions_mapping.insert(inner, new_ver);
                    versions.push(new_ver)
                });
            }

            // copy internal & outgoing edges
            for (o_from, &edge) in outgoing_edges.iter() {
                let curr_from = versions_mapping[o_from];
                self.cfg.add_edge(curr_from, edge);
            }

            let from_split: HashMap<_, _> = split_for
                .contained
                .iter()
                // .copied()
                .map(|&l| (l, *self.cfg.edge(l)))
                .filter(|(_l, e)| e.to_vec().iter().any(|to| *to == split_snode.head))
                .collect();

            for (f, e) in from_split {
                let redirected = e.redirect(|to| match versions_mapping.get(&to) {
                    Some(redirected_to) => *redirected_to,
                    None => to,
                });
                self.cfg.remove_edge(f, e);
                self.cfg.add_edge(f, redirected);
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
    }

    pub fn reduce(&mut self) {
        let mut in_edges: Option<HashMap<SLabel<TLabel>, HashSet<SLabel<TLabel>>>> = None;
        'outer: loop {
            let order: Vec<SLabel<TLabel>> = self.snode_order();

            println!("cfg: {:?}", self.cfg);
            println!("nod: {:?}", self.nodes.values());
            println!("ord: {:?}", order);

            let mut splits: Vec<(SLabel<TLabel>, SplitInto<TLabel>)> = Vec::new();
            // TODO switch to maps and flattens to get rid of `Option`?
            for snode_label in order {
                let n = self.nodes.get(&snode_label).unwrap();
                let in_edges = in_edges.get_or_insert_with(|| self.cfg.in_edges());
                match self.node_action(n, in_edges) {
                    None => {}
                    Some(SplitFor(split)) => splits.push((n.head, split)),
                    Some(MergeInto(to)) => {
                        println!("merged {:?} to {:?}", n.head, to);
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

                println!("split {:?} for {:?}", n, split);
                self.split(*n, split);
                in_edges = None;

                continue;
            } else {
                break;
            }
        }
        assert_eq!(self.nodes.len(), 1);
    }
}

pub fn reduce<TLabel: CfgLabel + Debug>(cfg: &Cfg<TLabel>) -> Cfg<SLabel<TLabel>> {
    let mut super_graph = SuperGraph::new(cfg);
    super_graph.reduce();
    super_graph.cfg
}
