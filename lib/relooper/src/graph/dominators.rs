use super::cfg::CfgLabel;
use crate::graph::cfg::Cfg;
use std::cmp::min;
use std::collections::{HashMap, HashSet};



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

}

struct LinkEval <TLabel: CfgLabel> {
    // below a LINK-EVAL structures
    le_parent: HashMap<TLabel, TLabel>,
    roots: HashSet<TLabel>,
    le_label: HashMap<TLabel, TLabel>,
    le_size: HashMap<TLabel, usize>,
    le_child: HashMap<TLabel, TLabel>,
}


impl<TLabel: CfgLabel> LinkEval<TLabel> {
    fn compress(&mut self, v: TLabel, semi: &HashMap<TLabel, usize>, entry: TLabel) {
        // println!("compress, v:{:?}", v);

        if self.le_parent.get(&self.le_parent[&v]).is_some() {
            if self.le_parent[&self.le_parent[&v]] == entry {
                return;
            }
            self.compress(self.le_parent[&v], semi, entry);
            if semi[&self.le_label[&self.le_parent[&v]]] < semi[&self.le_label[&v]] {
                self.le_label.insert(v, self.le_label[&self.le_parent[&v]]);
            }
            self.le_label.insert(v, self.le_parent[&self.le_parent[&v]]);
        }
    }
    
    fn eval(&mut self, v: TLabel, semi: &HashMap<TLabel, usize>, entry: TLabel) -> TLabel {
        // println!("eval, v:{:?}", v);
        if self.le_parent.get(&v).is_none() {
            self.le_label[&v]
        } else {
            self.compress(v, semi, entry);
            if semi[&self.le_label[&self.le_parent[&v]]] >= semi[&self.le_label[&v]] {
                return self.le_label[&v];
            } else {
                return self.le_label[&self.le_parent[&v]];
            }
        }
    }

    fn link(&mut self, v: TLabel, w: TLabel, semi: &HashMap<TLabel, usize>, entry: TLabel) {
        // println!("link, v:{:?}, w{:?}", v, w);
        let mut s = w;
        while semi[&self.le_label[&w]] < semi[&self.le_label[&self.le_child[&s]]] {
            if self.le_size[&s] + self.le_size[&self.le_child[&self.le_child[&s]]] >= 2 * self.le_size[&self.le_child[&s]] {
                self.le_parent.insert(self.le_child[&s], s);
                self.le_child.insert(s, self.le_child[&self.le_child[&s]]);
            } else {
                self.le_size.insert(self.le_child[&s], self.le_size[&s]);
                self.le_parent.insert(s, self.le_child[&s]);
                s = self.le_child[&s];
            }
        }
        self.le_label.insert(s, self.le_label[&w]);
        self.le_size.insert(v, self.le_size[&v] + self.le_size[&w]);
        if self.le_size[&v] < 2 * self.le_size[&w] {
            let temp = s;
            s = self.le_child[&v];
            self.le_child.insert(v, temp);
        }
        loop {
            // println!("link loop, s = {:?}", s);
            self.le_parent.insert(s, v);
            match self.le_child.get(&s) {
                Some(ss) => {
                    if *ss == entry {
                        break;
                    }
                    s = *ss;
                },
                None => {break;}
            }
        }
    }
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

        DominationBuilder {
            cfg: cfg.clone(),
            preds,
            preorder_dfs,
            parent,
            dfs2label,
            label2dfs,
            semi,
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

    pub fn build(&mut self) -> HashMap<TLabel, TLabel> {

        let mut le_parent: HashMap<TLabel, TLabel> = Default::default();
        let mut roots: HashSet<TLabel> = Default::default();

        let mut le_label: HashMap<TLabel, TLabel> = Default::default();
        let mut le_size: HashMap<TLabel, usize> = Default::default();
        let mut le_child: HashMap<TLabel, TLabel> = Default::default();

        for node in self.cfg.nodes() {
            roots.insert(*node);
            le_child.insert(*node, self.cfg.entry);
            if *node == self.cfg.entry {
                le_size.insert(*node, 0);
            } else {
                le_size.insert(*node, 1);
            }
            le_label.insert(*node, *node);
        }

        let mut le_forest = LinkEval{le_parent, roots, le_label, le_size, le_child};



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
                .map(|x| *self.label2dfs.get(&le_forest.eval(*x, &self.semi, self.cfg.entry)).unwrap())
                .min()
                .unwrap();
            self.semi.insert(*node, sdom);

            // After this semi[node] is semidominator of node

            // add ``node`` to bucket(w) where w is node which number in dfs is semi(node)
            let key = self.dfs2label.get(&self.semi.get(node).unwrap()).unwrap();
            bucket.entry(*key).or_default().push(*node);

            // LINK(parent(node), node)
            le_forest.link(self.parent(*node), *node, &self.semi, self.cfg.entry);

            // TODO: try to rename v and u to smth more verbose

            // for each v in bucket(parent(node)):
            //    1) delete v from bucket(parent(w));
            //    2) dom(v) = EVAL(v) if semi(EVAL(v)) < semi(v), else parent(node)
            // println!("node: {:#?}\nentry: {:#?}", *node, self.cfg.entry);
            while !bucket.get(&self.parent(*node)).unwrap().is_empty() {
                let v = bucket.get_mut(&self.parent(*node)).unwrap().pop().unwrap();
                let u = le_forest.eval(v, &self.semi, self.cfg.entry);
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
