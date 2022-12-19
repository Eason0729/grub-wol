use indexmap::IndexMap;
use std::hash::Hash;

pub struct HashVec<K, V>
where
    K: Hash + Eq,
{
    map: IndexMap<K, Vec<V>>,
}

impl<K, V> Default for HashVec<K, V>
where
    K: Hash + Eq,
{
    fn default() -> Self {
        Self {
            map: Default::default(),
        }
    }
}

impl<K, V> HashVec<K, V>
where
    K: Hash + Eq,
{
    pub fn push(&mut self, key: K, val: V) {
        if let Some(content) = self.map.get_mut(&key) {
            content.push(val);
        } else {
            self.map.insert(key, vec![val]);
        }
    }
    pub fn pop(&mut self, key: &K) -> Option<V> {
        if let Some(content) = self.map.get_mut(key) {
            let result = content.pop();
            if content.is_empty() {
                self.map.remove(key);
            }
            return result;
        }
        None
    }
    pub fn contains_key(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }
    pub fn remove_with_value(&mut self, key: &K, value: &V) -> Option<V>
    where
        V: Hash + Eq,
    {
        if let Some(content) = self.map.get_mut(key) {
            if let Some((i, _)) = content.iter().enumerate().find(|(_, val)| *val == value) {
                let result = content.swap_remove(i);
                if content.is_empty() {
                    self.map.remove(key);
                }
                Some(result)
            } else {
                None
            };
        }
        None
    }
    pub fn is_empty(&self, key: &K) -> bool {
        if let Some(x) = self.map.get(key) {
            x.is_empty()
        } else {
            true
        }
    }
}
