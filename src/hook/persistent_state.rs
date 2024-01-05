use std::sync::Arc;

use arc_swap::ArcSwap;
use egui::util::id_type_map::SerializableAny;

use super::{
    state::{State, StateBackend, StateHook},
    Hook,
};

pub struct PersistentStateHook<T: 'static> {
    inner: StateHook<T>,
}

impl<T> PersistentStateHook<T> {
    pub fn new(default: T) -> Self {
        Self {
            inner: StateHook::new(default),
        }
    }
}

pub struct PersistentStateBackend<T> {
    pub(crate) inner: Arc<ArcSwap<StateBackend<T>>>,
    pub(crate) persisted: Arc<ArcSwap<T>>,
}

impl<T: SerializableAny> Hook for PersistentStateHook<T> {
    type Backend = PersistentStateBackend<T>;
    type Output = State<T>;

    fn init(&mut self, index: usize, ui: &mut egui::Ui) -> Self::Backend {
        let key = ui.id().with(("persistent state", index));
        let backend = self.inner.init(index, ui);
        let persisted = ui.data_mut(|data| {
            data.get_persisted_mut_or_insert_with::<Arc<ArcSwap<T>>>(key, || {
                Arc::new(ArcSwap::from(backend.load().current.clone()))
            })
            .clone()
        });
        PersistentStateBackend {
            inner: backend,
            persisted,
        }
    }

    fn hook(self, backend: &mut Self::Backend, ui: &mut egui::Ui) -> Self::Output {
        let mut state = self.inner.hook(&mut backend.inner, ui);
        let persisted = backend.persisted.clone();
        state.subscribe(move |next| {
            persisted.store(next.clone());
        });
        state
    }
}

#[test]
fn test_saved_on_init() {
    let ctx = egui::Context::default();
    egui::containers::Area::new("test").show(&ctx, |ui| {
        let mut hook = PersistentStateHook::new(42);
        hook.init(0, ui);
        assert_eq!(get_persisted::<i32>(0, ui), 42);
    });
}

#[test]
fn test_saved_on_set_next() {
    let ctx = egui::Context::default();
    egui::containers::Area::new("test").show(&ctx, |ui| {
        let mut hook = PersistentStateHook::new(42);
        let mut backend = hook.init(0, ui);
        let state = hook.hook(&mut backend, ui);
        state.set_next(43);
        assert_eq!(get_persisted::<i32>(0, ui), 43);
    });
}

#[test]
fn no_deadlock() {
    let ctx = egui::Context::default();
    egui::containers::Area::new("test").show(&ctx, |ui| {
        let mut hook = PersistentStateHook::new(42);
        let mut backend = hook.init(0, ui);
        let state = hook.hook(&mut backend, ui);
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
        data.insert_persisted::<Arc<ArcSwap<i32>>>(
            id.with(("persistent state", 0)),
            Arc::new(ArcSwap::from(Arc::new(12345))),
        );
    });
    egui::containers::Area::new("test").show(&ctx, |ui| {
        let mut hook = PersistentStateHook::new(42);
        let mut backend = hook.init(0, ui);
        let state = hook.hook(&mut backend, ui);
        assert_eq!(get_persisted::<i32>(0, ui), 12345);
        state.set_next(43);
        assert_eq!(get_persisted::<i32>(0, ui), 43);
    });
}

#[cfg(test)]
fn get_persisted<T: SerializableAny>(index: usize, ui: &mut egui::Ui) -> T {
    ui.data_mut(|data| {
        data.get_persisted::<Arc<ArcSwap<T>>>(ui.id().with(("persistent state", index)))
            .unwrap()
    })
    .as_ref()
    .load()
    .as_ref()
    .clone()
}
