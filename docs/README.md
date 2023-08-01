# Relooper Algorithm Description

## Definitions
In this chapter, we will review some well-known compiler definitions and introduce some definitions specific to this project.

**Control Flow Graph (CFG)** is an abstraction used in compilers and programming language analysis to represent the flow of control within a program. It is a directed graph that models the different paths or sequences of instructions that can be executed in a program. The nodes in the graph represent basic blocks, which are sequences of instructions with no branching or jumping within them.

**Node N dominates node M** if and only if every path from the entry node to node M in the CFG contains node N. The entry node is considered to dominate all nodes in the CFG, including itself.

There are two important cases of dominance:

- **Immediate Dominator (IDOM):** Node N is the immediate dominator of node M if N is the dominator of M, and there is no other dominator of M that also dominates N. In other words, N is the closest dominator to M among all dominators of M.

- **Dominance Frontier:** The dominance frontier of a node N is the set of all nodes that are not strictly dominated by N but have at least one predecessor that is dominated by N. It helps in analyzing control dependencies in a program.

**The Domination Tree** (or Dominator Tree) is a data structure used in graph theory and compiler optimizations, specifically in the context of Control Flow Graphs (CFGs). It provides a hierarchical representation of the dominance relationships between nodes in the CFG.

The Domination Tree is a tree where each node corresponds to a basic block in the CFG. The root of the tree represents the entry node of the CFG, and each node's children are the nodes that are immediately dominated by it. In other words, if node A is the immediate dominator of node B in the CFG, then in the Domination Tree, there is an edge from node A to node B.

**Properties of the Domination Tree:**
- It is a directed tree: The edges in the Domination Tree point from the immediate dominator (parent) to the dominated node (child).
- Unique paths: For each node in the CFG, there is a unique path in the Domination Tree from the root (entry node) to that node, representing the chain of immediate dominators.

**A reducible CFG** is one with edges that can be partitioned into two disjoint sets: forward edges and back edges, such that:
- Forward edges form a directed acyclic graph with all nodes reachable from the entry node.
- For all back edges (A, B), node B dominates node A.

If some of these definitions were new to you, I recommend reading this page: [Control-flow_graph](https://en.wikipedia.org/wiki/Control-flow_graph)

## The Problem
EVM bytecode is very similar to well-known assembly languages, and control flow in it is defined by JMP and CJMP instructions, while WASM control flow looks like control flow of high-level languages -- branching, loops, scopes, BR instruction which just breaks from scopes, and no GOTO-like instruction. So, this algorithm converts GOTO-style control flow to high-level-style control flow.

## Approach
The full description of the algorithm can be read in this paper: [Link to the paper](https://dl.acm.org/doi/pdf/10.1145/3547621). Here are the main steps:

1) **Deal with dynamic edges:** Without extra analysis and information, we must assume that each JMP or CJMP instruction can jump to each JUMPDEST instruction. This outgoing edge of the CFG with an undefined destination is called a "dynamic edge". The Relooper algorithm can't work with dynamic edges, so we need to change all dynamic edges to sets of static edges (edges with known destinations) in our CFG.

2) **Domination tree building:** The domination tree is a helper structure built on top of the control flow graph. This structure is widely used in compilers and in the Relooper algorithm as well.

3) **"Reduce" the control flow graph:** All control flow graphs are reducible or irreducible. The Relooper algorithm can deal only with reducible ones. So, the next step is building an equivalent reducible CFG to the given irreducible one.

4) **Nodes and edges labeling:** The Relooper algorithm needs some flags for all nodes and edges. Nodes can have the following flags: (if, loop, merge). Nodes can have each combination of these flags (even all true or all false). Edges can be forward or backward following way: if the DFS number of an edge's origin is less than the DFS number of its destination, then this edge is forward, and backward otherwise.

5) **Relooping:** It is the final stage of the algorithm.

## Dive Deeper
In this section, we will review all steps more detailed and mention files with code that produce these computations.

1) **Dynamic edges:** This code is processed in the function `basic_cfg(program: &Program) -> BasicCfg` in `bin/evm2near/src/analyze.rs` file. The main approach is creating an extra CFG node without any code called "Dynamic," and all nodes with a dynamic edge now have a static edge to the dynamic node. The dynamic node has a special "switch" edge that points to one of the JUMPDEST instructions according to the jumptable. Later, this edge will be changed to a "switch" wasm instruction.

2) **Domination tree:** The algorithm implemented in the current project is quite big, and you can find the full description [here](https://dl.acm.org/doi/pdf/10.1145/357062.357071). Also, you can find a bit faster but far more complicated algorithm [here](https://dl.acm.org/doi/10.5555/982792.982922) (from my opinion, the priority of upgrading to this algorithm is low). Currently, this implementation contains a bug that is located somewhere in the LINK-EVAL data structure implementation. To reproduce this bug, you need to compile some contract with a big CFG. For example, `test/big-cfg.sol` or `test/Collatz.sol`. If you replace the LINK-EVAL implementation with the naive one, everything will work. Also, there is a more naive implementation of the domination tree algorithm in earlier commits. The implementation of the algorithm is [here](lib/relooper/src/graph/dominators.rs). If you want to make changes in this code, I strongly recommend reading the paper because the algorithm is pretty big, and the code is very close to the paper. If you don't have time to read all the mathematics in the paper, I can recommend you to focus on the semidominator definition, theorem 4, corollary 1, chapters 3, 4, and appendix B.

3) **Reducing:** The main idea of creating an equivalent reducible graph to the given irreducible one is node duplicating. Let's look at the next CFG: A->B, A->C, B->C, C->B, A is origin. This CFG is irreducible since the B-C loop has two headers. We can duplicate node B and create node B', and redirect edges in the following way: A->B, A->C, B->C, C->B', B'->C. You can see that with the same input, both graphs will provide the same execution, but the new graph has only one loop -- C-B', and this loop has exactly one origin -- C, thus it is reducible. This was an idea; now let's take a look at the approach on how to do it for any input CFG. The code is located in `lib/relooper/src/graph/reduction/mod.rs` and `lib/relooper/src/graph/supergraph.rs` for older (deprecated) version. You can find documentation for that approaches in code.

4) **Labeling:** Labeling is a pretty easy step. Each node that has more than one in-edge is called a merge node. Each node that has more than one out-edge is called an if node. Each node that has at least one backward in-edge is called a loop node. Edges are divided into backward and forward following way: if the DFS number of an edge's origin is less than the DFS number of its destination, then this edge is forward, and backward otherwise. You can find code that performs this labeling in `lib/relooper/src/graph/enrichments.rs`.

5) **Relooping:** It is the final stage of the algorithm. This part is also quite difficult and I recommend to read the paper. But there is a short description. Algorithm manipulates with next functions:
• Function doTree is called on a subtree of the dominator tree, rooted at node 𝑋; doTree
returns the translation of the subtree, which includes 𝑋 and everything that 𝑋 dominates.
Function doTree first creates a syntactic template based on the properties of 𝑋 from section 4,
then fills the template with the translations of 𝑋 ’s children. These children are the nodes that
𝑋 immediately dominates.
• Function doBranch is called on the labels of two nodes 𝑋 and 𝑌; it returns code that, when
placed after the translation of 𝑋, transfers control to the translation of 𝑌. If 𝑋 is 𝑌’s only
forward-edge predecessor, doBranch simply returns the translation of 𝑌 (and everything
that 𝑌 dominates). Otherwise 𝑌’s translation already appears in the context, and doBranch
returns a br instruction.
• Function nodeWithin is an auxiliary function; it places the translation of a single node into
a nest of blocks. Function nodeWithin is called by doTree 𝑋, which looks at 𝑋’s children in
the dominator tree and computes Ys: the children of 𝑋 that are merge nodes. Then doTree
passes 𝑋 and Ys to nodeWithin, which returns the translation of 𝑋 (and its other children)
nested inside one block for each element of Ys. The elements of Ys are ordered with higher
reverse postorder numbers first.
This functions get information from domination tree, labeling, and manipulates with one other structure, `context`. It describes the syntactic context into
which WebAssembly code is placed. That context determines the behavior of `br` instructions.
You can find the code that perform relooping in `lib/relooper/src/graph/relooper.rs`

## Helpers
In this section, we briefly introduce some helpful code that is not related to the main algorithm but can be useful for testing, debugging, and benchmarking.

1. **Printing CFG in .dot format:** You can use the following code to print your CFG in .dot format and save it to a file named "cfg.dot".

    ```rust
    let debug = format!("digraph {{{}}}", cfg.cfg_to_dot("cfg"));
    std::fs::write("cfg.dot", debug).expect("fs error while writing debug file");
    ```

2. **Graph Traversals:** If you need to perform some graph traversals (DFS, BFS), check out the code from `lib/relooper/src/traversal`. It is very likely that the traversal you need is already implemented here.

3. **Initializing CFG:** There are several ways to easily initialize your CFG. For example, you can use the `from_edges(entry: TLabel, edges: HashMap<TLabel, CfgEdge<TLabel>>) -> Self` function. Also, there are many useful functions for manipulating CFGs, such as `add_edge_or_promote(&mut self, from: T, to: T)`, `remove_edge(&mut self, from: T, edge: &CfgEdge<T>)`, and others. You can find all of them in `lib/relooper/src/graph/cfg/mod.rs`.

4. **Test Contracts:** You can find some simple contracts in the `test/` directory and use them as input for the compiler.

5. **Tools:** There are some Python scripts for testing and debugging in the `tools/` directory. `tools/test.py` compiles some contracts, runs them in Wasmtime, calls some functions, and asserts that the output is correct. `tools/bench.py` compiles contracts, runs them in the NEAR localnet, measures gas consumption, and produces a CSV with gas consumption of different contracts with different inputs. You can find benchmarking data in the following CSV: `tools/benchmark/csvs/<commit-hash>.csv`. Be careful, Rust code in `tools/benchmark/` makes some assumptions (for example, that contracts are compiled), so it is better not to run this code manually, just run it with `tools/bench.py`.

## Some words about how we store the CFG
Mainly, we have two structures -- `CfgLabel` (node) and `CfgEdge` (edge). Label usually is a id number with some extra information, but a lot of function that process CfgLabels are generic. `CfgEdge` is more interesting structure, it is a enum defined by following:
```
pub enum CfgEdge<TLabel> {
    Uncond(TLabel),
    Cond(TLabel, TLabel),
    Switch(Vec<(usize, TLabel)>),
    Terminal,
}
```
In Cfg each `CfgLabel` have exactly one `CfgLabel`, so it can't contain two Uncond edges. Mapping this structure to mathematical representation of graph is following:
Uncond means that this CFG node have exactly one outedge.
Cond means that this node have exactly two outedges, first is for case when condition is true and second for the opposite case.
Switch means that this node have more than two outedges and store table that maps number on top of stack to destination node.
Terminal means that this node have no outedges and program terminates if we trapped here.

`lib/relooper/test_data` contains some files with CFGs in following format:
```
cfg_origin
/// edges:
edge_origin edge_dest second_edge_dest(if edge is cond)
```
You can parse this files using code from `lib/relooper/src/graph/cfg/cfg_parsing.rs.`

If you still have any questions, don't hesitate to mail: [mcviktor@tut.by](mailto:mcviktor@tut.by)
