use std::collections::{BTreeSet, HashSet, VecDeque};
use std::fmt::Debug;
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

pub trait Contains<T> {
    fn contains(&self, item: &T) -> bool;
    fn insert(&mut self, item: T);
}

impl<T> Contains<T> for HashSet<T>
where
    T: Eq + Hash,
{
    fn contains(&self, item: &T) -> bool {
        HashSet::contains(self, item)
    }

    fn insert(&mut self, item: T) {
        HashSet::insert(self, item);
    }
}

impl<T> Contains<T> for BTreeSet<T>
where
    T: Eq + Ord,
{
    fn contains(&self, item: &T) -> bool {
        BTreeSet::contains(self, item)
    }

    fn insert(&mut self, item: T) {
        BTreeSet::insert(self, item);
    }
}

#[derive(Debug)]
pub struct DfsPost<T, ChFun, TContains> {
    visited: TContains,
    queued: TContains,
    stack: Vec<VecDeque<T>>,
    get_children: ChFun,
}

impl<T, ChIt, ChFun, TContains> Iterator for DfsPost<T, ChFun, TContains>
where
    T: Eq + Copy,
    ChIt: IntoIterator<Item = T>,
    ChFun: FnMut(&T) -> ChIt,
    TContains: Contains<T>,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.stack.last_mut() {
                None => {
                    return None;
                }
                Some(queue) => match queue.front() {
                    None => {
                        self.stack.pop();
                    }
                    Some(qtop) => {
                        if self.visited.contains(qtop) {
                            if !self.queued.contains(qtop) {
                                self.queued.insert(*qtop);
                                return Some(queue.pop_front().unwrap());
                            } else {
                                queue.pop_front();
                            }
                        } else {
                            self.visited.insert(*qtop);
                            let children = (self.get_children)(qtop)
                                .into_iter()
                                .filter(|x| !self.queued.contains(x) && !self.visited.contains(x))
                                .collect::<VecDeque<_>>();
                            if !children.is_empty() {
                                self.stack.push(children);
                            }
                        }
                    }
                },
            }
        }
    }
}

pub trait DfsPostInstantiator<T, ChFun> {
    type Contains: Contains<T>;
    fn new(start: T, get_children: ChFun) -> DfsPost<T, ChFun, Self::Contains>;
}

pub trait DfsPostReverseInstantiator<T, ChFun, ChIt>: DfsPostInstantiator<T, ChFun>
where
    T: Eq + Copy,
    ChIt: IntoIterator<Item = T>,
    ChFun: FnMut(&T) -> ChIt,
{
    fn reverse(start: T, get_children: ChFun) -> Vec<T> {
        let mut vec: Vec<_> = Self::new(start, get_children).collect();
        vec.reverse();
        vec
    }
}

impl<T, ChIt, ChFun> DfsPostInstantiator<T, ChFun> for DfsPost<T, ChFun, HashSet<T>>
where
    T: Eq + Copy + Hash,
    ChIt: IntoIterator<Item = T>,
    ChFun: FnMut(&T) -> ChIt,
{
    type Contains = HashSet<T>;

    fn new(start: T, get_children: ChFun) -> Self {
        let visited = HashSet::default();
        let queued = HashSet::default();
        Self {
            visited,
            queued,
            stack: vec![VecDeque::from(vec![start])],
            get_children,
        }
    }
}

impl<T, ChIt, ChFun> DfsPostInstantiator<T, ChFun> for DfsPost<T, ChFun, BTreeSet<T>>
where
    T: Eq + Copy + Ord,
    ChIt: IntoIterator<Item = T>,
    ChFun: FnMut(&T) -> ChIt,
{
    type Contains = BTreeSet<T>;

    fn new(start: T, get_children: ChFun) -> Self {
        let visited = BTreeSet::default();
        let queued = BTreeSet::default();
        Self {
            visited,
            queued,
            stack: vec![VecDeque::from(vec![start])],
            get_children,
        }
    }
}

impl<T, ChIt, ChFun, Inst: DfsPostInstantiator<T, ChFun>> DfsPostReverseInstantiator<T, ChFun, ChIt>
    for Inst
where
    T: Eq + Copy,
    ChIt: IntoIterator<Item = T>,
    ChFun: FnMut(&T) -> ChIt,
{
}

#[cfg(test)]
mod test {
    use std::collections::{BTreeSet, HashMap, HashSet};

    use crate::traversal::graph::dfs::{DfsPost, DfsPostReverseInstantiator};

    #[test]
    fn test_simple() {
        let map: HashMap<i32, Vec<i32>> = HashMap::from_iter(vec![
            (0, vec![1, 2]),
            (1, vec![3, 4]),
            (2, vec![4, 8]),
            (3, vec![5, 6]),
            (4, vec![9]),
            (5, vec![7]),
            (6, vec![7]),
            (7, vec![9]),
            (8, vec![9, 10]),
            (9, vec![10]),
            (10, vec![]),
        ]);

        let dfs_post_hash: Vec<i32> =
            DfsPost::<_, _, HashSet<_>>::reverse(&0, |x| map.get(x).unwrap())
                .into_iter()
                .copied()
                .collect();
        let dfs_post_ord: Vec<i32> =
            DfsPost::<_, _, BTreeSet<_>>::reverse(&0, |x| map.get(x).unwrap())
                .into_iter()
                .copied()
                .collect();
        assert_eq!(dfs_post_hash, dfs_post_ord);
        assert_eq!(dfs_post_hash, vec![0, 2, 8, 1, 4, 3, 6, 5, 7, 9, 10]);
    }

    #[test]
    fn test_const() {
        let map: HashMap<i32, Vec<i32>> = HashMap::from_iter(vec![
            (0, vec![1, 2]),
            (1, vec![6]),
            (2, vec![3, 4]),
            (3, vec![4, 5]),
            (4, vec![6]),
            (5, vec![6]),
            (6, vec![]),
        ]);

        let dfs_post_hash: Vec<i32> =
            DfsPost::<_, _, HashSet<_>>::reverse(&0, |x| map.get(x).unwrap())
                .into_iter()
                .copied()
                .collect();
        let dfs_post_ord: Vec<i32> =
            DfsPost::<_, _, BTreeSet<_>>::reverse(&0, |x| map.get(x).unwrap())
                .into_iter()
                .copied()
                .collect();

        assert_eq!(dfs_post_hash, dfs_post_ord);
        assert_eq!(dfs_post_hash, vec![0, 2, 3, 5, 4, 1, 6]);
    }

    #[test]
    fn test_const_modified() {
        let map: HashMap<i32, Vec<i32>> = HashMap::from_iter(vec![
            (0, vec![1, 2]),
            (1, vec![6]),
            (2, vec![3, 4]),
            (3, vec![4, 5]),
            (4, vec![5, 6]),
            (5, vec![6]),
            (6, vec![]),
        ]);

        let dfs_post_hash: Vec<i32> =
            DfsPost::<_, _, HashSet<_>>::reverse(&0, |x| map.get(x).unwrap())
                .into_iter()
                .copied()
                .collect();
        let dfs_post_ord: Vec<i32> =
            DfsPost::<_, _, BTreeSet<_>>::reverse(&0, |x| map.get(x).unwrap())
                .into_iter()
                .copied()
                .collect();

        assert_eq!(dfs_post_hash, dfs_post_ord);
        assert_eq!(dfs_post_hash, vec![0, 2, 3, 4, 5, 1, 6]);
    }

    #[test]
    fn test_simple_cycle() {
        let map: HashMap<i32, Vec<i32>> = HashMap::from_iter(vec![
            (0, vec![1]),
            (1, vec![2, 3]),
            (2, vec![0, 4]),
            (3, vec![4]),
            (4, vec![]),
        ]);

        let dfs_post_hash: Vec<i32> =
            DfsPost::<_, _, HashSet<_>>::reverse(&0, |x| map.get(x).unwrap())
                .into_iter()
                .copied()
                .collect();

        let dfs_post_ord: Vec<i32> =
            DfsPost::<_, _, BTreeSet<_>>::reverse(&0, |x| map.get(x).unwrap())
                .into_iter()
                .copied()
                .collect();

        assert_eq!(dfs_post_hash, dfs_post_ord);
        assert_eq!(dfs_post_hash, vec![0, 1, 3, 2, 4]);
    }
}
