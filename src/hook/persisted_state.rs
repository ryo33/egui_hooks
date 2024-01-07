use std::sync::Arc;

use egui::util::id_type_map::SerializableAny;

use crate::deps::Deps;

use super::{
    state::{State, StateBackend, StateHookInner},
    Hook,
};

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

impl<T: SerializableAny, F: FnOnce() -> T, D: Deps> Hook<D> for PersistedStateHook<F> {
    type Backend = StateBackend<T>;
    type Output = State<T>;

    #[inline]
    fn init(
        &mut self,
        index: usize,
        _deps: &D,
        backend: Option<Self::Backend>,
        ui: &mut egui::Ui,
    ) -> Self::Backend {
        let init = Arc::new(self.inner.take()());
        if let Some(backend) = backend {
            let guard = backend.load();
            backend.store(init, Some(guard.current.clone()));
            backend
        } else {
            let key = ui.id().with(("persisted state", index));
            ui.data_mut(|data| {
                // TODO: use TwoFrameMap instead of directly using IdTypeMap
                data.get_persisted_mut_or_insert_with::<StateBackend<T>>(key, || {
                    StateBackend::new(init, None)
                })
                .clone()
            })
            .clone()
        }
    }

    #[inline]
    fn hook(self, backend: &mut Self::Backend, _ui: &mut egui::Ui) -> Self::Output {
        State::new(backend)
    }
}

#[test]
fn test_saved_on_init() {
    let ctx = egui::Context::default();
    egui::containers::Area::new("test").show(&ctx, |ui| {
        let mut hook = PersistedStateHook::new(|| 42);
        hook.init(0, &(), None, ui);
        assert_eq!(get_persisted::<i32>(0, ui), 42);
    });
}

#[test]
fn test_saved_on_set_next() {
    let ctx = egui::Context::default();
    egui::containers::Area::new("test").show(&ctx, |ui| {
        let mut hook = PersistedStateHook::new(|| 42);
        let mut backend = hook.init(0, &(), None, ui);
        let state = Hook::<()>::hook(hook, &mut backend, ui);
        state.set_next(43);
        assert_eq!(get_persisted::<i32>(0, ui), 43);
    });
}

#[test]
fn no_deadlock() {
    let ctx = egui::Context::default();
    egui::containers::Area::new("test").show(&ctx, |ui| {
        let mut hook = PersistedStateHook::new(|| 42);
        let mut backend = hook.init(0, &(), None, ui);
        let state = Hook::<()>::hook(hook, &mut backend, ui);
        // try to lock the data in locking data
        ui.data_mut(|_data| {
            state.set_next(43);
        });
        assert_eq!(get_persisted::<i32>(0, ui), 43);
    });
}

#[test]
fn use_persisted_value_on_init() {
    let ctx = egui::Context::default();
    let id = egui::Id::new("test");
    ctx.data_mut(|data| {
        data.insert_persisted::<StateBackend<i32>>(
            id.with(("persisted state", 0)),
            StateBackend::new(Arc::new(12345), None),
        );
    });
    egui::containers::Area::new("test").show(&ctx, |ui| {
        let mut hook = PersistedStateHook::new(|| 42);
        let mut backend = hook.init(0, &(), None, ui);
        let state = Hook::<()>::hook(hook, &mut backend, ui);
        assert_eq!(get_persisted::<i32>(0, ui), 12345);
        assert_eq!(*state, 12345);
        state.set_next(43);
        assert_eq!(get_persisted::<i32>(0, ui), 43);
    });
}

#[test]
fn init_with_last_backend_updates_with_new_default_value() {
    let ctx = egui::Context::default();
    let id = egui::Id::new("test");
    let backend = StateBackend::new(Arc::new(12345), None);
    ctx.data_mut(|data| {
        data.insert_persisted::<StateBackend<i32>>(
            id.with(("persisted state", 0)),
            backend.clone(),
        );
    });
    egui::containers::Area::new("test").show(&ctx, |ui| {
        let mut hook = PersistedStateHook::new(|| 42);
        let mut backend = hook.init(0, &(), Some(backend), ui);
        let state = Hook::<()>::hook(hook, &mut backend, ui);
        assert_eq!(get_persisted::<i32>(0, ui), 42);
        assert_eq!(*state, 42);
        assert_eq!(state.previous(), Some(&12345));
    });
}

#[cfg(test)]
fn get_persisted<T: SerializableAny>(index: usize, ui: &mut egui::Ui) -> T {
    ui.data_mut(|data| {
        data.get_persisted::<StateBackend<T>>(ui.id().with(("persisted state", index)))
            .unwrap()
    })
    .load()
    .current
    .as_ref()
    .clone()
}
