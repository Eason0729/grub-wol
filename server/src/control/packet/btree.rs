use indexmap::IndexMap;
use std::collections::BTreeMap;
use std::hash::Hash;

pub struct HashVec<K, V>
where
    K: Hash + Eq,
{
    tree: IndexMap<K, Vec<V>>,
}

impl<K, V> Default for HashVec<K, V>
where
    K: Hash + Eq,
{
    fn default() -> Self {
        Self {
            tree: Default::default(),
        }
    }
}

impl<K, V> HashVec<K, V>
where
    K: Hash + Eq,
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
    pub fn remove_with_value(&mut self, key: &K, value: &V) -> Option<V>
    where
        V: Hash + Eq,
    {
        if let Some(content) = self.tree.get_mut(key) {
            if let Some((i, _)) = content.iter().enumerate().find(|(_, val)| *val == value) {
                let result = content.swap_remove(i);
                if content.is_empty() {
                    self.tree.remove(key);
                }
                Some(result)
            } else {
                None
            };
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
