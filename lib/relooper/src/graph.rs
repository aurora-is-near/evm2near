use queues::IsQueue;
use queues::Queue;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::vec::Vec;

pub type NodeId = isize;

#[derive(Default, Clone)]
struct Node {
    pub id: NodeId,
    pub succ: HashSet<NodeId>,
    pub prec: HashSet<NodeId>,
}

impl Node {
    pub fn new(id_: NodeId) -> Node {
        return Node {
            id: id_,
            succ: HashSet::default(),
            prec: HashSet::default(),
        };
    }
}

#[derive(Clone)]
pub struct Block {
    pub data: u32,
}

impl Block {
    pub fn new() -> Self {
        Block { data: 1 }
    }
    pub fn copy(&self) -> Block {
        return Block { data: self.data };
    }
}

// TODO: set begin = 0 as default
#[derive(Default)]
pub struct Graph {
    id2block: HashMap<NodeId, Block>,
    id2node: HashMap<NodeId, Node>,
    next_id: NodeId,
    merge_nodes: HashSet<NodeId>,
    loop_nodes: HashSet<NodeId>,
    if_nodes: HashSet<NodeId>,
}

impl Graph {
    pub fn new() -> Self {
        Graph {
            id2block: HashMap::new(),
            id2node: HashMap::new(),
            next_id: 0,
            merge_nodes: HashSet::new(),
            loop_nodes: HashSet::new(),
            if_nodes: HashSet::new(),
        }
    }

    pub fn add_vertex(&mut self, block: Block) -> () {
        self.id2block.insert(self.next_id, block);
        self.id2node.insert(self.next_id, Node::new(self.next_id));
        self.next_id += 1;
        // println!("{}", self.next_id);
    }

    pub fn add_edge(&mut self, from: NodeId, to: NodeId) -> () {
        self.id2node.get_mut(&from).unwrap().succ.insert(to);
        self.id2node.get_mut(&to).unwrap().prec.insert(from);
    }
    // TODO cache result
    pub fn reverse_postorder(&self, begin: NodeId) -> Vec<NodeId> {
        let mut res = Vec::<NodeId>::default();
        let mut visited = HashSet::<NodeId>::default();
        self.dfs(begin, &mut res, &mut visited);
        res.reverse();
        return res;
    }

    fn dfs(
        &self,
        current_node: NodeId,
        res: &mut Vec<NodeId>,
        visited: &mut HashSet<NodeId>,
    ) -> () {
        for id in &self.id2node.get(&current_node).unwrap().succ {
            if !visited.contains(&id) {
                visited.insert(*id);
                self.dfs(*id, res, visited);
            }
        }
        res.push(current_node);
    }

    pub fn print_keys(&self) -> () {
        println!("id2block");
        for k in self.id2block.keys() {
            println!("{}", *k);
        }
    }

    pub fn domination_tree(&self, begin: NodeId) -> HashMap<NodeId, NodeId> /* map points from node id to id of its dominator */
    {
        let mut result = HashMap::<NodeId, NodeId>::new();
        let mut bfs = Queue::<NodeId>::new();
        let mut visited = HashSet::<NodeId>::new();
        let nodes = self.reverse_postorder(begin);
        for n in nodes {
            result.insert(n, begin);
        }
        bfs.add(begin).unwrap(); // should be next. upd: i dont think so
        visited.insert(begin);
        loop {
            if bfs.size() == 0 {
                break;
            }
            let cur_id = bfs.peek().unwrap();
            visited.insert(cur_id);
            bfs.remove().unwrap();
            self.update_dominators(cur_id, begin, &mut result);
            for id in &self.id2node.get(&cur_id).unwrap().succ {
                if !visited.contains(id) {
                    bfs.add(*id).unwrap();
                }
            }
        }
        return result;
    }

    fn update_dominators(
        &self,
        cur_id: NodeId,
        origin: NodeId,
        result: &mut HashMap<NodeId, NodeId>,
    ) -> () {
        let reachable = self.reverse_postorder(origin);
        let mut reachable_set = HashSet::<NodeId>::default();
        for node in reachable {
            reachable_set.insert(node);
        }
        let mut reached = Vec::<NodeId>::default();
        let mut visited = HashSet::<NodeId>::default();
        visited.insert(cur_id);
        self.dfs(origin, &mut reached, &mut visited);
        for id in reached {
            reachable_set.remove(&id);
        }
        reachable_set.remove(&cur_id);
        for id in reachable_set {
            result.insert(id, cur_id);
        }
    }

    pub fn put_labels(&mut self, begin: NodeId) -> () {
        self.put_merge_labels(begin);
        self.put_loop_labels(begin);
        self.put_if_labels(begin);
    }

    fn put_merge_labels(&mut self, begin: NodeId) {
        for (id, node) in &self.id2node {
            let mut forward_inedjes = 0;
            for origin in &node.prec {
                if self.is_forward(begin, *origin, *id) {
                    forward_inedjes += 1;
                }
            }
            if forward_inedjes >= 2 {
                self.merge_nodes.insert(*id);
            }
        }
    }
    // TODO: check if edge exist
    fn is_forward(&self, begin: NodeId, from: NodeId, to: NodeId) -> bool {
        let order = self.reverse_postorder(begin);
        for id in order {
            if id == from {
                return true;
            }
            if id == to {
                return false;
            }
        }
        return false;
    }

    fn is_backward(&self, begin: NodeId, from: NodeId, to: NodeId) -> bool {
        return !self.is_forward(begin, from, to);
    }

    fn put_loop_labels(&mut self, begin: NodeId) -> () {
        for (id, node) in &self.id2node {
            for origin in &node.prec {
                if self.is_backward(begin, *origin, *id) {
                    self.loop_nodes.insert(*id);
                    break;
                }
            }
        }
    }

    fn put_if_labels(&mut self, _begin: NodeId) -> () {
        for (id, node) in &self.id2node {
            if node.succ.len() > 1 {
                self.if_nodes.insert(*id);
            }
        }
    }

    pub fn print_labels(&self) -> () {
        for (id, _node) in &self.id2node {
            print!("{}: ", id);
            if self.merge_nodes.contains(id) {
                print!("MERGE, ");
            }
            if self.loop_nodes.contains(id) {
                print!("LOOP, ");
            }
            if self.if_nodes.contains(id) {
                print!("IF, ");
            }
            print!("\n");
        }
    }

    pub fn gen_dot(&self, graphname: &str) -> () {
        let mut res = format!("digraph {graphname} {{ \n");
        for i in 0..self.next_id {
            let s = format!("    N{i}[label=\"N{i}\"];\n");
            res.push_str(&s);
        }
        for (from, node) in &self.id2node {
            for to in &node.succ {
                let s = format!("    N{from} -> N{to}[label=\"\"];\n");
                res.push_str(&s);
            }
        }
        res.push_str("}\n");
        let mut file = File::create(format!("dots/{graphname}.dot")).unwrap();
        file.write_all(res.as_bytes()).unwrap();
    }

    pub fn reducable(&self) -> Graph {
        return Supergraph::default().build(&self).run().cfg();
    }
}

pub fn read_graph(filepath: &str) -> Graph {
    println!("{}:", filepath);
    let mut fullpath = "test/".to_owned();
    fullpath.push_str(filepath);
    let data = fs::read_to_string(fullpath).unwrap();
    let lines = data.split("\n").collect::<Vec<&str>>();
    let mut result = Graph::new();
    let size = lines[0].parse::<NodeId>().unwrap();
    for _ in 0..size {
        result.add_vertex(Block::new());
    }
    for line in lines {
        if line.contains(" ") {
            let nums = line.split(" ").collect::<Vec<&str>>();
            result.add_edge(
                nums[0].parse::<NodeId>().unwrap(),
                nums[1].parse::<NodeId>().unwrap(),
            )
        }
    }
    return result;
}

pub type SuperNodeId = isize;

#[derive(Default, Clone)]
struct SuperNode {
    super_id: SuperNodeId,
    cfg_ids: HashSet<NodeId>, // which cfg nodes in this supernode
    cfg_ids2cfg_nodes: HashMap<NodeId, Node>,
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
struct Supergraph {
    id2node: HashMap<SuperNodeId, SuperNode>,
    clone2origin: HashMap<NodeId, NodeId>, // clone2origin[x] = y means that cfg node with id=x was clonned from cfg node with id=y;
    origin2block: HashMap<NodeId, Block>,
    next_id: SuperNodeId,
    next_cfg_id: NodeId,
}

impl Supergraph {
    // i think here is a mistake
    pub fn build(mut self, g: &Graph) -> Supergraph {
        println!("len g.id2node = {}", g.id2node.len());
        for (id, block) in &g.id2block {
            self.origin2block.insert(*id, block.clone());
        }
        for (_id, cfg_node) in &g.id2node {
            let tmp = SuperNode::default();
            self.id2node
                .insert(self.next_id, tmp.build(cfg_node.clone(), self.next_id));
            self.next_id += 1;
        }
        self.next_cfg_id = g.next_id;
        // TODO: make superedges;
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
            return self;
        }
    }

    pub fn in_which_supernode(&self, nid: NodeId) -> SuperNodeId {
        println!("in which supernode called with nid = {}", nid);
        for (id, node) in &self.id2node {
            format!("super id {}\n", id);
            for sid in &node.cfg_ids {
                print!("\tsub id {}\n", sid);
            }
            if node.cfg_ids.contains(&nid) {
                return *id;
            }
        }
        println!("BBBBBBBBBBBBBBBBBB");
        println!("len id2node = {}", &self.id2node.len());
        panic!("No such node");
    }

    pub fn can_merge(&self) -> bool {
        for (_sid, snode) in &self.id2node {
            let mut super_precs: HashSet<SuperNodeId> = HashSet::default();
            for (_cfg_id, cfg_node) in &snode.cfg_ids2cfg_nodes {
                for prec_id in &cfg_node.prec {
                    super_precs.insert(self.in_which_supernode(*prec_id));
                }
            }
            if super_precs.len() == 1 {
                return true;
            }
        }
        return false;
    }

    pub fn can_clone(&self) -> bool {
        for (_sid, snode) in &self.id2node {
            let mut super_precs: HashSet<SuperNodeId> = HashSet::default();
            for (_cfg_id, cfg_node) in &snode.cfg_ids2cfg_nodes {
                for prec_id in &cfg_node.prec {
                    super_precs.insert(self.in_which_supernode(*prec_id));
                }
            }
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
                    super_precs.insert(self.in_which_supernode(*prec_id));
                }
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
                    super_precs.insert(self.in_which_supernode(*prec_id));
                }
            }
            if super_precs.len() == 1 {
                return (super_precs, *sid);
            }
        }
        panic!("no clonable nodes");
    }

    pub fn merge(&mut self, (master, slave): (SuperNodeId, SuperNodeId)) -> () {
        println!("merge nodes slave : {}, master : {}", slave, master);
        self.make_clone((master, slave));
        self.id2node.remove(&slave);
    }

    pub fn split(&mut self, (masters, slave): (HashSet<SuperNodeId>, SuperNodeId)) -> () {
        println!("split nodes slave : {}, masters:", slave);
        for id in &masters {
            println!("{},", id)
        }
        for master in masters {
            self.make_clone((master, slave));
        }
        self.id2node.remove(&slave);
    }

    pub fn make_clone(&mut self, (master, slave): (SuperNodeId, SuperNodeId)) -> () {
        let mut new_cfg_ids: HashMap<NodeId, NodeId> = HashMap::default(); // old -> new
        for id in &self.id2node.get(&slave).unwrap().cfg_ids {
            new_cfg_ids.insert(*id, self.next_id);
            self.next_cfg_id += 1;
        }
        for (old, _new) in &new_cfg_ids {
            let node = self
                .id2node
                .get(&slave)
                .unwrap()
                .cfg_ids2cfg_nodes
                .get(&old)
                .unwrap()
                .clone();

            let new_id = *new_cfg_ids.get(&node.id).unwrap();

            self.copy_to_other_supernode(node.clone(), slave, master, new_id);
        }
    }

    pub fn cfg(self) -> Graph {
        let mut id2n: HashMap<NodeId, Node> = HashMap::default();
        let mut id2b: HashMap<NodeId, Block> = HashMap::default();
        for (_sid, snode) in self.id2node {
            for (cid, cnode) in snode.cfg_ids2cfg_nodes {
                id2n.insert(cid, cnode);
                id2b.insert(
                    cid,
                    self.origin2block
                        .get(self.clone2origin.get(&cid).unwrap())
                        .unwrap()
                        .clone(),
                );
            }
        }
        return Graph {
            id2block: (id2b),
            id2node: (id2n),
            next_id: (self.next_cfg_id),
            merge_nodes: (HashSet::default()),
            loop_nodes: (HashSet::default()),
            if_nodes: (HashSet::default()),
        };
    }

    pub fn copy_to_other_supernode(
        &mut self,
        node: Node,
        snode_from_id: SuperNodeId,
        snode_to_id: SuperNodeId,
        new_id: NodeId,
    ) -> () {
        let snode_from = self.id2node.get(&snode_from_id).unwrap().clone();
        let mut snode_to = self.id2node.get(&snode_to_id).unwrap().clone();
        snode_to.cfg_ids.insert(new_id);
        snode_to.cfg_ids2cfg_nodes.insert(new_id, node.clone());
        snode_to
            .cfg_ids2cfg_nodes
            .get_mut(&new_id)
            .unwrap()
            .prec
            .retain(|prec_id: &NodeId| -> bool {
                snode_from.cfg_ids.contains(&prec_id) && snode_to.cfg_ids.contains(&prec_id)
            });

        let is_foreighn = |prec_id: &NodeId| -> bool {
            snode_from.cfg_ids.contains(&prec_id) && snode_to.cfg_ids.contains(&prec_id)
            // maybe wrong predicate ?
        };

        let mut foreighn_precs: HashSet<(NodeId, NodeId)> = HashSet::default();
        for id in &snode_from.cfg_ids2cfg_nodes.get(&node.id).unwrap().prec {
            // here should be other id. Maybe i fix it ?
            if is_foreighn(id) {
                foreighn_precs.insert((*id, node.id));
            }
        }
        for (from, to) in foreighn_precs {
            let origin = self.in_which_supernode(from);
            self.id2node
                .get_mut(&origin)
                .unwrap()
                .cfg_ids2cfg_nodes
                .get_mut(&from)
                .unwrap()
                .succ
                .remove(&to);

            snode_to
                .cfg_ids2cfg_nodes
                .get_mut(&to)
                .unwrap()
                .prec
                .remove(&from);
        }
        *self.id2node.get_mut(&snode_to_id).unwrap() = snode_to;
    }
}

#[test]
pub fn test_build_1() -> () {
    let g = read_graph("1.txt");
    let sg = Supergraph::default().build(&g);
    for i in 0..7 {
        assert!(sg.origin2block.contains_key(&i));
    }
    let mut cfg_ids: HashSet<NodeId> = HashSet::default();
    for i in 0..7 {
        let node = sg.id2node.get(&i).unwrap();
        assert!(node.super_id == i);
        for id in &node.cfg_ids {
            println!("{}", id);
            cfg_ids.insert(*id);
        }
    }
    for i in 0..7 {
        assert!(cfg_ids.contains(&i));
    }
}

#[test]
pub fn test_node_clone() -> () {
    let node = Node {
        id: 3,
        prec: HashSet::default(),
        succ: HashSet::default(),
    };
    let node_clonned = node.clone();
    assert!(node_clonned.id == 3);
}

#[test]
pub fn test_graph_ids() -> () {
    let g = read_graph("1.txt");
    for id in 0..7 {
        assert!(g.id2node.get(&id).unwrap().id == id);
    }
}
