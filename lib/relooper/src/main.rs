mod traversal;

use crate::traversal::{BfsGraph, BfsTree, DfsGraph, DfsTree, Traverse};
use crate::ReBlockType::{Block, If, Loop};
use dot::{Edges, Id, LabelText, Nodes, Style};
use std::borrow::{Borrow, Cow};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{Read};
use std::iter::{once, Map, Once};

type CfgLabel = isize;
pub struct Cfg(BTreeMap<CfgLabel, Vec<CfgLabel>>);

impl From<Vec<(CfgLabel, CfgLabel)>> for Cfg {
    fn from(edges: Vec<(CfgLabel, CfgLabel)>) -> Self {
        let mut m: BTreeMap<CfgLabel, Vec<CfgLabel>> = BTreeMap::new();
        for (from, to) in edges {
            m.entry(from).or_default().push(to);
        }
        Cfg(m)
    }
}

impl Cfg {
    fn nodes(&self) -> HashSet<CfgLabel> {
        let m = &self
            .0
            .iter()
            .flat_map(|(&from, to)| to.iter().copied().chain(std::iter::once(from)))
            .collect::<HashSet<_>>();
        m.to_owned()
    }

    fn children(&self, label: CfgLabel) -> HashSet<CfgLabel> {
        self.0.get(&label).into_iter().flatten().copied().collect()
    }
}

impl<'a> dot::Labeller<'a, CfgLabel, (CfgLabel, CfgLabel)> for Cfg {
    fn graph_id(&'a self) -> Id<'a> {
        Id::new("cfg").unwrap()
    }

    fn node_id(&'a self, n: &CfgLabel) -> Id<'a> {
        Id::new(format!("n{}", n)).unwrap()
    }
}

impl<'a> dot::GraphWalk<'a, CfgLabel, (CfgLabel, CfgLabel)> for Cfg {
    fn nodes(&'a self) -> Nodes<'a, CfgLabel> {
        let nodes = self.nodes();
        let v = nodes.into_iter().collect::<Vec<CfgLabel>>();
        Cow::Owned(v)
    }

    fn edges(&'a self) -> Edges<'a, (CfgLabel, CfgLabel)> {
        let x: Vec<(CfgLabel, CfgLabel)> = self
            .0
            .clone()
            .into_iter()
            .flat_map(|(from, to)| to.into_iter().map(move |t| (from, t)))
            .collect(); //TODO
        Cow::Owned(x)
    }

    fn source(&'a self, (from, _to): &(CfgLabel, CfgLabel)) -> CfgLabel {
        *from
    }

    fn target(&'a self, (_from, to): &(CfgLabel, CfgLabel)) -> CfgLabel {
        *to
    }
}

type ReLabel = isize;

#[derive(Debug, Clone, Copy)]
enum ReBlockType {
    Block,
    Loop,
    If,
}

#[derive(Debug, Clone, Copy)]
struct ReBlock {
    block_type: ReBlockType,
    curr: ReLabel,
    //TODO change to branch?
    inner: ReLabel,
    next: ReLabel,
}

pub struct ReGraph(BTreeMap<ReLabel, ReBlock>);

impl ReBlock {
    fn new(typ: ReBlockType, curr: ReLabel, inner: ReLabel, next: ReLabel) -> ReBlock {
        ReBlock {
            block_type: typ,
            curr,
            inner,
            next,
        }
    }

    fn block(curr: ReLabel, next: ReLabel) -> ReBlock {
        ReBlock::new(Block, curr, curr, next)
    }

    fn looop(curr: ReLabel, next: ReLabel) -> ReBlock {
        ReBlock::new(Loop, curr, curr, next)
    }

    fn iff(curr: ReLabel, tru: ReLabel, fal: ReLabel) -> ReBlock {
        ReBlock::new(If, curr, tru, fal)
    }
}

impl<'a> dot::Labeller<'a, ReBlock, (ReLabel, ReLabel)> for ReGraph {
    fn graph_id(&'a self) -> Id<'a> {
        Id::new("relooped").unwrap()
    }

    fn node_id(&'a self, b: &ReBlock) -> Id<'a> {
        Id::new(format!("{:?}{:?}", b.block_type, b.curr)).unwrap()
    }
}

impl<'a> dot::GraphWalk<'a, ReBlock, (ReLabel, ReLabel)> for ReGraph {
    fn nodes(&'a self) -> Nodes<'a, ReBlock> {
        let v: Vec<ReBlock> = self.0.iter().map(|(_l, block)| *block).collect();
        Cow::Owned(v)
    }

    fn edges(&'a self) -> Edges<'a, (ReLabel, ReLabel)> {
        Cow::Owned(
            self.0
                .clone()
                .into_values()
                .flat_map(|b| match b.block_type {
                    If => vec![(b.curr, b.inner), (b.curr, b.next)],
                    _ => vec![(b.curr, b.next)],
                })
                .collect(),
        ) //TODO
    }

    fn source(&'a self, (from, _to): &(ReLabel, ReLabel)) -> ReBlock {
        *self.0.get(from).unwrap()
    }

    fn target(&'a self, (_from, to): &(ReLabel, ReLabel)) -> ReBlock {
        *self.0.get(to).unwrap()
    }
}

pub fn dummy() -> ReGraph {
    let mut m = BTreeMap::new();
    m.insert(0, ReBlock::iff(0, 1, 2));
    m.insert(1, ReBlock::block(1, 3));
    m.insert(2, ReBlock::block(2, 3));
    m.insert(3, ReBlock::block(3, 4));
    m.insert(4, ReBlock::looop(4, 0));

    ReGraph(m)
}

fn create_block(
    cfg: &Cfg,
    reachability: HashMap<CfgLabel, Vec<CfgLabel>>,
    entries: Vec<CfgLabel>,
    remaining: HashSet<CfgLabel>,
) -> (ReLabel, ReBlock) {
    match entries[..] {
        [single]
            if !reachability
                .get(&single)
                .expect("entry must be part of a graph!")
                .contains(&single) =>
        {
            let next_entries: Vec<_> = cfg
                .children(single)
                .intersection(&remaining)
                .copied()
                .collect();
            let next_remaining = remaining
                .difference(&once(single).collect())
                .copied()
                .collect();
            let (next_label, next_block) =
                create_block(cfg, reachability, next_entries, next_remaining);
            (single, ReBlock::block(single, next_label))
        }
        [..] if entries.iter().all(|e| {
            reachability
                .get(e)
                .expect("entry must be part of a graph!")
                .contains(e)
        }) =>
        {
            todo!()
        }
        _ => todo!(),
    }
}

pub fn reloop(cfg: &Cfg, entry: CfgLabel) -> ReGraph {
    let nodes = cfg.nodes();

    let reachability: HashMap<CfgLabel, Vec<CfgLabel>> = nodes
        .into_iter()
        .map(|l| {
            (
                l,
                BfsGraph::entries(cfg.children(l).iter())
                    .traverse(|node| cfg.0.get(node).into_iter().flatten())
                    .into_iter()
                    .copied()
                    .collect(),
            )
        })
        .collect();
    println!("n{:?}", reachability);

    let reachable_from_start: HashSet<CfgLabel> = reachability
        .get(&entry)
        .expect("entry block must be part of CFG")
        .into_iter()
        .copied()
        .collect();

    let _block = create_block(cfg, reachability, vec![entry], reachable_from_start);

    dummy()
}

pub fn main() {
    use std::fs::File;

    let graph = Cfg::from(vec![
        (0, 1),
        (0, 2),
        (1, 3),
        (2, 3),
        (3, 4),
        (1, 5),
        (5, 6),
        (5, 7),
        (6, 8),
        (7, 8),
        (4, 9),
        (8, 9),
        (8, 5),
    ]);
    // let graph = Cfg::from(vec![(0, 1), (0, 2), (1, 3), (1, 4), (1, 5), (2, 6), (6, 7)]);

    let mut f_cfg = File::create("cfg.dot").unwrap();
    dot::render(&graph, &mut f_cfg).unwrap();

    let re_graph = reloop(&graph, 0);

    let mut f_relooped = File::create("relooped.dot").unwrap();
    dot::render(&re_graph, &mut f_relooped).unwrap();

    // let start = 0 as CfgLabel;
    // let res_b = BfsGraph::start_from(&start).traverse(|x| graph.0.get(x).into_iter().flatten());
    // println!("Bfs:{:?}", res_b);
    // let res_d = DfsGraph::start_from(&start).traverse(|x| graph.0.get(x).into_iter().flatten());
    // println!("Dfs:{:?}", res_d);
}
