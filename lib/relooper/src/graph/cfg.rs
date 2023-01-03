use crate::graph::cfg::CfgEdge::{Cond, Terminal, Uncond};
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::iter::once;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Default, Debug)]
pub struct CfgLabel(usize);

impl Display for CfgLabel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.0))
    }
}

impl CfgLabel {
    fn try_parse(str: &str) -> Result<CfgLabel, String> {
        str.parse::<usize>()
            .map(CfgLabel)
            .map_err(|err| err.to_string())
    }
}

#[derive(Copy, Clone)]
pub enum CfgEdge {
    Uncond(CfgLabel),
    Cond(CfgLabel, CfgLabel),
    Terminal,
}

impl CfgEdge {
    fn try_parse(str: &str) -> Result<CfgEdge, String> {
        let split_v = str.split(' ').map(|s| s.to_string()).collect::<Vec<_>>();
        match &split_v[..] {
            [to] => CfgLabel::try_parse(to).map(Uncond),
            [t, f] => {
                CfgLabel::try_parse(t).and_then(|t| CfgLabel::try_parse(f).map(|f| Cond(t, f)))
            }
            _ => Err("invalid edge description".to_string()),
        }
    }
}

impl CfgEdge {
    pub fn to_vec(&self) -> Vec<CfgLabel> {
        match self {
            Uncond(u) => vec![*u],
            Cond(cond, fallthrough) => vec![*cond, *fallthrough],
            Terminal => vec![],
        }
    }
}

pub struct Cfg {
    pub(crate) out_edges: HashMap<CfgLabel, CfgEdge>,
    pub(crate) entry: CfgLabel,
}

impl Cfg {
    pub fn from_edges(edges: Vec<(CfgLabel, CfgEdge)>, entry: CfgLabel) -> Result<Self, String> {
        let mut out_edges = HashMap::new();
        let mut nodes = HashSet::new();
        for (from, edge) in edges {
            let old_val = out_edges.insert(from, edge);
            if old_val.is_some() {
                return Err("repeating source node".to_string());
            }
            nodes.insert(from);
            nodes.extend(edge.to_vec());
        }

        for n in nodes {
            out_edges.entry(n).or_insert(Terminal);
        }

        Ok(Self { out_edges, entry })
    }

    pub fn from_strings(strings: Vec<String>) -> Result<Self, String> {
        match &strings[..] {
            [entry, edges @ ..] => {
                let entry = CfgLabel::try_parse(entry)?;
                let edges_vec_res: Vec<_> = edges
                    .iter()
                    .map(|s| {
                        s.split_once(' ')
                            .ok_or_else(|| "invalid label-edge format".to_string())
                            .and_then(|(from, edge)| {
                                CfgLabel::try_parse(from)
                                    .and_then(|f| CfgEdge::try_parse(edge).map(|e| (f, e)))
                            })
                    })
                    .collect();
                //TODO this is too obscure and ugly. the only purpose is to convert Vec<Res<>> to Res<Vec<>>
                let edges_res_vec: Result<Vec<_>, _> = edges_vec_res.into_iter().collect();
                edges_res_vec.and_then(|edges| Cfg::from_edges(edges, entry))
            }
            _ => Err("well-formed cfg should contain entry line and at least one edge".to_string()),
        }
    }

    pub fn nodes(&self) -> HashSet<CfgLabel> {
        self.out_edges
            .iter()
            .flat_map(|(&from, &to)| once(from).chain(to.to_vec()))
            .collect()
    }

    pub fn edge(&self, label: CfgLabel) -> &CfgEdge {
        self.out_edges
            .get(&label)
            .expect("any node should have outgoing edges")
    }

    pub fn children(&self, label: CfgLabel) -> HashSet<CfgLabel> {
        self.out_edges
            .get(&label)
            .into_iter()
            .flat_map(|edge| edge.to_vec())
            .collect()
    }
}
