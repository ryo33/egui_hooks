use std::{
    any::{Any, TypeId},
    collections::BTreeMap,
    sync::Arc,
};

use egui::mutex::RwLock;

use crate::{cleanup::Cleanup, deps::BoxedDeps, hook::Hook, two_frame_map::TwoFrameMap};

#[derive(Default)]
pub struct Dispatcher {
    // Option<Backend> is used to allow `Option::take` to get owned value from vec without changing
    // the length of the vec or cloning the value.
    inner: RwLock<TwoFrameMap<egui::Id, BTreeMap<usize, Backend>>>,
}

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
    pub(crate) fn from_ui(ui: &egui::Ui) -> Arc<Self> {
        ui.data_mut(|data| {
            data.get_temp_mut_or_default::<Arc<Dispatcher>>(egui::Id::NULL)
                .clone()
        })
    }

    #[inline]
    pub(crate) fn may_advance_frame(&self, frame_nr: u64) {
        self.inner.write().may_advance_frame(frame_nr);
    }

    #[inline]
    pub(crate) fn get_backend<T: Hook<D>, D>(
        &self,
        id: egui::Id,
        index: usize,
    ) -> Option<(T::Backend, BoxedDeps)> {
        let backend = self
            .inner
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
        self.inner.write().entry(id).or_default().insert(
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
        self.inner.write().register_boxed_cleanup(id, cleanup)
    }
}
