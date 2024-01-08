use std::{
    any::{Any, TypeId},
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use egui::util::id_type_map::SerializableAny;
use parking_lot::RwLock;

use crate::{cleanup::Cleanup, deps::BoxedDeps, hook::Hook, two_frame_map::TwoFrameMap};

#[derive(Default)]
pub struct Dispatcher {
    /// Option<Backend> is used to allow `Option::take` to get owned value from vec without changing
    /// the length of the vec or cloning the value.
    backends: RwLock<TwoFrameMap<egui::Id, BTreeMap<usize, Backend>>>,
    /// kv store for normal kvs.
    kvs: RwLock<KvStore>,
    /// kv store for normal kvs that are persisted.
    persisted_kvs: RwLock<KvStore>,
}

// ahash is ok because type is provided at compile time not runtime (not malicious).
type KvStore = egui::ahash::HashMap<(TypeId, TypeId), Box<dyn Any + Send + Sync>>;

#[test]
fn dispatcher_is_send_and_sync() {
    fn assert_send_and_sync<T: Send + Sync>() {}
    assert_send_and_sync::<Dispatcher>();
}

struct Backend {
    type_id: TypeId,
    value: Box<dyn Any + Send + Sync>,
    deps: BoxedDeps,
}

impl Dispatcher {
    #[inline]
    pub(crate) fn from_ctx(ctx: &egui::Context) -> Arc<Self> {
        ctx.data_mut(|data| {
            data.get_temp_mut_or_default::<Arc<Dispatcher>>(egui::Id::NULL)
                .clone()
        })
    }

    #[inline]
    pub(crate) fn may_advance_frame(&self, frame_nr: u64) {
        self.backends.write().may_advance_frame(frame_nr);
    }

    #[inline]
    pub(crate) fn get_backend<T: Hook<D>, D>(
        &self,
        id: egui::Id,
        index: usize,
    ) -> Option<(T::Backend, BoxedDeps)> {
        let backend = self
            .backends
            .write()
            .get_mut(&id)
            .and_then(|backends| backends.remove(&index));
        if let Some(backend) = backend {
            if backend.type_id == TypeId::of::<T::Backend>() {
                return Some((
                    *backend.value.downcast::<T::Backend>().unwrap(),
                    backend.deps,
                ));
            } else {
                panic!(
                    "Backend type mismatch for hook (expected {:?}, got {:?}). May be caused by a the order of hooks being different between frames.",
                    TypeId::of::<T::Backend>(),
                    backend.type_id
                );
            }
        }
        None
    }

    #[inline]
    pub(crate) fn push_backend<T: Hook<D>, D>(
        &self,
        id: egui::Id,
        index: usize,
        backend: T::Backend,
        deps: BoxedDeps,
    ) {
        self.backends.write().entry(id).or_default().insert(
            index,
            Backend {
                type_id: TypeId::of::<T::Backend>(),
                value: Box::new(backend),
                deps,
            },
        );
    }

    #[inline]
    pub(crate) fn register_cleanup(&self, id: egui::Id, cleanup: Box<dyn Cleanup>) {
        self.backends.write().register_boxed_cleanup(id, cleanup)
    }

    #[inline]
    pub(crate) fn get_kv_or_default<K: Send + Sync + 'static, V: Send + Sync + 'static>(
        &self,
    ) -> Arc<RwLock<HashMap<K, V>>> {
        self.kvs
            .write()
            .entry((TypeId::of::<K>(), TypeId::of::<V>()))
            .or_insert_with(|| Box::new(Arc::new(RwLock::new(HashMap::<K, V>::default()))))
            .downcast_ref::<Arc<RwLock<HashMap<K, V>>>>()
            .unwrap()
            .clone()
    }

    #[inline]
    pub(crate) fn get_persisted_kv_or_default<
        K: SerializableAny + Eq + std::hash::Hash,
        V: SerializableAny,
    >(
        &self,
        ctx: &egui::Context,
    ) -> Arc<RwLock<HashMap<K, V>>> {
        self.persisted_kvs
            .write()
            .entry((TypeId::of::<K>(), TypeId::of::<V>()))
            .or_insert_with(|| {
                // Clone from egui data
                ctx.data_mut(|data| {
                    Box::new(
                        data.get_persisted_mut_or_insert_with::<Arc<RwLock<HashMap<K, V>>>>(
                            egui::Id::new((TypeId::of::<K>(), TypeId::of::<V>())),
                            || Arc::new(RwLock::new(HashMap::<K, V>::default())),
                        )
                        .clone(),
                    )
                })
            })
            .downcast_ref::<Arc<RwLock<HashMap<K, V>>>>()
            .unwrap()
            .clone()
    }
}
