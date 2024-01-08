use std::{collections::HashMap, sync::Arc};

use egui::util::id_type_map::SerializableAny;
use parking_lot::{lock_api::ArcRwLockWriteGuard, RawRwLock, RwLock};

use crate::dispatcher::Dispatcher;

use super::Hook;

#[derive(Default)]
pub struct KvHook<K, V> {
    _marker: std::marker::PhantomData<(K, V)>,
}

impl<K, V> KvHook<K, V> {
    #[inline]
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

impl<K: Send + Sync + 'static, V: Send + Sync + 'static, D> Hook<D> for KvHook<K, V> {
    type Backend = Arc<RwLock<HashMap<K, V>>>;
    type Output = Kv<K, V>;

    fn init(
        &mut self,
        _index: usize,
        _deps: &D,
        backend: Option<Self::Backend>,
        ui: &mut egui::Ui,
    ) -> Self::Backend {
        backend.unwrap_or_else(|| Dispatcher::from_ctx(ui.ctx()).get_kv_or_default())
    }

    fn hook(self, backend: &mut Self::Backend, _ui: &mut egui::Ui) -> Self::Output {
        Kv(backend.write_arc())
    }
}

#[derive(Default)]
pub struct PersistedKvHook<K, V> {
    _marker: std::marker::PhantomData<(K, V)>,
}

impl<K, V> PersistedKvHook<K, V> {
    #[inline]
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

impl<K: SerializableAny + Eq + std::hash::Hash, V: SerializableAny, D> Hook<D>
    for PersistedKvHook<K, V>
{
    type Backend = Arc<RwLock<HashMap<K, V>>>;
    type Output = Kv<K, V>;

    fn init(
        &mut self,
        _index: usize,
        _deps: &D,
        backend: Option<Self::Backend>,
        ui: &mut egui::Ui,
    ) -> Self::Backend {
        backend
            .unwrap_or_else(|| Dispatcher::from_ctx(ui.ctx()).get_persisted_kv_or_default(ui.ctx()))
    }

    fn hook(self, backend: &mut Self::Backend, _ui: &mut egui::Ui) -> Self::Output {
        Kv(backend.write_arc())
    }
}

pub struct Kv<K, V>(ArcRwLockWriteGuard<RawRwLock, HashMap<K, V>>);

impl<K, V> std::ops::Deref for Kv<K, V> {
    type Target = HashMap<K, V>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<K, V> std::ops::DerefMut for Kv<K, V> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
