#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct EphemeralMap<K, V> {
    frame_nr: u64,
    map: std::collections::HashMap<K, V>,
}

impl<K, V> Default for EphemeralMap<K, V> {
    #[inline]
    fn default() -> Self {
        Self {
            frame_nr: 0,
            map: std::collections::HashMap::default(),
        }
    }
}

impl<K: Eq + std::hash::Hash + Clone, V> EphemeralMap<K, V> {
    #[inline]
    pub(crate) fn may_advance_frame(&mut self, frame_nr: u64) {
        if frame_nr != self.frame_nr {
            self.frame_nr = frame_nr;
            self.map.clear();
        }
    }

    #[inline]
    pub fn get(&mut self, key: &K) -> Option<&V> {
        self.map.get(key)
    }

    #[inline]
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.map.get_mut(key)
    }

    #[inline]
    pub fn entry(&mut self, key: K) -> std::collections::hash_map::Entry<K, V> {
        self.map.entry(key)
    }

    #[inline]
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.map.insert(key, value)
    }

    #[inline]
    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.map.remove(key)
    }
}
