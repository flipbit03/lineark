//! Schema dependency-graph utilities.
//!
//! Used by codegen to decide where `Box<T>` is actually required on
//! Object / InputObject fields. The high-level rule:
//!
//! For a field `Container.f: Target` (a direct, non-list reference to another
//! Object/InputObject), `Box<Target>` is needed iff `Target` can transitively
//! reach `Container` in the schema's reference graph — that's exactly the
//! shape that would otherwise produce an infinite-size Rust struct.
//!
//! Edges through lists are *excluded* from the graph: `Vec<T>` is already a
//! heap pointer with a fixed stack size, so it breaks any size cycle.

use std::collections::{HashMap, HashSet};

/// For each node in `edges`, compute the set of nodes reachable from it
/// (transitively, including the node itself if there's a cycle back to it).
pub fn reachability(edges: &HashMap<String, Vec<String>>) -> HashMap<String, HashSet<String>> {
    let mut result = HashMap::with_capacity(edges.len());
    for start in edges.keys() {
        result.insert(start.clone(), reachable_from(start, edges));
    }
    result
}

fn reachable_from(start: &str, edges: &HashMap<String, Vec<String>>) -> HashSet<String> {
    let mut visited: HashSet<String> = HashSet::new();
    let mut stack: Vec<String> = match edges.get(start) {
        Some(succs) => succs.clone(),
        None => Vec::new(),
    };
    while let Some(node) = stack.pop() {
        if !visited.insert(node.clone()) {
            continue;
        }
        if let Some(succs) = edges.get(&node) {
            for s in succs {
                if !visited.contains(s) {
                    stack.push(s.clone());
                }
            }
        }
    }
    visited
}

/// Returns true iff `target` can reach `container` in the graph — i.e.,
/// embedding `target` directly inside `container` would form a size cycle.
pub fn reaches(target: &str, container: &str, reach: &HashMap<String, HashSet<String>>) -> bool {
    reach.get(target).is_some_and(|r| r.contains(container))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph(pairs: &[(&str, &[&str])]) -> HashMap<String, Vec<String>> {
        pairs
            .iter()
            .map(|(k, vs)| (k.to_string(), vs.iter().map(|s| s.to_string()).collect()))
            .collect()
    }

    #[test]
    fn empty_graph_has_no_reach() {
        let g = graph(&[]);
        assert!(reachability(&g).is_empty());
    }

    #[test]
    fn linear_chain_reach_forward_only() {
        let g = graph(&[("A", &["B"]), ("B", &["C"]), ("C", &[])]);
        let r = reachability(&g);
        assert!(reaches("A", "B", &r));
        assert!(reaches("A", "C", &r));
        assert!(reaches("B", "C", &r));
        assert!(!reaches("B", "A", &r));
        assert!(!reaches("C", "A", &r));
        // No cycles → no node reaches itself.
        assert!(!reaches("A", "A", &r));
    }

    #[test]
    fn self_loop_reaches_itself() {
        let g = graph(&[("A", &["A"])]);
        let r = reachability(&g);
        assert!(reaches("A", "A", &r));
    }

    #[test]
    fn mutual_recursion_both_reach_each_other() {
        let g = graph(&[("A", &["B"]), ("B", &["A"])]);
        let r = reachability(&g);
        assert!(reaches("A", "B", &r));
        assert!(reaches("B", "A", &r));
        assert!(reaches("A", "A", &r));
        assert!(reaches("B", "B", &r));
    }

    #[test]
    fn cycle_not_involving_node_does_not_reach_it() {
        // A → B → C → B (B,C in cycle; A is upstream and not part of it)
        let g = graph(&[("A", &["B"]), ("B", &["C"]), ("C", &["B"])]);
        let r = reachability(&g);
        assert!(reaches("A", "B", &r));
        assert!(reaches("A", "C", &r));
        // B and C reach each other and themselves, but not A.
        assert!(reaches("B", "C", &r));
        assert!(reaches("C", "B", &r));
        assert!(reaches("B", "B", &r));
        assert!(reaches("C", "C", &r));
        assert!(!reaches("B", "A", &r));
        assert!(!reaches("C", "A", &r));
    }
}
