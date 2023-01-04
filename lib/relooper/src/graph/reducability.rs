use crate::graph::cfg::CfgEdge;
use crate::graph::cfg::CfgLabel;
use crate::Cfg;
use crate::EnrichedCfg;
use std::collections::{HashMap, HashSet};
pub type SuperNodeId = usize;

#[derive(Eq, Hash, PartialEq, Copy, Clone)]
/// (from, to)
pub enum ProperEdge {
    Uncond(CfgLabel, CfgLabel),
    Cond(CfgLabel, CfgLabel),
    Terminal,
}

#[derive(Default, Clone)]
struct Node {
    pub id: CfgLabel,
    pub succ: HashSet<ProperEdge>,
    pub prec: HashSet<ProperEdge>,
}

impl Node {
    pub fn new(id_: CfgLabel) -> Node {
        return Node {
            id: id_,
            succ: HashSet::default(),
            prec: HashSet::default(),
        };
    }
}

// TODO: set begin = 0 as default
#[derive(Default)]
pub struct Graph {
    id2node: HashMap<CfgLabel, Node>,
    next_id: CfgLabel,
    entry: CfgLabel,
    terminal: CfgLabel,
}

impl Graph {
    pub fn new() -> Self {
        Graph {
            id2node: HashMap::new(),
            next_id: 0,
            entry: 0,
            terminal: 0,
        }
    }

    pub fn build_from(&mut self, cfg: &Cfg) -> &Graph {
        self.entry = cfg.entry;

        for (label, edge) in &cfg.out_edges {
            if label >= &self.next_id {
                self.next_id = *label;
            }
            self.id2node.insert(*label, Node::new(*label));
            match edge {
                CfgEdge::Cond(to_cond, to_ucond) => {
                    self.id2node.insert(*to_cond, Node::new(*to_cond));
                    self.id2node.insert(*to_ucond, Node::new(*to_ucond));
                    self.id2node
                        .get_mut(&label)
                        .unwrap()
                        .succ
                        .insert(ProperEdge::Cond(*label, *to_cond));
                    self.id2node
                        .get_mut(&label)
                        .unwrap()
                        .succ
                        .insert(ProperEdge::Uncond(*label, *to_ucond));
                    self.id2node
                        .get_mut(&to_cond)
                        .unwrap()
                        .prec
                        .insert(ProperEdge::Cond(*label, *to_cond));
                    self.id2node
                        .get_mut(&to_ucond)
                        .unwrap()
                        .prec
                        .insert(ProperEdge::Uncond(*label, *to_ucond));
                }
                CfgEdge::Uncond(to_uncond) => {
                    self.id2node.insert(*to_uncond, Node::new(*to_uncond));
                    self.id2node
                        .get_mut(&label)
                        .unwrap()
                        .succ
                        .insert(ProperEdge::Uncond(*label, *to_uncond));
                    self.id2node
                        .get_mut(&to_uncond)
                        .unwrap()
                        .prec
                        .insert(ProperEdge::Uncond(*label, *to_uncond));
                }
                CfgEdge::Terminal => {
                    // self.id2node
                    //     .get_mut(&label)
                    //     .unwrap()
                    //     .succ
                    //     .insert(ProperEdge::Terminal);
                    self.terminal = *label;
                }
            }
        }
        return self;
    }

    pub fn cfg(&self) -> Cfg {
        let mut out_edges: HashMap<CfgLabel, CfgEdge> = HashMap::default();
        for (id, node) in &self.id2node {
            if node.succ.len() == 1 {
                match &node.succ.iter().next().unwrap() {
                    ProperEdge::Uncond(from, to) => {
                        out_edges.insert(*id, CfgEdge::Uncond(*to));
                    }
                    ProperEdge::Terminal => {
                        panic!("Here should not be a terminal node");
                        // out_edges.insert(*id, CfgEdge::Terminal);
                    }
                    ProperEdge::Cond(from, to) => {
                        panic!("Here should not be a cond node!");
                    }
                }
            } else if node.succ.len() > 1 {
                let mut cond_to: CfgLabel = usize::MAX;
                let mut ucond_to: CfgLabel = usize::MAX;
                for edge in &node.succ {
                    match edge {
                        ProperEdge::Cond(from, to) => {
                            cond_to = *to;
                        }
                        ProperEdge::Uncond(from, to) => {
                            ucond_to = *to;
                        }
                        ProperEdge::Terminal => {
                            panic!("Here should not be terminal")
                        }
                    }
                }
                if cond_to == usize::MAX || ucond_to == usize::MAX {
                    panic!("Unitialized variable!");
                }
                out_edges.insert(*id, CfgEdge::Cond(cond_to, ucond_to));
            } else {
                if node.id != self.terminal || true {
                    panic!("This node must be a terminal");
                }
                out_edges.insert(*id, CfgEdge::Terminal);
            }
        }
        out_edges.insert(self.terminal, CfgEdge::Terminal);
        return Cfg {
            out_edges,
            entry: self.entry,
        };
    }

    pub fn add_vertex(&mut self, id: CfgLabel) -> () {
        self.id2node.insert(id, Node::new(id));
        if self.next_id <= id {
            self.next_id = id + 1;
        }
    }

    pub fn add_edge(&mut self, edge: ProperEdge) -> () {
        match edge {
            ProperEdge::Uncond(from, to) => {
                self.id2node
                    .get_mut(&from)
                    .unwrap()
                    .succ
                    .insert(edge.clone());
                self.id2node.get_mut(&to).unwrap().prec.insert(edge.clone());
            }
            ProperEdge::Cond(from, to) => {
                self.id2node
                    .get_mut(&from)
                    .unwrap()
                    .succ
                    .insert(edge.clone());
                self.id2node.get_mut(&to).unwrap().prec.insert(edge.clone());
            }
            ProperEdge::Terminal => {}
        }
    }
}

pub fn reducable(g: &Cfg) -> Cfg {
    return Supergraph::default().build(&g).run().cfg();
}

#[derive(Default, Clone)]
struct SuperNode {
    super_id: SuperNodeId,
    cfg_ids: HashSet<CfgLabel>, // which cfg nodes in this supernode
    cfg_ids2cfg_nodes: HashMap<CfgLabel, Node>,
}

impl SuperNode {
    pub fn build(mut self, cfg_node: Node, id: SuperNodeId) -> SuperNode {
        self.super_id = id;
        self.cfg_ids.insert(cfg_node.id);
        self.cfg_ids2cfg_nodes.insert(cfg_node.id, cfg_node);
        return self;
    }
}

#[derive(Default)]
/// This SuperGraph is used as help struct to generate reducable CFG from irreducable one.
/// Main idea of algorithm:
/// 1) SuperGraph is graph where each node is group from one or more nodes  of CFG.
/// 2) Firstly, we make SuperGraph and each node of it contain exactly one CFG node.
/// 3) Then we do two operations -- merge and split until only one SuperGraph node left.
///
/// In process of this operations some CFG nodes will be cloned, and finally all this nodes will represent
/// equivalent reducable CFG.
struct Supergraph {
    id2node: HashMap<SuperNodeId, SuperNode>,
    clone2origin: HashMap<CfgLabel, CfgLabel>, // clone2origin[x] = y means that cfg node with id=x was clonned from cfg node with id=y;
    next_cfg_id: CfgLabel,
    entry: CfgLabel,
    terminal: CfgLabel,
}

impl Supergraph {
    pub fn build(mut self, c: &Cfg) -> Supergraph {
        let mut next_id = 0;
        let mut g = Graph::new();
        g.build_from(c);
        self.entry = g.entry;
        self.terminal = g.terminal;
        for (_id, cfg_node) in &g.id2node {
            let tmp = SuperNode::default();
            self.id2node
                .insert(next_id, tmp.build(cfg_node.clone(), next_id));
            next_id += 1;
        }
        self.next_cfg_id = g.next_id;

        println!("BUILD RESULTS:");
        for (id, node) in &self.id2node {
            println!("supernode {} with cfg node:", id);
            for idd in &node.cfg_ids {
                print!("{}, ", idd);
            }
            print!("\n");
        }

        return self;
    }

    pub fn run(mut self) -> Supergraph {
        loop {
            if self.can_merge() {
                self.merge(self.mergeble_nodes());
                continue;
            }
            if self.can_clone() {
                self.split(self.clonable_nodes());
                continue;
            }

            println!("RUN FINISHED");
            return self;
        }
    }

    pub fn in_which_supernode(&self, nid: CfgLabel) -> SuperNodeId {
        for (id, node) in &self.id2node {
            println!("Snode {} have cfg nodes:", id);
            for cfgid in &node.cfg_ids {
                print!("{}, ", cfgid);
            }
            print!("\n");
            if node.cfg_ids.contains(&nid) {
                return *id;
            }
        }
        panic!("No such node! NID = {}", nid);
    }

    pub fn can_merge(&self) -> bool {
        for (sid, snode) in &self.id2node {
            let mut super_precs: HashSet<SuperNodeId> = HashSet::default();
            for (_cfg_id, cfg_node) in &snode.cfg_ids2cfg_nodes {
                for prec_id in &cfg_node.prec {
                    match prec_id {
                        ProperEdge::Cond(from, to) => {
                            super_precs.insert(self.in_which_supernode(*from));
                        }
                        ProperEdge::Uncond(from, to) => {
                            super_precs.insert(self.in_which_supernode(*from));
                        }
                        ProperEdge::Terminal => {}
                    }
                }
            }
            if super_precs.contains(sid) {
                super_precs.remove(sid);
            }
            println!(
                "NODE {} HAVE {} PRECS, TRYING TO MERGE",
                sid,
                super_precs.len()
            );
            if super_precs.len() == 1 {
                return true;
            }
        }
        return false;
    }

    pub fn can_clone(&self) -> bool {
        for (sid, snode) in &self.id2node {
            let mut super_precs: HashSet<SuperNodeId> = HashSet::default();
            for (_cfg_id, cfg_node) in &snode.cfg_ids2cfg_nodes {
                for prec_id in &cfg_node.prec {
                    match prec_id {
                        ProperEdge::Cond(from, to) => {
                            super_precs.insert(self.in_which_supernode(*from));
                        }
                        ProperEdge::Uncond(from, to) => {
                            super_precs.insert(self.in_which_supernode(*from));
                        }
                        ProperEdge::Terminal => {}
                    }
                }
            }
            if super_precs.contains(sid) {
                super_precs.remove(sid);
            }
            println!(
                "NODE {} HAVE {} PRECS, TRYING TO SPLIT",
                sid,
                super_precs.len()
            );
            if super_precs.len() > 1 {
                return true;
            }
        }
        return false;
    }

    // returns two random mergeble nodes in format (master_id, slave_id)
    pub fn mergeble_nodes(&self) -> (SuperNodeId, SuperNodeId) {
        for (sid, snode) in &self.id2node {
            let mut super_precs: HashSet<SuperNodeId> = HashSet::default();
            for (_cfg_id, cfg_node) in &snode.cfg_ids2cfg_nodes {
                for prec_id in &cfg_node.prec {
                    match prec_id {
                        ProperEdge::Cond(from, to) => {
                            super_precs.insert(self.in_which_supernode(*from));
                        }
                        ProperEdge::Uncond(from, to) => {
                            super_precs.insert(self.in_which_supernode(*from));
                        }
                        ProperEdge::Terminal => {}
                    }
                }
            }
            if super_precs.contains(sid) {
                super_precs.remove(sid);
            }
            if super_precs.len() == 1 {
                return (*super_precs.iter().next().unwrap(), *sid);
            }
        }

        panic!("no mergable nodes");
    }

    // returns clonable node with all its precessors in format (masters_ids, slave_id)
    pub fn clonable_nodes(&self) -> (HashSet<SuperNodeId>, SuperNodeId) {
        for (sid, snode) in &self.id2node {
            let mut super_precs: HashSet<SuperNodeId> = HashSet::default();
            for (_cfg_id, cfg_node) in &snode.cfg_ids2cfg_nodes {
                for prec_id in &cfg_node.prec {
                    match prec_id {
                        ProperEdge::Cond(from, to) => {
                            super_precs.insert(self.in_which_supernode(*from));
                        }
                        ProperEdge::Uncond(from, to) => {
                            super_precs.insert(self.in_which_supernode(*from));
                        }
                        ProperEdge::Terminal => {
                            panic!("Here should not be a terminal edge!");
                        }
                    }
                }
            }
            if super_precs.contains(sid) {
                super_precs.remove(sid);
            }
            if super_precs.len() > 1 {
                return (super_precs, *sid);
            }
        }
        panic!("no clonable nodes");
    }

    pub fn merge(&mut self, (master, slave): (SuperNodeId, SuperNodeId)) -> () {
        println!("MERGE nodes slave : {}, master : {}", slave, master);
        self.make_clone((master, slave));
        self.delete_snode(slave);
        println!("REMAIN SIDS AFTER MERGE {} TO {}:", slave, master);
        for (id, node) in &self.id2node {
            print!("{}, ", id);
        }
        print!("\n");
    }

    pub fn split(&mut self, (masters, slave): (HashSet<SuperNodeId>, SuperNodeId)) -> () {
        println!("split nodes slave : {}, masters:", slave);
        for id in &masters {
            println!("{},", id)
        }
        for master in &masters {
            self.make_clone((*master, slave));
        }
        println!("SPLIT WITH SLAVE = {}; MASTERS:", slave);
        for master in &masters {
            println!("{}, ", master);
        }
        println!("\n");
        self.delete_snode(slave);
        println!("REMAIN SIDS AFTER SPLIT OF {}:", slave);
        for (id, node) in &self.id2node {
            print!("{}, ", id);
        }
        print!("\n");
    }

    /// This method deletes supernode from supergraph with all its inedges and outedges
    pub fn delete_snode(&mut self, node_id: SuperNodeId) -> () {
        // here we don't delete all links to prev node!!!
        println!("SNODE {} DELETING", node_id);
        let mut edges_to_del: HashSet<(CfgLabel, CfgLabel, ProperEdge)> = HashSet::default();
        for (id, cfg_node) in &self.id2node.get_mut(&node_id).unwrap().cfg_ids2cfg_nodes {
            for edge in &cfg_node.prec {
                match edge {
                    ProperEdge::Cond(from, to) => {
                        edges_to_del.insert((*from, *to, edge.clone()));
                    }
                    ProperEdge::Uncond(from, to) => {
                        edges_to_del.insert((*from, *to, edge.clone()));
                    }
                    ProperEdge::Terminal => {
                        panic!("Here should not be a terminal edge!");
                    }
                }
            }

            for edge in &cfg_node.succ {
                match edge {
                    ProperEdge::Cond(from, to) => {
                        edges_to_del.insert((*from, *to, edge.clone()));
                    }
                    ProperEdge::Uncond(from, to) => {
                        edges_to_del.insert((*from, *to, edge.clone()));
                    }
                    ProperEdge::Terminal => {
                        panic!("Here should not be a terminal edge!");
                    }
                }
            }
        }
        for (from, to, edge) in edges_to_del {
            &self.delete_edge(from, to, edge);
        }
        self.id2node.remove(&node_id);
    }

    /// This function node by node copy slave cfg nodes to master supernode
    pub fn make_clone(&mut self, (master, slave): (SuperNodeId, SuperNodeId)) -> () {
        let mut new_cfg_ids: HashMap<CfgLabel, CfgLabel> = HashMap::default(); // old -> new
        for id in &self.id2node.get(&slave).unwrap().cfg_ids {
            new_cfg_ids.insert(*id, self.next_cfg_id);
            self.clone2origin.insert(self.next_cfg_id, *id);
            self.next_cfg_id += 1;
        }

        for (old, new) in &new_cfg_ids {
            let mut node = self
                .id2node
                .get(&slave)
                .unwrap()
                .cfg_ids2cfg_nodes
                .get(&old)
                .unwrap()
                .clone();
            self.copy_to_other_supernode(node, slave, master, *new);
        }
    }

    pub fn cfg(self) -> Cfg {
        let mut id2n: HashMap<CfgLabel, Node> = HashMap::default();
        for (_sid, snode) in self.id2node {
            for (cid, cnode) in snode.cfg_ids2cfg_nodes {
                id2n.insert(cid, cnode);
            }
        }
        Graph {
            id2node: (id2n),
            next_id: (self.next_cfg_id),
            entry: self.entry,
            terminal: self.terminal,
        }
        .cfg()
    }

    pub fn delete_edge(&mut self, from: CfgLabel, to: CfgLabel, edge: ProperEdge) -> () {
        self.id2node
            .get_mut(&self.in_which_supernode(from))
            .unwrap()
            .cfg_ids2cfg_nodes
            .get_mut(&from)
            .unwrap()
            .succ
            .remove(&edge);

        self.id2node
            .get_mut(&self.in_which_supernode(to))
            .unwrap()
            .cfg_ids2cfg_nodes
            .get_mut(&to)
            .unwrap()
            .prec
            .remove(&edge);
    }

    /// This method copyes one node (cfg node) from snode_from to snode_to (super nodes)
    /// This method copyes this cfg node with only edges, that have origin in from node or in to node
    /// Also it add this edges into precessors edges lists (!)
    pub fn copy_to_other_supernode(
        &mut self,
        node: Node,
        snode_from_id: SuperNodeId,
        snode_to_id: SuperNodeId,
        new_id: CfgLabel,
    ) -> () {
        let snode_from = self.id2node.get(&snode_from_id).unwrap().clone();
        let mut snode_to = self.id2node.get(&snode_to_id).unwrap().clone();

        let mut cfg_node = Node::new(new_id);

        for succ in node.succ {
            match succ {
                ProperEdge::Cond(from, to) => {
                    cfg_node.succ.insert(ProperEdge::Cond(new_id, to));
                    self.id2node
                        .get_mut(&self.in_which_supernode(to))
                        .unwrap()
                        .cfg_ids2cfg_nodes
                        .get_mut(&to)
                        .unwrap()
                        .prec
                        .insert(ProperEdge::Cond(new_id, to));
                }
                ProperEdge::Uncond(from, to) => {
                    cfg_node.succ.insert(ProperEdge::Uncond(new_id, to));
                    self.id2node
                        .get_mut(&self.in_which_supernode(to))
                        .unwrap()
                        .cfg_ids2cfg_nodes
                        .get_mut(&to)
                        .unwrap()
                        .prec
                        .insert(ProperEdge::Uncond(new_id, to));
                }
                ProperEdge::Terminal => {
                    panic!("Here should not be a terminal edge!");
                }
            }
        }
        for prec in node.prec {
            match prec {
                ProperEdge::Cond(from, to) => {
                    if self.in_which_supernode(from) != snode_from_id
                        && self.in_which_supernode(from) != snode_to_id
                    {
                        continue;
                    }
                    cfg_node.prec.insert(ProperEdge::Cond(from, new_id));
                    self.id2node
                        .get_mut(&self.in_which_supernode(from))
                        .unwrap()
                        .cfg_ids2cfg_nodes
                        .get_mut(&from)
                        .unwrap()
                        .succ
                        .insert(ProperEdge::Cond(from, new_id));
                }
                ProperEdge::Uncond(from, to) => {
                    if self.in_which_supernode(from) != snode_from_id
                        && self.in_which_supernode(from) != snode_to_id
                    {
                        continue;
                    }
                    cfg_node.prec.insert(ProperEdge::Uncond(from, new_id));
                    self.id2node
                        .get_mut(&self.in_which_supernode(from))
                        .unwrap()
                        .cfg_ids2cfg_nodes
                        .get_mut(&from)
                        .unwrap()
                        .succ
                        .insert(ProperEdge::Uncond(from, new_id));
                }
                ProperEdge::Terminal => {
                    panic!("Here should not be a terminal edge!");
                }
            }
        }

        // prev bug fixed but now there is some issues with links...

        println!(
            "pre_copy_to_other_supernode; snode_to id = {}; cfg ids in it:",
            snode_to_id
        );
        for id in &self.id2node.get(&snode_to_id).unwrap().cfg_ids {
            print!("{}, ", id);
        }
        print!("\n");
        println!("new id = {}", new_id);

        snode_to.cfg_ids.insert(new_id);
        snode_to.cfg_ids2cfg_nodes.insert(new_id, cfg_node.clone());

        self.id2node.insert(snode_to_id, snode_to);

        println!(
            "copy_to_other_supernode; snode_to id = {}; cfg ids in it:",
            snode_to_id
        );
        for id in &self.id2node.get(&snode_to_id).unwrap().cfg_ids {
            print!("{}, ", id);
        }
        print!("\n");
    }

    pub fn full_log(&self) {}
}

#[test]
pub fn test_reducer() -> () {
    println!("test reducer");
    let graph = Cfg::from_edges(
        vec![
            (0, CfgEdge::Cond(1, 2)),
            (1, CfgEdge::Cond(3, 5)),
            (2, CfgEdge::Uncond(3)),
            (3, CfgEdge::Uncond(4)),
            (5, CfgEdge::Cond(6, 7)),
            (6, CfgEdge::Uncond(8)),
            (7, CfgEdge::Uncond(8)),
            (4, CfgEdge::Uncond(9)),
            (8, CfgEdge::Cond(9, 5)),
        ],
        0,
    );
    let e_graph = Cfg::from(graph.unwrap());
    let reducable = reducable(&e_graph);

    println!("Graph after reducing:\n");
    for (node, edge) in &reducable.out_edges {
        match edge {
            CfgEdge::Cond(cond, ucond) => {
                println!(
                    "Node {} has cond edge to {} and uncond to {}",
                    node, cond, ucond
                );
            }
            CfgEdge::Uncond(to) => {
                println!("Node {} has uncond edge to {}", node, to);
            }
            CfgEdge::Terminal => {
                println!("Node {} has terminal edge", node);
            }
        }
    }

    let enriched = EnrichedCfg::new(reducable);
    let enriched_irr = EnrichedCfg::new(e_graph);
    let dot_lines: Vec<String> = vec![
        "digraph {".to_string(),
        enriched_irr.cfg_to_dot(),
        String::new(),
        enriched.dom_to_dot(),
        String::new(),
        enriched.cfg_to_dot(),
        "}".to_string(),
    ];
    std::fs::write("reduced.dot", dot_lines.join("\n")).expect("fs error");
}
