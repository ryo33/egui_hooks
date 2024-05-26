use std::sync::Arc;

use egui::util::id_type_map::SerializableAny;
use parking_lot::{lock_api::ArcRwLockWriteGuard, RawRwLock, RwLock};

use crate::{dispatcher::Dispatcher, two_frame_map::TwoFrameMap};

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
    type Backend = Arc<RwLock<TwoFrameMap<K, V>>>;
    type Output = TwoFrameKv<K, V>;

    fn init(
        &mut self,
        _index: usize,
        _deps: &D,
        _backend: Option<Self::Backend>,
        ui: &mut egui::Ui,
    ) -> Self::Backend {
        // Using hashmap for singleton key-value is inefficient, but it's not a big deal because it's cached as the backend on init.
        Dispatcher::from_ctx(ui.ctx())
            .get_kv_or_default::<(), Self::Backend>()
            .write()
            .entry(())
            .or_default()
            .clone()
    }

    fn hook(self, backend: &mut Self::Backend, ui: &mut egui::Ui) -> Self::Output {
        let mut lock = backend.write_arc();
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
    type Backend = Arc<RwLock<TwoFrameMap<K, V>>>;
    type Output = TwoFrameKv<K, V>;

    fn init(
        &mut self,
        _index: usize,
        _deps: &D,
        _backend: Option<Self::Backend>,
        ui: &mut egui::Ui,
    ) -> Self::Backend {
        // Using hashmap for singleton key-value is inefficient, but it's not a big deal because it's cached as the backend on init.
        Dispatcher::from_ctx(ui.ctx())
            .get_persisted_kv_or_default::<(), Self::Backend>(ui.ctx())
            .write()
            .entry(())
            .or_default()
            .clone()
    }

    fn hook(self, backend: &mut Self::Backend, ui: &mut egui::Ui) -> Self::Output {
        let mut lock = backend.write_arc();
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
        egui::Area::new("test".into()).show(ctx, |ui| {
            let mut kv = ui.use_2f_kv::<u32, u32>();
            kv.insert(0, 0);
            kv.insert(1, 1);
        });
    });

    let _ = ctx.run(Default::default(), |ctx| {
        egui::Area::new("test".into()).show(ctx, |ui| {
            let mut kv = ui.use_2f_kv::<u32, u32>();
            assert_eq!(kv.get(&0), Some(&0));
            // not access to the key 1
        });
    });

    let _ = ctx.run(Default::default(), |ctx| {
        egui::Area::new("test".into()).show(ctx, |ui| {
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
        egui::Area::new("test".into()).show(ctx, |ui| {
            let mut kv = ui.use_persisted_2f_kv::<u32, u32>();
            kv.insert(0, 0);
            kv.insert(1, 1);
        });
    });

    let _ = ctx.run(Default::default(), |ctx| {
        egui::Area::new("test".into()).show(ctx, |ui| {
            let mut kv = ui.use_persisted_2f_kv::<u32, u32>();
            assert_eq!(kv.get(&0), Some(&0));
            // not access to the key 1
        });
    });

    let _ = ctx.run(Default::default(), |ctx| {
        egui::Area::new("test".into()).show(ctx, |ui| {
            let mut kv = ui.use_persisted_2f_kv::<u32, u32>();
            assert_eq!(kv.get(&0), Some(&0));
            assert_eq!(kv.get(&1), None);
        });
    });
}
