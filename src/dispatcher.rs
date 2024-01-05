use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

use egui::mutex::RwLock;

use crate::{deps::DynDeps, hook::Hook};

#[derive(Default)]
pub struct Dispatcher {
    inner: RwLock<Inner>,
}

#[derive(Default)]
pub struct Inner {
    frame_nr: u64,
    previous: Components,
    current: Components,
}

#[derive(Default)]
struct Components {
    // Option<Backend> is used to allow `Option::take` to get owned value from vec without changing
    // the length of the vec or cloning the value.
    map: HashMap<egui::Id, Vec<Option<Backend>>>,
}

struct Backend {
    type_id: TypeId,
    value: Box<dyn Any + Send + Sync>,
    deps: Box<dyn DynDeps>,
}

impl Dispatcher {
    pub(crate) fn may_advance_frame(&self, frame_nr: u64) {
        let mut inner = self.inner.write();
        if frame_nr != inner.frame_nr {
            inner.frame_nr = frame_nr;
            inner.previous = std::mem::take(&mut inner.current);
        }
    }

    pub(crate) fn get_backend<T: Hook>(
        &self,
        id: egui::Id,
        index: usize,
    ) -> Option<(T::Backend, Box<dyn DynDeps>)> {
        let backend = self
            .inner
            .write()
            .previous
            .map
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
            .current
            .map
            .entry(id)
            .or_default()
            .push(Some(Backend {
                type_id: TypeId::of::<T::Backend>(),
                value: Box::new(backend),
                deps,
            }));
    }
}
