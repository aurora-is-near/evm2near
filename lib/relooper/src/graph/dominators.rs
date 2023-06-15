use super::cfg::CfgLabel;
use crate::graph::cfg::Cfg;
use crate::graph::elu::{ELUForest, TwoArgLambdaTrait};
use std::cmp::min;
use std::collections::{HashMap, HashSet};

pub struct SdomMin<'bebra, TLabel> {
    semi: &'bebra HashMap<TLabel, usize>,
}

impl<TLabel: CfgLabel> TwoArgLambdaTrait<TLabel> for SdomMin<'_, TLabel> {
    fn apply(&self, arg1: TLabel, arg2: TLabel) -> TLabel {
        if *self.semi.get(&arg1).unwrap() < *self.semi.get(&arg2).unwrap() {
            return arg1;
        }
        arg2
    }
}

impl<TLabel: CfgLabel> SdomMin<'_, TLabel> {
    pub fn new(semi: &HashMap<TLabel, usize>) -> SdomMin<TLabel> {
        SdomMin { semi }
    }
}

pub fn domination_tree<TLabel: CfgLabel>(
    cfg: &Cfg<TLabel>,
    begin: TLabel,
) -> HashMap<TLabel, TLabel> {
    DominationBuilder::new(cfg).build()
}

struct DominationBuilder<TLabel: CfgLabel> {
    cfg: Cfg<TLabel>,
    preds: HashMap<TLabel, HashSet<TLabel>>,
    preorder_dfs: Vec<TLabel>,
    parent: HashMap<TLabel, TLabel>,
    dfs2label: HashMap<usize, TLabel>,
    label2dfs: HashMap<TLabel, usize>,
    semi: HashMap<TLabel, usize>, // map from node to number of its sdom in dfs

    // below a LINK-EVAL structures
    le_parent: HashMap<TLabel, TLabel>,
    roots: HashSet<TLabel>,
}

impl<TLabel: CfgLabel> DominationBuilder<TLabel> {
    pub fn new(cfg: &Cfg<TLabel>) -> DominationBuilder<TLabel> {
        let mut preds: HashMap<TLabel, HashSet<TLabel>> = Default::default();

        for node in cfg.nodes() {
            for child in cfg.children(node) {
                preds.entry(*child).or_default().insert(*node);
            }
        }

        let mut preorder_dfs: Vec<TLabel> = Default::default();
        let mut visited: HashSet<TLabel> = Default::default();
        let mut parent: HashMap<TLabel, TLabel> = Default::default();

        // TODO: it will we better if we will add preorder dfs in traversal and use it here
        Self::dfs_entry(&mut preorder_dfs, &mut visited, cfg, cfg.entry, &mut parent);

        let mut dfs2label: HashMap<usize, TLabel> = Default::default();
        let mut label2dfs: HashMap<TLabel, usize> = Default::default();

        for (dfs_index, label) in preorder_dfs.iter().enumerate() {
            dfs2label.insert(dfs_index, *label);
            label2dfs.insert(*label, dfs_index);
        }

        let semi: HashMap<TLabel, usize> = Default::default();
        let le_parent: HashMap<TLabel, TLabel> = Default::default();
        let mut roots: HashSet<TLabel> = Default::default();

        for node in cfg.nodes() {
            roots.insert(*node);
        }

        DominationBuilder {
            cfg: cfg.clone(),
            preds,
            preorder_dfs,
            parent,
            dfs2label,
            label2dfs,
            semi,
            le_parent,
            roots,
        }
    }

    pub fn dfs_entry(
        dfs_vec: &mut Vec<TLabel>,
        visited: &mut HashSet<TLabel>,
        cfg: &Cfg<TLabel>,
        node: TLabel,
        parent: &mut HashMap<TLabel, TLabel>,
    ) {
        visited.insert(node);
        dfs_vec.push(node);
        for child in cfg.children(&node) {
            if !visited.contains(child) {
                parent.insert(child.clone(), node);
                Self::dfs_entry(dfs_vec, visited, cfg, *child, parent);
            }
        }
    }

    pub fn succ(&self, node: TLabel) -> HashSet<TLabel> {
        self.cfg.children(&node).into_iter().map(|x| *x).collect()
    }

    pub fn pred(&self, node: TLabel) -> HashSet<TLabel> {
        self.preds.get(&node).unwrap().clone()
    }

    pub fn parent(&self, node: TLabel) -> TLabel {
        *self.parent.get(&node).unwrap()
    }

    // TODO: maybe it will make sence to move this eval link to other file and struct. but borrow checker leaded me here  (=
    pub fn eval(&self, node: TLabel) -> TLabel {
        let mut cur_node = node;
        let mut res = *self.semi.get(&node).unwrap();
        while !self.roots.contains(&cur_node) {
            res = min(res, *self.semi.get(&cur_node).unwrap());
            cur_node = *self.le_parent.get(&cur_node).unwrap();
        }
        *self.dfs2label.get(&res).unwrap()
    }

    pub fn link(&mut self, v: TLabel, w: TLabel) -> () {
        if !self.roots.contains(&v) || !self.roots.contains(&w) {
            panic!("Both nodes must be roots for link");
        }
        self.roots.remove(&w);
        self.le_parent.insert(w, v);
    }

    pub fn build(&mut self) -> HashMap<TLabel, TLabel> {
        let mut bucket: HashMap<TLabel, Vec<TLabel>> = Default::default(); // bucket(w) is set of vertixes which sdom is w
        let mut dom: HashMap<TLabel, TLabel> = Default::default(); // map from node to its immediate dominator

        // bucket initialization. just empty set for each node.
        for node in &self.preorder_dfs {
            bucket.insert(*node, Default::default());
        }

        // semi initializing. while sdom is not computed semi(w) contains number of w in dfs
        for (idx, node) in self.preorder_dfs.iter().enumerate() {
            self.semi.insert(*node, idx);
        }

        // TODO: get rid of this .clone() cosed by fighting with borrow checker
        for node in self.preorder_dfs.clone().iter().rev() {
            if *node == self.cfg.entry {
                continue;
            }

            // sdom[node] = min(semi(EVAL(v))), for all v such that there is an edge from v to node in cfg
            let sdom = self
                .pred(*node)
                .iter()
                .map(|x| *self.label2dfs.get(&self.eval(*x)).unwrap())
                .min()
                .unwrap();
            self.semi.insert(*node, sdom);

            // After this semi[node] is semidominator of node

            // add ``node`` to bucket(w) where w is node which number in dfs is semi(node)
            let key = self.dfs2label.get(&self.semi.get(node).unwrap()).unwrap();
            bucket.entry(*key).or_default().push(*node);

            // LINK(parent(node), node)
            self.link(self.parent(*node), *node);

            // TODO: try to rename v and u to smth more verbose

            // for each v in bucket(parent(node)):
            //    1) delete v from bucket(parent(w));
            //    2) dom(v) = EVAL(v) if semi(EVAL(v)) < semi(v), else parent(node)
            println!("node: {:#?}\nentry: {:#?}", *node, self.cfg.entry);
            while !bucket.get(&self.parent(*node)).unwrap().is_empty() {
                let v = bucket.get_mut(&self.parent(*node)).unwrap().pop().unwrap();
                let u = self.eval(v);
                if self.semi.get(&u) < self.semi.get(&v) {
                    dom.insert(v, u);
                } else {
                    dom.insert(v, self.parent(*node));
                }
            }
        }

        // Go throught nodes in increasing order. Let w is current node. Than:
        // if dom(w) != vertex(semi(w)) then dom(w) := dom(dom(w)) fi od;
        // dom of root is root
        for node in &self.preorder_dfs {
            if *node == self.cfg.entry {
                dom.insert(*node, *node);
                continue;
            }
            if dom.get(&node).unwrap()
                != self.dfs2label.get(&self.semi.get(&node).unwrap()).unwrap()
            {
                dom.insert(*node, *dom.get(dom.get(&node).unwrap()).unwrap());
            }
        }
        dom
    }
}
