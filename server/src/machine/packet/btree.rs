use std::collections::BTreeMap;

pub struct BTreeVec<K, V>
where
    K: Ord,
{
    tree: BTreeMap<K, Vec<V>>,
}

impl<K, V> Default for BTreeVec<K, V>
where
    K: Ord,
{
    fn default() -> Self {
        Self {
            tree: Default::default(),
        }
    }
}

impl<K, V> BTreeVec<K, V>
where
    K: Ord,
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
}
