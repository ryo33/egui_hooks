use std::sync::Arc;

use egui::util::id_type_map::SerializableAny;

use super::{
    state::{State, StateBackend, StateHookInner},
    Hook,
};

pub struct PersistentStateHook<T: 'static> {
    inner: StateHookInner<T>,
}

impl<T> PersistentStateHook<T> {
    pub fn new(default: T) -> Self {
        Self {
            inner: StateHookInner::Default(default),
        }
    }
}

impl<T: SerializableAny> Hook for PersistentStateHook<T> {
    type Backend = StateBackend<T>;
    type Output = State<T>;

    fn init(&mut self, index: usize, ui: &mut egui::Ui) -> Self::Backend {
        let key = ui.id().with(("persistent state", index));
        ui.data_mut(|data| {
            data.get_persisted_mut_or_insert_with::<StateBackend<T>>(key, || {
                StateBackend::new(Arc::new(self.inner.take()), None)
            })
            .clone()
        })
        .clone()
    }

    fn hook(self, backend: &mut Self::Backend, _ui: &mut egui::Ui) -> Self::Output {
        State::new(backend)
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
        data.insert_persisted::<StateBackend<i32>>(
            id.with(("persistent state", 0)),
            StateBackend::new(Arc::new(12345), None),
        );
    });
    egui::containers::Area::new("test").show(&ctx, |ui| {
        let mut hook = PersistentStateHook::new(42);
        let mut backend = hook.init(0, ui);
        let state = hook.hook(&mut backend, ui);
        assert_eq!(get_persisted::<i32>(0, ui), 12345);
        assert_eq!(*state, 12345);
        state.set_next(43);
        assert_eq!(get_persisted::<i32>(0, ui), 43);
    });
}

#[cfg(test)]
fn get_persisted<T: SerializableAny>(index: usize, ui: &mut egui::Ui) -> T {
    ui.data_mut(|data| {
        data.get_persisted::<StateBackend<T>>(ui.id().with(("persistent state", index)))
            .unwrap()
    })
    .load()
    .current
    .as_ref()
    .clone()
}
