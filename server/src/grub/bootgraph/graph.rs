use indexmap::IndexMap;
use serde;
use std::{collections::VecDeque, fmt::Debug, hash::Hash};

#[derive(Clone, serde::Deserialize, serde::Serialize, Debug)]
pub struct Graph<V, E>
where
    V: Hash + Eq,
{
    edges: Vec<Vec<Edge<E>>>,
    values: IndexMap<V, usize>,
}

impl<V, E> Default for Graph<V, E>
where
    V: Hash + Eq,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<V, E> Graph<V, E>
where
    V: Hash + Eq,
{
    pub fn new() -> Self {
        Self {
            edges: vec![],
            values: Default::default(),
        }
    }
    pub fn update_node(&mut self, origin: &V, value: V) -> Option<V> {
        let node = self.values.remove(origin);
        if let Some(id) = node {
            self.values.insert(value, id);
            None
        } else {
            Some(value)
        }
    }
    pub fn list_node(&self) -> impl Iterator<Item = &V> {
        self.values.iter().map(|(node, _)| node)
    }
    pub fn find_node(&self, value: &V) -> Option<Node> {
        let id = self.values.get(value)?.clone();
        Some({
            let id = id;
            Node(id)
        })
    }
    pub fn add_node(&mut self, value: V) -> Node {
        if let Some(node) = self.find_node(&value) {
            node
        } else {
            let value = value;
            let id = self.values.len();
            self.values.insert(value, id);
            self.edges.push(vec![]);

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
            if edge.to == to.0 {
                contain = true;
            }
        });
        contain
    }
    pub fn bfs(&self, root: &Node) -> BFS<'_, V, E> {
        let mut queue = VecDeque::new();
        self.edges[root.0].iter().for_each(|edge| {
            queue.push_back(edge);
        });
        BFS { graph: self, queue }
    }
    pub fn dfs(&self, root: &Node) -> DFS<'_, V, E> {
        let mut stack = VecDeque::new();
        self.edges[root.0].iter().for_each(|edge| {
            stack.push_back(edge);
        });
        DFS { graph: self, stack }
    }
    pub fn dijkstra<'a>(&'a self, from: &Node) -> Dijkstra<'a, E> {
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
            const COsT: usize = 1;

            self.edges[from_l].iter().for_each(|edge| {
                let edge_dist = edge.to;
                let dist_d = distance[edge_dist].unwrap_or(usize::MAX);
                if from_d < dist_d {
                    distance[edge_dist] = Some(from_d + COsT);
                    last_node[edge_dist] = from_l;
                    last_edge[edge_dist] = Some(&edge.value);
                    queue.push_back(edge_dist);
                }
            })
        }

        Dijkstra {
            distance,
            last_node,
            last_edge,
        }
    }
    pub fn transform_node<F, T>(self, mut f: F) -> Graph<T, E>
    where
        F: FnMut(V) -> T,
        T: Hash + Eq,
    {
        let mut values = IndexMap::default();
        self.values.into_iter().for_each(|(k, v)| {
            values.insert(f(k), v);
        });
        Graph {
            edges: self.edges,
            values,
        }
    }
}

// TODO: impl binary multiply of LCA to speed up tracing on large graph
pub struct Dijkstra<'a, E> {
    distance: Vec<Option<usize>>,
    last_node: Vec<usize>,
    last_edge: Vec<Option<&'a E>>,
}

impl<'a, E> Dijkstra<'a, E> {
    pub fn to(&self, dist: &Node) -> Option<usize> {
        self.distance[dist.0]
    }
    pub fn trace(&self, dist: &Node) -> Option<Vec<&E>> {
        match self.distance[dist.0] {
            Some(_) => {
                let mut trace: Vec<&E> = vec![];
                let mut last = dist.0;

                while last != self.last_node[last] {
                    trace.push(self.last_edge[last].unwrap());
                    last = self.last_node[last];
                }

                trace.reverse();

                Some(trace)
            }
            None => None,
        }
    }
}

pub struct BFS<'a, V, E>
where
    V: Hash + Eq,
{
    graph: &'a Graph<V, E>,
    queue: VecDeque<&'a Edge<E>>,
}

impl<'a, V, E> Iterator for BFS<'a, V, E>
where
    V: Hash + Eq,
{
    type Item = (&'a E, Node);

    fn next(&mut self) -> Option<Self::Item> {
        if !self.queue.is_empty() {
            let current_edge = self.queue.pop_front().unwrap();
            self.graph.edges[current_edge.to].iter().for_each(|edge| {
                self.queue.push_back(edge);
            });
            Some((&current_edge.value, Node(current_edge.to)))
        } else {
            None
        }
    }
}

pub struct DFS<'a, V, E>
where
    V: Hash + Eq,
{
    graph: &'a Graph<V, E>,
    stack: VecDeque<&'a Edge<E>>,
}

impl<'a, V, E> Iterator for DFS<'a, V, E>
where
    V: Hash + Eq,
{
    type Item = (&'a E, Node);

    fn next(&mut self) -> Option<Self::Item> {
        if !self.stack.is_empty() {
            let current_edge = self.stack.pop_back().unwrap();
            self.graph.edges[current_edge.to].iter().for_each(|edge| {
                self.stack.push_back(edge);
            });
            Some((&current_edge.value, Node(current_edge.to)))
        } else {
            None
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

impl Node {
    pub unsafe fn new(index: usize) -> Self {
        Self(index)
    }
}

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

        assert_eq!(g.dijkstra(&c).trace(&a), Some(vec![&"edge c to a"]));
    }
}
