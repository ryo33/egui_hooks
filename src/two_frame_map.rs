use std::collections::HashMap;

use crate::cleanup::Cleanup;

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct TwoFrameMap<K: Eq + std::hash::Hash, V> {
    #[cfg_attr(feature = "serde", serde(skip))]
    frame_nr: u64,
    current: HashMap<K, V>,
    previous: HashMap<K, V>,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default = "Default::default"))]
    cleanup: Cleanups<K>,
}

#[test]
fn serializable_any() {
    fn assert_serializable_any<T: egui::util::id_type_map::SerializableAny>() {}
    assert_serializable_any::<TwoFrameMap<u32, u32>>();
}

pub struct Cleanups<K> {
    vec: Vec<(K, Box<dyn Cleanup>)>,
}

impl<K> Default for Cleanups<K> {
    #[inline]
    fn default() -> Self {
        Self {
            vec: Vec::default(),
        }
    }
}

impl<K: Eq + std::hash::Hash, V> Default for TwoFrameMap<K, V> {
    #[inline]
    fn default() -> Self {
        Self {
            frame_nr: 0,
            current: HashMap::default(),
            previous: HashMap::default(),
            cleanup: Default::default(),
        }
    }
}

impl<K: Eq + std::hash::Hash + Clone, V: Clone> Clone for TwoFrameMap<K, V> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            frame_nr: self.frame_nr,
            current: self.current.clone(),
            previous: self.previous.clone(),
            cleanup: Default::default(),
        }
    }
}

/// This is a map like the normal HashMap, but it automatically clears entries not used in the
/// previous frame.
/// `TwoFrameMap` stands for you can get a value that used in the two frames, current and previous.
/// `K: Clone` is required because the key is cloned when
impl<K: Eq + std::hash::Hash + Clone, V> TwoFrameMap<K, V> {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub(crate) fn may_advance_frame(&mut self, frame_nr: u64) {
        if frame_nr != self.frame_nr {
            self.frame_nr = frame_nr;
            self.previous = std::mem::take(&mut self.current);
            self.cleanup = Cleanups {
                vec: self
                    .cleanup
                    .vec
                    .drain(..)
                    .filter_map(|(key, mut cleanup)| {
                        if !self.previous.contains_key(&key) {
                            cleanup.cleanup();
                            None
                        } else {
                            Some((key, cleanup))
                        }
                    })
                    .collect(),
            };
        }
    }

    #[inline]
    pub fn get(&mut self, key: &K) -> Option<&V> {
        if !self.current.contains_key(key) {
            if let Some(value) = self.previous.remove(key) {
                self.current.insert(key.clone(), value);
            }
        }
        self.current.get(key)
    }

    #[inline]
    /// Peek the value in the map without advancing the frame.
    pub fn peek(&self, key: &K) -> Option<&V> {
        self.current.get(key).or_else(|| self.previous.get(key))
    }

    #[inline]
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        if !self.current.contains_key(key) {
            if let Some(value) = self.previous.remove(key) {
                self.current.insert(key.clone(), value);
            }
        }
        self.current.get_mut(key)
    }

    #[inline]
    pub fn peek_mut(&mut self, key: &K) -> Option<&mut V> {
        self.current
            .get_mut(key)
            .or_else(|| self.previous.get_mut(key))
    }

    #[inline]
    pub fn entry(&mut self, key: K) -> std::collections::hash_map::Entry<'_, K, V> {
        if !self.current.contains_key(&key) {
            if let Some(value) = self.previous.remove(&key) {
                self.current.insert(key.clone(), value);
            }
        }
        self.current.entry(key)
    }

    #[inline]
    pub fn insert(&mut self, key: K, value: V) {
        self.current.insert(key, value);
    }

    #[inline]
    pub fn contains_key(&mut self, key: &K) -> bool {
        self.current.contains_key(key) || self.previous.contains_key(key)
    }

    #[inline]
    pub fn current(&self) -> &HashMap<K, V> {
        &self.current
    }

    #[inline]
    pub fn current_mut(&mut self) -> &mut HashMap<K, V> {
        &mut self.current
    }

    #[inline]
    pub fn previous(&self) -> &HashMap<K, V> {
        &self.previous
    }

    #[inline]
    pub fn previous_mut(&mut self) -> &mut HashMap<K, V> {
        &mut self.previous
    }

    #[inline]
    pub fn register_cleanup(&mut self, key: K, cleanup: impl FnOnce() + Send + Sync + 'static) {
        self.cleanup.vec.push((key, cleanup.into()));
    }

    #[inline]
    pub(crate) fn register_boxed_cleanup(&mut self, key: K, cleanup: Box<dyn Cleanup>) {
        self.cleanup.vec.push((key, cleanup));
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
