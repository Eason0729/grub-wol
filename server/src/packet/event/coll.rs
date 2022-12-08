use std::collections::BTreeMap;

pub struct BTreeVec<K, V>
where
    K: Ord + Clone,
{
    tree: BTreeMap<K, Vec<V>>,
}

impl<K, V> BTreeVec<K, V>
where
    K: Ord + Clone,
{
    pub fn push(&mut self, key: K, val: V) {
        if let Some(content) = self.tree.get_mut(&key) {
            content.push(val);
        } else {
            self.tree.insert(key, vec![val]);
        }
    }
    pub fn pop(&mut self, key: &K) -> Option<V> {
        if let Some(content) = self.tree.get_mut(key) {
            let result = content.pop();
            if content.is_empty() {
                self.tree.remove(key);
            }
            return result;
        }
        None
    }
    pub fn is_empty(&self, key: &K) -> bool {
        if let Some(x) = self.tree.get(key) {
            x.is_empty()
        } else {
            true
        }
    }
    pub fn find_pop(&mut self, f: impl Fn(&V) -> bool) -> Option<(K, V)> {
        let mut element = None;
        'outer: for (key, val) in &mut self.tree {
            for i in (0..val.len()).rev() {
                if !f(&val[i]) {
                    element = Some((key.clone(), val.swap_remove(i)));
                    break 'outer;
                }
            }
        }
        element
    }
}

impl<K, V> Default for BTreeVec<K, V>
where
    K: Ord + Clone,
{
    fn default() -> Self {
        Self {
            tree: Default::default(),
        }
    }
}
