use crate::graph::cfg::{Cfg, CfgEdge, CfgLabel};
use crate::graph::enrichments::EnrichedCfg;
use crate::graph::relooper::{ReBlock, ReSeq};
use std::fmt::Display;

impl<TLabel: CfgLabel + Display> Cfg<TLabel> {
    pub fn cfg_to_dot(&self, name: &str) -> String {
        let mut lines: Vec<String> = Vec::new();
        lines.push(format!("subgraph cluster_{name} {{ label=\"{name}\";"));
        lines.push(format!("{name}_nstart[label=\"start\"]"));
        lines.push(format!("{name}_nend[label=\"end\"]"));

        for n in self.nodes() {
            lines.push(format!("{name}_n{n}[label=\"{n}\"];"));
        }

        let mut edges: Vec<String> = Vec::new();
        for (n, edge) in self.edges() {
            match edge {
                CfgEdge::Uncond(u) => {
                    edges.push(format!("{name}_n{n} -> {name}_n{u};"));
                }
                CfgEdge::Cond(t, f) => {
                    edges.push(format!("{name}_n{n} -> {name}_n{t}[style=\"dashed\"];"));
                    edges.push(format!("{name}_n{n} -> {name}_n{f};"));
                }
                CfgEdge::Switch(v) => {
                    for (u, t) in v {
                        edges.push(format!(
                            "{name}_n{n} -> {name}_n{t}[style=\"dashed\",text=\"{u}\"]"
                        ));
                    }
                }
                CfgEdge::Terminal => {
                    edges.push(format!("{name}_n{n} -> {name}_nend;"));
                }
            }
        }
        lines.push(format!("{name}_nstart -> {name}_n{}", self.entry));
        lines.extend(edges);
        lines.push("}".to_string());

        lines.join("\n")
    }
}

impl<TLabel: CfgLabel + Display> EnrichedCfg<TLabel> {
    fn labels(&self, n: &TLabel) -> String {
        let mut res = "".to_string();
        if self.loop_nodes.contains(n) {
            res += "l";
        }
        if self.if_nodes.contains(n) {
            res += "i";
        }
        if self.merge_nodes.contains(n) {
            res += "m";
        }

        res
    }

    pub fn cfg_to_dot(&self, name: &str) -> String {
        let mut lines: Vec<String> = Vec::new();
        lines.push(format!("subgraph cluster_{name} {{ label=\"{name}\";"));
        lines.push(format!("{name}_nstart[label=\"start\"]"));
        lines.push(format!("{name}_nend[label=\"end\"]"));

        let mut edges: Vec<String> = Vec::new();
        for &n in self.cfg.nodes() {
            lines.push(format!("{name}_n{n}[label=\"{n} {}\"];", self.labels(&n)));
            match self.cfg.edge(&n) {
                CfgEdge::Uncond(u) => {
                    edges.push(format!("{name}_n{n} -> {name}_n{u};"));
                }
                CfgEdge::Cond(t, f) => {
                    edges.push(format!("{name}_n{n} -> {name}_n{t}[style=\"dashed\"];"));
                    edges.push(format!("{name}_n{n} -> {name}_n{f};"));
                }
                CfgEdge::Switch(v) => {
                    for (u, t) in v {
                        edges.push(format!(
                            "{name}_n{n} -> {name}_n{t}[style=\"dashed\",text=\"{u}\"]"
                        ));
                    }
                }
                CfgEdge::Terminal => {
                    edges.push(format!("{name}_n{n} -> {name}_nend;"));
                }
            }
        }
        lines.push(format!("{name}_nstart -> {name}_n{}", self.cfg.entry));
        lines.extend(edges);
        lines.push("}".to_string());

        lines.join("\n")
    }

    pub fn dom_to_dot(&self) -> String {
        let mut lines: Vec<String> = Vec::new();

        lines.push("subgraph cluster_dom { label=\"dom\"; edge [dir=\"back\"];".to_string());
        for n in self.cfg.nodes() {
            lines.push(format!("d{n}[label=\"{n}\"];"));
        }
        for (&n, &d) in &self.domination.dominated {
            lines.push(format!("d{d} -> d{n};"));
        }
        lines.push("}".to_string());

        lines.join("\n")
    }
}

impl<TLabel: CfgLabel + Display> ReSeq<TLabel> {
    fn to_dot_inner(&self, current_id: usize, back_branches: &Vec<usize>) -> (usize, Vec<String>) {
        let mut res: Vec<String> = Vec::new();

        let (id, _) = self
            .0
            .iter()
            .fold((current_id, None), |(current_id, prev_block), block| {
                let next_id = match block {
                    ReBlock::Block(next) | ReBlock::Loop(next) => {
                        let mut back = back_branches.clone();
                        back.push(current_id);

                        if let ReBlock::Block(_) = block {
                            res.push(format!(
                                "r{current_id}[label=\"Block {current_id}\",shape=\"rectangle\"];"
                            ));
                        } else {
                            res.push(format!("r{current_id}[label=\"Loop {current_id}\"];"));
                        };

                        let ch_id = current_id + 1;
                        let (ch_last_id, ch_str) = next.to_dot_inner(ch_id, &back);
                        res.extend(ch_str);

                        res.push(format!("r{current_id} -> r{ch_id};"));

                        (ch_last_id + 1, Some(current_id))
                    }
                    ReBlock::If(t, f) => {
                        let mut back = back_branches.clone();
                        back.push(current_id);

                        res.push(format!(
                            "r{current_id}[label=\"If {current_id}\", shape=diamond];"
                        ));

                        let t_ch_id = current_id + 1;
                        let (t_id, t_str) = t.to_dot_inner(t_ch_id, &back);
                        res.push(format!("r{current_id} -> r{t_ch_id}[style=\"dashed\"];"));

                        let f_ch_id = t_id + 1;
                        let (f_id, f_str) = f.to_dot_inner(f_ch_id, &back);
                        res.push(format!("r{current_id} -> r{f_ch_id};"));

                        res.extend(t_str);
                        res.extend(f_str);

                        (f_id + 1, Some(current_id))
                    }
                    ReBlock::Actions(label) => {
                        res.push(format!(
                            "r{current_id}[label=\"{label} Actions {current_id}\"];"
                        ));

                        (current_id + 1, Some(current_id))
                    }
                    ReBlock::Br(jmp) => {
                        res.push(format!("r{current_id}[label=\"Br {current_id}\"];"));

                        let branch_to = back_branches
                            .get(back_branches.len() - 1 - (*jmp as usize))
                            .expect("unexpected branch");
                        res.push(format!(
                            "r{current_id} -> r{branch_to}[constraint=false,color=\"blue\"]"
                        ));

                        (current_id + 1, None)
                    }
                    ReBlock::Return => {
                        res.push(format!("r{current_id}[label=\"Return {current_id}\"];"));

                        (current_id + 1, None)
                    }
                    ReBlock::TableJump(table) => {
                        res.push(format!("r{current_id}[label=\"BrTable {current_id}\"];"));

                        for &jmp in table.values() {
                            let branch_to = back_branches
                                .get(back_branches.len() - 1 - (jmp as usize))
                                .expect("unexpected branch");
                            res.push(format!(
                                "r{current_id} -> r{branch_to}[constraint=false,color=\"blue\"]"
                            ));
                        }

                        (current_id + 1, None)
                    }
                };

                if let Some(last_id) = prev_block {
                    res.push(format!(
                        "r{last_id} -> r{current_id}[style=\"bold\",color=\"red\"];"
                    ));
                }

                next_id
            });
        (id, res)
    }

    pub fn to_dot(&self) -> String {
        let (_id, strs) = self.to_dot_inner(0, &vec![]);
        let mut lines: Vec<String> =
            vec!["subgraph cluster_relooped { label=\"relooped\";".to_string()];
        lines.extend(strs);
        lines.push("}".to_string());

        lines.join("\n")
    }
}
