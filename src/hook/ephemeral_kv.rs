use std::sync::Arc;

use parking_lot::{RawRwLock, RwLock, lock_api::ArcRwLockWriteGuard};

use crate::{dispatcher::Dispatcher, ephemeral_map::EphemeralMap};

use super::Hook;

#[derive(Default)]
pub struct EphemeralKvHook<K, V> {
    _marker: std::marker::PhantomData<(K, V)>,
}

impl<K, V> EphemeralKvHook<K, V> {
    #[inline]
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

impl<K: Eq + std::hash::Hash + Send + Sync + 'static, V: Send + Sync + 'static, D> Hook<D>
    for EphemeralKvHook<K, V>
{
    type Backend = Arc<RwLock<EphemeralMap<K, V>>>;
    type Output = EphemeralKv<K, V>;

    fn init(
        &mut self,
        _index: usize,
        _deps: &D,
        _backend: Option<Self::Backend>,
        ui: &mut egui::Ui,
    ) -> Self::Backend {
        // Using hashmap for singleton key-value is inefficient, but it's not a big deal because
        // it's cached as the backend on init.
        Dispatcher::from_ctx(ui.ctx())
            .get_kv_or_default::<(), Self::Backend>()
            .write()
            .entry(())
            .or_default()
            .clone()
    }

    fn hook(self, backend: &mut Self::Backend, ui: &mut egui::Ui) -> Self::Output {
        let mut lock = backend.write_arc();
        lock.may_advance_frame(ui.ctx().cumulative_pass_nr());
        EphemeralKv(lock)
    }
}

pub struct EphemeralKv<K: Eq + std::hash::Hash, V>(
    ArcRwLockWriteGuard<RawRwLock, EphemeralMap<K, V>>,
);

impl<K: Eq + std::hash::Hash, V> std::ops::Deref for EphemeralKv<K, V> {
    type Target = EphemeralMap<K, V>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<K: Eq + std::hash::Hash, V> std::ops::DerefMut for EphemeralKv<K, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[test]
fn clears_on_frame_advance() {
    let ctx = egui::Context::default();

    let _ = ctx.run(Default::default(), |ctx| {
        egui::Area::new("test".into()).show(ctx, |ui| {
            let mut hook = EphemeralKvHook::<u32, u32>::new();
            let mut backend = hook.init(0, &(), None, ui);
            let mut kv = Hook::<()>::hook(hook, &mut backend, ui);
            kv.insert(0, 0);
            kv.insert(1, 1);
        });

        // same frame
        egui::Area::new("test".into()).show(ctx, |ui| {
            let mut hook = EphemeralKvHook::<u32, u32>::new();
            let mut backend = hook.init(0, &(), None, ui);
            let mut kv = Hook::<()>::hook(hook, &mut backend, ui);
            // Can be accessed because it's still in the same frame
            assert_eq!(kv.get(&0), Some(&0));
            assert_eq!(kv.get(&1), Some(&1));
            drop(kv);
        });
    });

    // next frame
    let _ = ctx.run(Default::default(), |ctx| {
        egui::Area::new("test".into()).show(ctx, |ui| {
            assert_eq!(ui.ctx().cumulative_pass_nr(), 1);
            let mut hook = EphemeralKvHook::<u32, u32>::new();
            let mut backend = hook.init(0, &(), None, ui);
            let mut kv = Hook::<()>::hook(hook, &mut backend, ui);
            assert!(kv.get(&0).is_none());
            assert!(kv.get(&1).is_none());
        });
    });
}
