use std::collections::HashMap;
use std::hash::Hash;
use std::iter;

struct EdgeListGraph {
    vertices: Vec<usize>,
    edges: Vec<Vec<usize>>
}

trait Graph {
    type VxT : Hash + Eq + Clone;
    fn get_decorator<T: Clone>(&self, default: T) -> HashMap<Self::VxT, T>;
    fn get_vertices(&self) -> Box<dyn Iterator<Item = &Self::VxT> + '_>;
    fn get_connected(&self, vx: &Self::VxT) -> Box<dyn Iterator<Item = &Self::VxT>+ '_>;
    fn size(&self) -> usize;
    fn is_tree(&self) -> bool {
        let mut visited = self.get_decorator(false);
        let mut unvisited = self.size();
        // Start with the first vertex as root.
        let mut vx_iter = self.get_vertices();
        let mut opt_root = vx_iter.next();
        while let Some(root) = opt_root.take() {
            // With our root, iterate through everything we touch 
            let mut next_to_touch : Box<dyn Iterator<Item = &Self::VxT>> = Box::new(iter::once(root));
            loop {
                if let Some(v) = next_to_touch.next() {
                    if *visited.get(v).unwrap() {
                        return false;
                    }
                    unvisited -= 1;
                    next_to_touch = Box::new(next_to_touch.chain(self.get_connected(v)));
                    visited.insert((*v).clone(), true);
                } else {
                    break;
                }
            }
            // We haven't touched all the nodes, find one we haven't touched and make it new root
            if unvisited > 0 {
                while let Some(v) = vx_iter.next() {
                    if !visited.get(v).unwrap() {
                        opt_root = Some(v);
                        break;
                    }
                }
                if opt_root.is_none() {
                    panic!("Couldn't find an unvisited node, but unvisited is {}!", unvisited);
                }
            }
        }
        return true;
    }
}

impl Graph for EdgeListGraph {
    type VxT = usize;
    fn get_decorator<T: Clone>(&self, default: T) -> HashMap<usize, T> {
        let mut dec = HashMap::new();
        for i in 0..self.size() {
            dec.insert(i, default.clone());
        }
        dec
    }
    fn get_vertices(&self) -> Box<dyn Iterator<Item = &usize> + '_> {
        Box::new(self.vertices.iter())
    }
    fn get_connected(&self, vx: &
        usize) -> Box<dyn Iterator<Item = &usize> + '_> {
        Box::new(self.edges[*vx].iter())
    }
    fn size(&self) -> usize {
        self.edges.len()
    }
}

impl EdgeListGraph {
    fn make(el: Vec<Vec<usize>>) -> Result<EdgeListGraph, &'static str> {
        let sz = el.len();
        for edges in el.iter() {
            for e in edges {
                if *e >= sz {
                    return Err("Oh no");
                }
            }
        }
        Ok(EdgeListGraph { vertices: (0..sz).collect(), edges: el })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_elg() -> Result<(), &'static str> {
        let cyc = EdgeListGraph::make(vec![vec![1], vec![0]])?;
        let forest = EdgeListGraph::make(vec![vec![], vec![]])?;
        let norm = EdgeListGraph::make(vec![vec![1], vec![]])?;
        let selfcyc = EdgeListGraph::make(vec![vec![1], vec![1]])?;
        assert!(!cyc.is_tree());
        assert!(forest.is_tree());
        assert!(norm.is_tree());
        assert!(!selfcyc.is_tree());
        Ok(())
    }
}