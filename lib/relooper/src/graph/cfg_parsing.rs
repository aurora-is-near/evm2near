use crate::graph::cfg::CfgEdge::{Cond, Uncond};
use crate::graph::cfg::{Cfg, CfgEdge, CfgLabel};

impl<'a, TLabel: CfgLabel + TryFrom<&'a str, Error = String>> TryFrom<&'a str> for CfgEdge<TLabel> {
    type Error = String;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let (first_str, maybe_second) = {
            let mut split_i = value.split(' ');
            (split_i.next().unwrap(), split_i.next())
        };

        match maybe_second {
            None => TLabel::try_from(first_str).map(Uncond),
            Some(uncond_str) => {
                let cond = TLabel::try_from(first_str)?;
                let uncond = TLabel::try_from(uncond_str)?;
                Ok(Cond(cond, uncond))
            }
        }
    }
}

impl<'a, TLabel: CfgLabel + TryFrom<&'a str, Error = String>> TryFrom<&'a Vec<String>>
    for Cfg<TLabel>
{
    type Error = String;

    fn try_from(strings: &'a Vec<String>) -> Result<Self, Self::Error> {
        if strings.len() < 2 {
            Err("well-formed cfg should contain entry line and at least one edge".to_string())?
        }

        let entry_str = strings.first().unwrap();
        let entry = TLabel::try_from(entry_str)?;

        let mut edges = Vec::with_capacity(strings.len() - 1);

        for edge_str in &strings[1..] {
            let (from, edge) = edge_str
                .split_once(' ')
                .ok_or_else(|| "invalid label-edge format".to_string())?;
            let from = TLabel::try_from(from)?;
            let edge = CfgEdge::try_from(edge)?;
            edges.push((from, edge));
        }
        Cfg::from_edges(edges, entry)
    }
}
