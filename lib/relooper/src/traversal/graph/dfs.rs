use std::collections::{HashSet, VecDeque};
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
    ChFun: FnMut(T) -> ChIt,
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

    pub fn start_from_except(item: T, mut get_children: ChFun) -> Self {
        Self::start_iter(get_children(item), get_children)
    }
}

impl<T, ChIt, ChFun> Iterator for Dfs<T, ChFun>
where
    T: Hash + Eq + Copy,
    ChIt: IntoIterator<Item = T>,
    ChFun: FnMut(T) -> ChIt,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.queue.pop_back().map(|current| {
            let children = (self.get_children)(current)
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

#[derive(Debug, PartialEq, Eq)]
pub enum VisitAction<T> {
    Enter(T),
    Leave(T),
}

pub struct PrePostOrder<T, ChFun> {
    visited: HashSet<T>,
    stack: VecDeque<VisitAction<T>>,
    get_children: ChFun,
}

impl<T: Eq + Hash + Clone, ChIt, ChFun> PrePostOrder<T, ChFun>
where
    ChIt: IntoIterator<Item = T>,
    ChFun: FnMut(T) -> ChIt,
{
    pub fn start_iter<I: IntoIterator<Item = T>>(iter: I, get_children: ChFun) -> Self {
        PrePostOrder {
            visited: HashSet::default(),
            stack: VecDeque::from_iter(iter.into_iter().map(|x| VisitAction::Enter(x))),
            get_children,
        }
    }

    pub fn start_from(item: T, get_children: ChFun) -> Self {
        Self::start_iter(Some(item).into_iter(), get_children)
    }

    pub fn start_from_except(item: T, mut get_children: ChFun) -> Self {
        Self::start_iter(get_children(item), get_children)
    }
}

impl<T, ChIt, ChFun> Iterator for PrePostOrder<T, ChFun>
where
    T: Hash + Eq + Copy,
    ChIt: IntoIterator<Item = T>,
    ChFun: FnMut(T) -> ChIt,
{
    type Item = VisitAction<T>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let res = self.stack.pop_back().map(|current| match current {
                VisitAction::Enter(x) => {
                    if self.visited.contains(&x) {
                        None
                    } else {
                        self.visited.insert(x);
                        self.stack.push_back(VisitAction::Leave(x));
                        let mut children: Vec<_> = (self.get_children)(x)
                            .into_iter()
                            .filter(|c| !self.visited.contains(c))
                            .collect();
                        children.reverse();

                        self.stack
                            .extend(children.into_iter().map(|x| VisitAction::Enter(x)));

                        Some(current)
                    }
                }
                VisitAction::Leave(_) => Some(current),
            });
            match res {
                Some(None) => continue,
                Some(Some(r)) => return Some(r),
                None => return None,
            }
        }
    }
}

impl<T, ChIt, ChFun> PrePostOrder<T, ChFun>
where
    T: Hash + Eq + Copy,
    ChIt: IntoIterator<Item = T>,
    ChFun: FnMut(T) -> ChIt,
{
    pub fn postorder(self) -> impl Iterator<Item = T> {
        self.into_iter().filter_map(|action| match action {
            VisitAction::Leave(n) => Some(n),
            _ => None,
        })
    }

    pub fn preorder(self) -> impl Iterator<Item = T> {
        self.into_iter().filter_map(|action| match action {
            VisitAction::Enter(n) => Some(n),
            _ => None,
        })
    }
}

#[cfg(test)]
mod pre_post_order_test {
    use std::collections::HashMap;

    use super::{PrePostOrder, VisitAction::*};

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

        let dfs_post_hash: Vec<i32> = PrePostOrder::start_from(&0, |x| map.get(x).unwrap())
            .postorder()
            .copied()
            .collect();
        assert_eq!(dfs_post_hash, vec![10, 9, 7, 5, 6, 3, 4, 1, 8, 2, 0]);
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

        let dfs_post_hash: Vec<i32> = PrePostOrder::start_from(&0, |x| map.get(x).unwrap())
            .postorder()
            .copied()
            .collect();

        assert_eq!(dfs_post_hash, vec![6, 1, 4, 5, 3, 2, 0]);
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

        let dfs_post_hash: Vec<i32> = PrePostOrder::start_from(&0, |x| map.get(x).unwrap())
            .postorder()
            .copied()
            .collect();

        assert_eq!(dfs_post_hash, vec![6, 1, 5, 4, 3, 2, 0]);
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

        let dfs_post_hash: Vec<i32> = PrePostOrder::start_from(&0, |x| map.get(x).unwrap())
            .postorder()
            .copied()
            .collect();

        assert_eq!(dfs_post_hash, vec![4, 2, 3, 1, 0]);
    }

    #[test]
    fn prepostorder_simple_irr() {
        let map: HashMap<i32, Vec<i32>> = HashMap::from_iter(vec![
            (0, vec![1, 1]),
            (1, vec![2]),
            (2, vec![3]),
            (3, vec![]),
        ]);

        let desired_order = vec![
            Enter(0),
            Enter(1),
            Enter(2),
            Enter(3),
            Leave(3),
            Leave(2),
            Leave(1),
            Leave(0),
        ];

        let preorder: Vec<_> =
            PrePostOrder::start_from(0, |x| map.get(&x).unwrap().to_vec()).collect();
        assert_eq!(desired_order, preorder)
    }

    #[test]
    fn prepostorder_simple() {
        let map: HashMap<i32, Vec<i32>> = HashMap::from_iter(vec![
            (0, vec![1, 6]),
            (1, vec![2, 4]),
            (2, vec![3]),
            (3, vec![]),
            (4, vec![5]),
            (5, vec![3]),
            (6, vec![5]),
        ]);

        let desired_order = vec![
            Enter(0),
            Enter(1),
            Enter(2),
            Enter(3),
            Leave(3),
            Leave(2),
            Enter(4),
            Enter(5),
            Leave(5),
            Leave(4),
            Leave(1),
            Enter(6),
            Leave(6),
            Leave(0),
        ];

        let preorder = PrePostOrder::start_from(0, |x| map.get(&x).unwrap().to_vec());
        assert!(desired_order.into_iter().zip(preorder).all(|(a, b)| a == b))
    }

    #[test]
    fn prepostorder_head_cycle() {
        let map: HashMap<i32, Vec<i32>> = HashMap::from_iter(vec![
            (0, vec![1]),
            (1, vec![2]),
            (2, vec![3, 0]),
            (3, vec![5, 4]),
            (4, vec![5]),
            (5, vec![6]),
            (6, vec![3]),
        ]);

        let desired_order = vec![
            Enter(0),
            Enter(1),
            Enter(2),
            Enter(3),
            Enter(5),
            Enter(6),
            Leave(6),
            Leave(5),
            Enter(4),
            Leave(4),
            Leave(3),
            Leave(2),
            Leave(1),
            Leave(0),
        ];

        let preorder = PrePostOrder::start_from(0, |x| map.get(&x).unwrap().to_vec());
        assert!(desired_order.into_iter().zip(preorder).all(|(a, b)| a == b))
    }
}
