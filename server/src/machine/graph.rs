use serde;
use std::{
    collections::{BTreeMap, VecDeque},
    fmt::Debug,
};

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct Graph<V, E>
where
    V: Ord,
{
    edges: Vec<Vec<Edge<E>>>,
    values: BTreeMap<V, usize>,
}

impl<V, E> Graph<V, E>
where
    V: Ord,
{
    pub fn new() -> Self {
        Self {
            edges: vec![],
            values: BTreeMap::default(),
        }
    }
    pub fn list_node(&self) -> impl Iterator<Item = &V> {
        self.values.iter().map(|(node, _)| node)
    }
    pub fn find_node(&mut self, value: &V) -> Option<Node> {
        let id = self.values.get(value)?.clone();
        Some({
            let id = id;
            Node(id)
        })
    }
    pub fn add_node(&mut self, value: V) -> Node {
        let value = value;
        let id = self.values.len();
        self.values.insert(value, id);
        self.edges.push(vec![]);
        {
            let id = id;
            Node(id)
        }
    }
    pub fn connect(&mut self, from: Node, to: Node, value: E) {
        let parent = from.0;
        let child = to.0;
        let edge = Edge::new(child, value);
        self.edges[parent].push(edge);
    }
    pub fn has_direct_edge(&mut self, from: Node, to: Node) -> bool {
        let mut contain = false;
        self.edges[from.0].iter().for_each(|edge| {
            contain = true;
        });
        contain
    }
    pub fn dijkstra(&mut self, from: &Node, dist: &Node) -> Option<Vec<&E>> {
        type NodeId = usize;
        type Distance = usize;

        let mut distance: Vec<Option<Distance>> = vec![None; self.edges.len()];
        let mut last_node: Vec<NodeId> = vec![from.0; self.edges.len()];
        let mut last_edge: Vec<Option<&E>> = vec![None; self.edges.len()];
        let mut queue = VecDeque::new();

        distance[from.0] = Some(0);
        queue.push_back(from.0);

        while !queue.is_empty() {
            let from_l = queue.pop_front().unwrap();
            let from_d = distance[from_l].unwrap();
            const COST: usize = 1;

            self.edges[from_l].iter().for_each(|edge| {
                let edge_dist = edge.to;
                let dist_d = distance[edge_dist].unwrap_or(usize::MAX);
                if from_d < dist_d {
                    distance[edge_dist] = Some(from_d + COST);
                    last_node[edge_dist] = from_l;
                    last_edge[edge_dist] = Some(&edge.value);
                    queue.push_back(edge_dist);
                }
            })
        }

        match distance[dist.0] {
            Some(_) => {
                let mut trace: Vec<&E> = vec![];
                let mut last = dist.0;

                while last != last_node[last] {
                    trace.push(last_edge[last].unwrap());
                    last = last_node[last];
                }

                trace.reverse();

                Some(trace)
            }
            None => None,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Edge<E> {
    to: usize,
    value: E,
}

impl<E> Edge<E> {
    fn new(to: usize, value: E) -> Self {
        Self { to, value }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Node(usize);

#[cfg(test)]
mod test {
    use super::Graph;

    #[test]
    fn graph() {
        let mut g = Graph::new();
        let a = g.add_node("node a".to_string());
        let b = g.add_node("node b".to_string());
        g.connect(a, b, "edge a to b".to_string());

        assert_eq!(g.find_node(&"node a".to_string()), Some(a));
        assert_eq!(g.find_node(&"node b".to_string()), Some(b));
    }
    #[test]
    fn dijkstra() {
        let mut g = Graph::new();
        let a = g.add_node("node a".to_string());
        let b = g.add_node("node b".to_string());
        let c = g.add_node("node c".to_string());
        let d = g.add_node("node d".to_string());

        g.connect(a, b, "edge a to b");
        g.connect(b, c, "edge b to c");
        g.connect(c, d, "edge c to d");
        g.connect(d, a, "edge d to a");
        g.connect(a, c, "edge a to c");
        g.connect(c, a, "edge c to a");
        g.connect(c, c, "edge c to c");

        assert_eq!(g.dijkstra(&c, &a), Some(vec![&"edge c to a"]));
    }
}
