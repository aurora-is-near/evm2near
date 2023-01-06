use std::collections::{BTreeSet, HashSet, VecDeque};
use std::hash::Hash;

pub struct Dfs<T, ChFun> {
    visited: HashSet<T>,
    queue: VecDeque<T>,
    get_children: ChFun,
}

impl<T, ChIt, ChFun> Dfs<T, ChFun>
where
    ChIt: IntoIterator<Item = T>,
    ChFun: FnMut(&T) -> ChIt,
{
    pub fn start_iter<I: IntoIterator<Item = T>>(iter: I, get_children: ChFun) -> Self {
        Dfs {
            visited: HashSet::new(),
            queue: VecDeque::from_iter(iter),
            get_children,
        }
    }

    pub fn start_from(item: T, get_children: ChFun) -> Self {
        Self::start_iter(Some(item).into_iter(), get_children)
    }

    pub fn start_from_except(item: &T, mut get_children: ChFun) -> Self {
        Self::start_iter(get_children(item), get_children)
    }
}

impl<T, ChIt, ChFun> Iterator for Dfs<T, ChFun>
where
    T: Hash + Eq + Copy,
    ChIt: IntoIterator<Item = T>,
    ChFun: FnMut(&T) -> ChIt,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.queue.pop_back().map(|current| {
            let children = (self.get_children)(&current)
                .into_iter()
                .filter(|c| !self.visited.contains(c))
                .collect::<HashSet<_>>();
            for &c in &children {
                self.visited.insert(c);
            }
            self.queue.extend(children.into_iter());

            current
        })
    }
}

fn dfs_post_inner<T, ChIt, ChFun>(
    start: T,
    get_children: &mut ChFun,
    res: &mut Vec<T>,
    visited: &mut HashSet<T>,
) where
    T: Hash + Eq + Copy,
    ChIt: IntoIterator<Item = T>,
    ChFun: FnMut(&T) -> ChIt,
{
    for x in get_children(&start) {
        if !visited.contains(&x) {
            visited.insert(x);
            dfs_post_inner(x, get_children, res, visited);
        }
    }

    res.push(start);
}

pub fn dfs_post<T, ChIt, ChFun>(start: T, get_children: &mut ChFun) -> Vec<T>
where
    T: Hash + Eq + Copy,
    ChIt: IntoIterator<Item = T>,
    ChFun: FnMut(&T) -> ChIt,
{
    let mut visited: HashSet<T> = HashSet::from([start]);
    let mut res: Vec<T> = Vec::new();

    dfs_post_inner(start, get_children, &mut res, &mut visited); //TODO rewrite using Iterator or at least without recursion

    res.reverse();
    res
}

// TODO this duplication drives me mad, but it is the simpliest way to work around absence of common `Map`-like trait for Hash/BTree maps
fn dfs_post_inner_ord<T, ChIt, ChFun>(
    start: T,
    get_children: &mut ChFun,
    res: &mut Vec<T>,
    visited: &mut BTreeSet<T>,
) where
    T: Ord + Eq + Copy,
    ChIt: IntoIterator<Item = T>,
    ChFun: FnMut(&T) -> ChIt,
{
    for x in get_children(&start) {
        if !visited.contains(&x) {
            visited.insert(x);
            dfs_post_inner_ord(x, get_children, res, visited);
        }
    }

    res.push(start);
}

pub fn dfs_post_ord<T, ChIt, ChFun>(start: T, get_children: &mut ChFun) -> Vec<T>
where
    T: Ord + Eq + Copy,
    ChIt: IntoIterator<Item = T>,
    ChFun: FnMut(&T) -> ChIt,
{
    let mut visited: BTreeSet<T> = BTreeSet::from([start]);
    let mut res: Vec<T> = Vec::new();

    dfs_post_inner_ord(start, get_children, &mut res, &mut visited); //TODO rewrite using Iterator or at least without recursion

    res.reverse();
    res
}
