use std::any::{Any, TypeId};

use egui::mutex::RwLock;

use crate::{deps::DynDeps, hook::Hook, two_frame_map::TwoFrameMap};

#[derive(Default)]
pub struct Dispatcher {
    // Option<Backend> is used to allow `Option::take` to get owned value from vec without changing
    // the length of the vec or cloning the value.
    inner: RwLock<TwoFrameMap<egui::Id, Vec<Option<Backend>>>>,
}

#[test]
fn dispatcher_is_send_and_sync() {
    fn assert_send_and_sync<T: Send + Sync>() {}
    assert_send_and_sync::<Dispatcher>();
}

struct Backend {
    type_id: TypeId,
    value: Box<dyn Any + Send + Sync>,
    deps: Box<dyn DynDeps>,
}

impl Dispatcher {
    pub(crate) fn may_advance_frame(&self, frame_nr: u64) {
        self.inner.write().may_advance_frame(frame_nr);
    }

    pub(crate) fn get_backend<T: Hook>(
        &self,
        id: egui::Id,
        index: usize,
    ) -> Option<(T::Backend, Box<dyn DynDeps>)> {
        let backend = self
            .inner
            .write()
            .get_mut(&id)
            .and_then(|backends| backends.get_mut(index).and_then(Option::take));
        if let Some(backend) = backend {
            if backend.type_id == TypeId::of::<T::Backend>() {
                return Some((
                    *backend.value.downcast::<T::Backend>().unwrap(),
                    backend.deps,
                ));
            } else {
                #[cfg(debug_assertions)]
                panic!(
                    "Backend type mismatch for hook (expected {:?}, got {:?}). May be caused by a the order of hooks being different between frames.",
                    TypeId::of::<T::Backend>(),
                    backend.type_id
                );
            }
        }
        None
    }

    pub(crate) fn push_backend<T: Hook>(
        &self,
        id: egui::Id,
        backend: T::Backend,
        deps: Box<dyn DynDeps>,
    ) {
        self.inner
            .write()
            .entry(id)
            .or_default()
            .push(Some(Backend {
                type_id: TypeId::of::<T::Backend>(),
                value: Box::new(backend),
                deps,
            }));
    }
}
