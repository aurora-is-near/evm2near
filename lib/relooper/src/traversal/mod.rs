pub mod graph;
pub mod tree;

// trait TreeTraversal {
//     type
//     pub fn tree_from_iter<I: Iterator<Item = T>>(iter: I, get_children: ChFun) -> Self;
// }

// struct GraphWalk<I, F> {
//     iter: I,
//     pred: F,
// }
//
// impl<T, It: Iterator<Item = T>, Pred: Fn(&T) -> bool> Iterator for GraphWalk<It, Pred> {
//     type Item = T;
//
//     fn next(&mut self) -> Option<Self::Item> {
//         self.iter.filter(&self.pred).next()
//     }
// }

// trait GraphTraversal {}

// impl<T, ChIt, ChFun, OIt, OFun> Bfs<T, ChFun>
// where
//     T: Eq + Hash + Copy,
//     ChIt: Iterator<Item = T>,
//     ChFun: FnMut(&T) -> ChIt,
//     OIt: Iterator<Item = T>,
//     OFun: FnMut(&T) -> OIt,
// {
//     fn graph_from_iter<I: Iterator<Item = T>>(iter: I, get_children: ChFun) -> Bfs<T, OIt> {
//         let mut visited: HashSet<T> = HashSet::new();
//         Bfs::tree_from_iter(iter, |l| {
//             get_children(l)
//                 .filter(|c| {
//                     if visited.contains(c) {
//                         false
//                     } else {
//                         visited.insert(c.to_owned());
//                         true
//                     }
//                 })
//                 .collect::<Vec<_>>() //TODO remove collect
//                 .into_iter()
//         })
//     }
// }

// impl<T, ChIt> Bfs<T, FnMut(&T) -> ChIt>
// where
//     T: Eq + Hash + Copy,
//     ChIt: Iterator<Item = T>,
// {
//     fn filter_visited(c: &T) -> bool {
//         let mut visited: HashSet<T> = HashSet::new(); //TODO
//
//         let res = visited.contains(c);
//         if res {
//             false
//         } else {
//             visited.insert(c.to_owned());
//             true
//         }
//     }
//
//     fn filter_visited_children(
//         get_children: FnMut(&T) -> ChIt,
//     ) -> impl Fn(&T) -> GraphWalk<ChIt, fn(&T) -> bool> {
//         let mut visited: HashSet<T> = HashSet::new();
//
//         move |node| {
//             get_children(node).filter(|c| {
//                 let res = visited.contains(c);
//                 if res {
//                     false
//                 } else {
//                     visited.insert(c.to_owned());
//                     true
//                 }
//             })
//             GraphWalk {
//                 iter: get_children(node),
//                 pred: Self::filter_visited,
//             }
//         }
//     }
//
//     // pub fn graph_from_iter<I: Iterator<Item = T>>(iter: I, get_children: FnMut(&T) -> ChIt) -> Self {
//     //     Bfs::<T, fn(&T) -> GraphWalk<ChIt, fn(&T) -> bool>>::tree_from_iter(
//     //         iter,
//     //         Self::filter_visited_children(get_children),
//     //     )
//     // }
//     //
//     // pub fn graph_from(item: T, get_children: FnMut(&T) -> ChIt) -> Self {
//     //     Self::graph_from_iter(Some(item).into_iter(), get_children)
//     // }
//     //
//     // pub fn graph_from_except(item: T, get_children: FnMut(&T) -> ChIt) -> Self {
//     //     Self::graph_from_iter(get_children(&item), get_children)
//     // }
// }
