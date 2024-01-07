use std::sync::Arc;

use egui::util::id_type_map::{SerializableAny, TypeId};
use parking_lot::{lock_api::ArcRwLockWriteGuard, RawRwLock, RwLock};

use crate::{two_frame_map::TwoFrameMap, UseHookExt};

use super::Hook;

#[derive(Default)]
pub struct TwoFrameKvHook<K, V> {
    _marker: std::marker::PhantomData<(K, V)>,
}

impl<K, V> TwoFrameKvHook<K, V> {
    #[inline]
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

impl<K: Clone + Eq + std::hash::Hash + Send + Sync + 'static, V: Send + Sync + 'static, D> Hook<D>
    for TwoFrameKvHook<K, V>
{
    type Backend = ();
    type Output = TwoFrameKv<K, V>;

    fn init(
        &mut self,
        _index: usize,
        _deps: &D,
        _backend: Option<Self::Backend>,
        _ui: &mut egui::Ui,
    ) -> Self::Backend {
    }

    fn hook(self, _backend: &mut Self::Backend, ui: &mut egui::Ui) -> Self::Output {
        let mut kv = ui.use_kv::<(TypeId, TypeId), Arc<RwLock<TwoFrameMap<K, V>>>>();
        let mut lock = kv
            .entry((TypeId::of::<K>(), TypeId::of::<V>()))
            .or_insert_with(|| Arc::new(RwLock::new(TwoFrameMap::<K, V>::new())))
            .write_arc();
        lock.may_advance_frame(ui.ctx().frame_nr());
        TwoFrameKv(lock)
    }
}

#[derive(Default)]
pub struct PersistedTwoFrameKvHook<K, V> {
    _marker: std::marker::PhantomData<(K, V)>,
}

impl<K, V> PersistedTwoFrameKvHook<K, V> {
    #[inline]
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

impl<K: Clone + Eq + std::hash::Hash + SerializableAny, V: SerializableAny, D> Hook<D>
    for PersistedTwoFrameKvHook<K, V>
{
    type Backend = ();
    type Output = TwoFrameKv<K, V>;

    fn init(
        &mut self,
        _index: usize,
        _deps: &D,
        _backend: Option<Self::Backend>,
        _ui: &mut egui::Ui,
    ) -> Self::Backend {
    }

    fn hook(self, _backend: &mut Self::Backend, ui: &mut egui::Ui) -> Self::Output {
        let mut kv = ui.use_persisted_kv::<(TypeId, TypeId), Arc<RwLock<TwoFrameMap<K, V>>>>();
        let mut lock = kv
            .entry((TypeId::of::<K>(), TypeId::of::<V>()))
            .or_insert_with(|| Arc::new(RwLock::new(TwoFrameMap::<K, V>::new())))
            .write_arc();
        lock.may_advance_frame(ui.ctx().frame_nr());
        TwoFrameKv(lock)
    }
}

pub struct TwoFrameKv<K: Eq + std::hash::Hash, V>(
    ArcRwLockWriteGuard<RawRwLock, TwoFrameMap<K, V>>,
);

impl<K: Eq + std::hash::Hash, V> std::ops::Deref for TwoFrameKv<K, V> {
    type Target = TwoFrameMap<K, V>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<K: Eq + std::hash::Hash, V> std::ops::DerefMut for TwoFrameKv<K, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[test]
fn test_clears_the_unused_key() {
    use crate::UseHookExt;
    let ctx = egui::Context::default();

    let _ = ctx.run(Default::default(), |ctx| {
        egui::Area::new("test").show(ctx, |ui| {
            let mut kv = ui.use_2f_kv::<u32, u32>();
            kv.insert(0, 0);
            kv.insert(1, 1);
        });
    });

    let _ = ctx.run(Default::default(), |ctx| {
        egui::Area::new("test").show(ctx, |ui| {
            let mut kv = ui.use_2f_kv::<u32, u32>();
            assert_eq!(kv.get(&0), Some(&0));
            // not access to the key 1
        });
    });

    let _ = ctx.run(Default::default(), |ctx| {
        egui::Area::new("test").show(ctx, |ui| {
            let mut kv = ui.use_2f_kv::<u32, u32>();
            assert_eq!(kv.get(&0), Some(&0));
            assert_eq!(kv.get(&1), None);
        });
    });
}

#[test]
fn test_clears_the_persisted_unused_key() {
    use crate::UseHookExt;
    let ctx = egui::Context::default();

    let _ = ctx.run(Default::default(), |ctx| {
        egui::Area::new("test").show(ctx, |ui| {
            let mut kv = ui.use_persisted_2f_kv::<u32, u32>();
            kv.insert(0, 0);
            kv.insert(1, 1);
        });
    });

    let _ = ctx.run(Default::default(), |ctx| {
        egui::Area::new("test").show(ctx, |ui| {
            let mut kv = ui.use_persisted_2f_kv::<u32, u32>();
            assert_eq!(kv.get(&0), Some(&0));
            // not access to the key 1
        });
    });

    let _ = ctx.run(Default::default(), |ctx| {
        egui::Area::new("test").show(ctx, |ui| {
            let mut kv = ui.use_persisted_2f_kv::<u32, u32>();
            assert_eq!(kv.get(&0), Some(&0));
            assert_eq!(kv.get(&1), None);
        });
    });
}
