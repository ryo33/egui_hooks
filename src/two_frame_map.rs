use std::collections::HashMap;

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub(crate) struct TwoFrameMap<K, V> {
    frame_nr: u64,
    current: HashMap<K, V>,
    previous: HashMap<K, V>,
    cleanup: Vec<(K, Box<dyn FnOnce() + Send + Sync + 'static>)>,
}

impl<K, V> Default for TwoFrameMap<K, V> {
    fn default() -> Self {
        Self {
            frame_nr: 0,
            current: HashMap::default(),
            previous: HashMap::default(),
            cleanup: Vec::default(),
        }
    }
}

impl<K: Eq + std::hash::Hash + Clone, V> TwoFrameMap<K, V> {
    pub(crate) fn may_advance_frame(&mut self, frame_nr: u64) {
        if frame_nr != self.frame_nr {
            self.frame_nr = frame_nr;
            self.previous = std::mem::take(&mut self.current);
            self.cleanup = self
                .cleanup
                .drain(..)
                .filter_map(|(key, cleanup)| {
                    if !self.previous.contains_key(&key) {
                        cleanup();
                        None
                    } else {
                        Some((key, cleanup))
                    }
                })
                .collect();
        }
    }

    pub(crate) fn get(&mut self, key: &K) -> Option<&V> {
        if !self.current.contains_key(key) {
            if let Some(value) = self.previous.remove(key) {
                self.current.insert(key.clone(), value);
            }
        }
        self.current.get(key)
    }

    pub(crate) fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        if !self.current.contains_key(key) {
            if let Some(value) = self.previous.remove(key) {
                self.current.insert(key.clone(), value);
            }
        }
        self.current.get_mut(key)
    }

    pub(crate) fn entry(&mut self, key: K) -> std::collections::hash_map::Entry<K, V> {
        if !self.current.contains_key(&key) {
            if let Some(value) = self.previous.remove(&key) {
                self.current.insert(key.clone(), value);
            }
        }
        self.current.entry(key)
    }

    pub(crate) fn insert(&mut self, key: K, value: V) {
        self.current.insert(key, value);
    }

    pub(crate) fn register_cleanup(
        &mut self,
        key: K,
        cleanup: impl FnOnce() + Send + Sync + 'static,
    ) {
        self.cleanup.push((key, Box::new(cleanup)));
    }
}

#[test]
fn test_get_current() {
    let mut map = TwoFrameMap::default();
    map.insert("foo", 1);
    assert_eq!(map.get(&"foo"), Some(&1));
    assert_eq!(map.current.get(&"foo"), Some(&1));
    assert_eq!(map.previous.get(&"foo"), None);
}

#[test]
fn test_get_previous_exists() {
    let mut map = TwoFrameMap::default();
    map.insert("foo", 1);
    map.may_advance_frame(1);
    assert_eq!(map.previous.get(&"foo"), Some(&1));
    assert_eq!(map.current.get(&"foo"), None);
    assert_eq!(map.get(&"foo"), Some(&1));
    assert_eq!(map.previous.get(&"foo"), None);
    assert_eq!(map.current.get(&"foo"), Some(&1));
}

#[test]
fn test_get_previous_does_not_exist() {
    let mut map = TwoFrameMap::<&str, i32>::default();
    assert_eq!(map.get(&"foo"), None);
    assert_eq!(map.previous.get(&"foo"), None);
    assert_eq!(map.current.get(&"foo"), None);
}

#[test]
fn test_not_advance_frame() {
    let mut map = TwoFrameMap {
        frame_nr: 42,
        ..Default::default()
    };
    map.insert("foo", 1);
    map.may_advance_frame(42);
    assert_eq!(map.previous.get(&"foo"), None);
    assert_eq!(map.current.get(&"foo"), Some(&1));
}

#[test]
fn test_cleanup_removed_key() {
    let mut map = TwoFrameMap::default();
    map.insert("foo", 1);
    let cleanup_called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let cleanup_called2 = cleanup_called.clone();
    map.register_cleanup("foo", move || {
        cleanup_called2.store(true, std::sync::atomic::Ordering::SeqCst);
    });
    map.may_advance_frame(1);
    // not called yet since the key still exists in the previous frame
    assert!(!cleanup_called.load(std::sync::atomic::Ordering::SeqCst));
    map.may_advance_frame(2);
    // called since the key is removed in the previous frame
    assert!(cleanup_called.load(std::sync::atomic::Ordering::SeqCst));
}

#[test]
fn test_cleanup_non_exist_key() {
    let mut map = TwoFrameMap::<_, i32>::default();
    let cleanup_called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let cleanup_called2 = cleanup_called.clone();
    map.register_cleanup("foo", move || {
        cleanup_called2.store(true, std::sync::atomic::Ordering::SeqCst);
    });
    map.may_advance_frame(1);
    assert!(cleanup_called.load(std::sync::atomic::Ordering::SeqCst));
}
