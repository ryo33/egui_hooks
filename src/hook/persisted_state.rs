use std::sync::Arc;

use egui::util::id_type_map::SerializableAny;
use parking_lot::RwLock;

use crate::{deps::Deps, dispatcher::Dispatcher, two_frame_map::TwoFrameMap};

use super::{
    state::{State, StateBackend, StateHookInner},
    Hook,
};

/// A persisted version of `StateHook`. It will free the persisted value if it's not used for 2 frames in best effort.
/// The "best effort" means that if no one uses the `PersistedStateHook<T>` with the same `T`, the freeing is postponed until the next time someone uses it.
pub struct PersistedStateHook<T> {
    inner: StateHookInner<T>,
}

impl<T, F: FnOnce() -> T> PersistedStateHook<F> {
    #[inline]
    pub fn new(default: F) -> Self {
        Self {
            inner: StateHookInner::Default(default),
        }
    }
}

type PersistedTwoFrameMap<T> = Arc<RwLock<TwoFrameMap<(egui::Id, usize), StateBackend<T>>>>;
pub struct PersistedStateBackend<T> {
    kv: PersistedTwoFrameMap<T>,
    inner: StateBackend<T>,
    index: usize,
}

impl<T: SerializableAny, F: FnOnce() -> T, D: Deps> Hook<D> for PersistedStateHook<F> {
    type Backend = PersistedStateBackend<T>;
    type Output = State<T>;

    #[inline]
    fn init(
        &mut self,
        index: usize,
        _deps: &D,
        backend: Option<Self::Backend>,
        ui: &mut egui::Ui,
    ) -> Self::Backend {
        let default = Arc::new((self.inner.take())());
        let backend = if let Some(backend) = backend {
            let previous = backend.inner.load().current.clone();
            println!("updated ");
            backend.inner.store(default, Some(previous));
            backend
        } else {
            let kv = Dispatcher::from_ctx(ui.ctx())
                .get_persisted_kv_or_default::<(), PersistedTwoFrameMap<T>>(ui.ctx())
                .write()
                .entry(())
                .or_default()
                .clone();
            // Use the persisted backend if it exists
            let backend = kv
                .write()
                .entry((ui.id(), index))
                .or_insert_with(|| StateBackend::new(default, None))
                .clone();
            PersistedStateBackend {
                kv,
                inner: backend,
                index,
            }
        };
        backend
    }

    #[inline]
    fn hook(self, backend: &mut Self::Backend, ui: &mut egui::Ui) -> Self::Output {
        let mut lock = backend.kv.write();
        // Don't forget to advance frame
        lock.may_advance_frame(ui.ctx().cumulative_pass_nr());
        // This `or_insert_with` is theoretically never called because the outer backend in
        // the dispatcher has longer lifetime than internal one.
        // Always: dispatcher.get_backend -> this line -> dispatcher.get_backend -> this line
        let state = lock
            .entry((ui.id(), backend.index))
            .or_insert_with(|| backend.inner.clone())
            .clone();
        State::new(&state)
    }
}

#[test]
fn test_saved_on_init() {
    let ctx = egui::Context::default();
    let _ = ctx.run(Default::default(), |ctx| {
        egui::containers::Area::new("test".into()).show(ctx, |ui| {
            let mut hook = PersistedStateHook::new(|| 42);
            hook.init(0, &(), None, ui);
            assert_eq!(get_persisted::<i32>(0, &ctx, "test"), Some(42));
        });
    });
}

#[test]
fn test_saved_on_set_next() {
    let ctx = egui::Context::default();
    let _ = ctx.run(Default::default(), |ctx| {
        egui::containers::Area::new("test".into()).show(ctx, |ui| {
            let mut hook = PersistedStateHook::new(|| 42);
            let mut backend = hook.init(0, &(), None, ui);
            let state = Hook::<()>::hook(hook, &mut backend, ui);
            state.set_next(43);
            assert_eq!(get_persisted::<i32>(0, &ctx, "test"), Some(43));
        });
    });
}

#[test]
fn no_deadlock() {
    let ctx = egui::Context::default();
    let _ = ctx.run(Default::default(), |ctx| {
        egui::containers::Area::new("test".into()).show(ctx, |ui| {
            let mut hook = PersistedStateHook::new(|| 42);
            let mut backend = hook.init(0, &(), None, ui);
            let state = Hook::<()>::hook(hook, &mut backend, ui);
            // try to lock the data in locking data
            ui.data_mut(|_data| {
                state.set_next(43);
            });
            assert_eq!(get_persisted::<i32>(0, &ctx, "test"), Some(43));
        });
    });
}

#[test]
fn use_persisted_value_on_init() {
    let ctx = egui::Context::default();
    set_persisted(0, &ctx, StateBackend::new(Arc::new(12345), None), "test");
    let _ = ctx.run(Default::default(), |ctx| {
        egui::containers::Area::new("test".into()).show(ctx, |ui| {
            let mut hook = PersistedStateHook::new(|| 42);
            let mut backend = hook.init(0, &(), None, ui);
            let state = Hook::<()>::hook(hook, &mut backend, ui);
            assert_eq!(get_persisted::<i32>(0, &ctx, "test"), Some(12345));
            assert_eq!(*state, 12345);
            state.set_next(43);
            assert_eq!(get_persisted::<i32>(0, &ctx, "test"), Some(43));
        });
    });
}

#[test]
fn init_with_last_backend_updates_with_new_default_value() {
    let ctx = egui::Context::default();
    let inner = StateBackend::new(Arc::new(12345), None);

    let mut backend = Some(set_persisted(0, &ctx, inner.clone(), "test"));

    let _ = ctx.run(Default::default(), move |ctx| {
        egui::containers::Area::new("test".into()).show(ctx, |ui| {
            let mut hook = PersistedStateHook::new(|| 42);
            backend = Some(hook.init(0, &(), backend.take(), ui));

            let state = Hook::<()>::hook(hook, &mut backend.as_mut().unwrap(), ui);
            assert_eq!(get_persisted::<i32>(0, &ctx, "test"), Some(42));
            assert_eq!(*state, 42);
            assert_eq!(state.previous(), Some(&12345));
        });
    });
}

#[test]
fn cleanup() {
    let ctx = egui::Context::default();

    let _ = ctx.run(Default::default(), |ctx| {
        egui::Area::new("test".into()).show(ctx, |ui| {
            let mut hook = PersistedStateHook::new(|| 42);
            let mut backend = hook.init(0, &(), None, ui);
            let state = Hook::<()>::hook(hook, &mut backend, ui);
            assert_eq!(*state, 42);
            assert_eq!(get_persisted::<i32>(0, ctx, "test"), Some(42));
        });
    });

    let _ = ctx.run(Default::default(), |ctx| {
        // ensure the advance of frame
        egui::Area::new("test2".into()).show(ctx, |ui| {
            use crate::UseHookExt;
            ui.use_persisted_state(|| 0, ());
        });
        // Not cleaned since this is the second frame
        assert_eq!(get_persisted::<i32>(0, ctx, "test"), Some(42));
    });

    let _ = ctx.run(Default::default(), |ctx| {
        // ensure the advance of frame
        egui::Area::new("test2".into()).show(ctx, |ui| {
            use crate::UseHookExt;
            ui.use_persisted_state(|| 0, ());
        });
        // Cleaned since this is the third frame
        assert!(get_persisted::<i32>(0, ctx, "test").is_none());
    });
}

#[cfg(test)]
fn set_persisted<T: SerializableAny>(
    index: usize,
    ctx: &egui::Context,
    backend: StateBackend<T>,
    id: &str,
) -> PersistedStateBackend<T> {
    let kv = Dispatcher::from_ctx(ctx)
        .get_persisted_kv_or_default::<(), PersistedTwoFrameMap<T>>(ctx)
        .write()
        .entry(())
        .or_default()
        .clone();
    kv.write()
        .insert((egui::Id::new(id), index), backend.clone());
    PersistedStateBackend {
        kv,
        inner: backend,
        index,
    }
}

#[cfg(test)]
fn get_persisted<T: SerializableAny>(index: usize, ctx: &egui::Context, id: &str) -> Option<T> {
    Dispatcher::from_ctx(ctx)
        .get_persisted_kv_or_default::<(), PersistedTwoFrameMap<T>>(ctx)
        .read()
        .get(&())
        .unwrap()
        .write()
        .peek(&(egui::Id::new(id), index))
        .map(|backend| backend.load().current.as_ref().clone())
}
