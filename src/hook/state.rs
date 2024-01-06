mod macros;
mod var;

pub use var::Var;

use std::{any::Any, sync::Arc};

use arc_swap::ArcSwap;

use crate::deps::Deps;

use super::Hook;

pub struct StateHook<T: 'static> {
    inner: StateHookInner<T>,
}

impl<T> StateHook<T> {
    pub fn new(default: T) -> Self {
        Self {
            inner: StateHookInner::Default(default),
        }
    }
}

pub(crate) enum StateHookInner<T: Any> {
    Default(T),
    Taken,
}

impl<T: 'static> StateHookInner<T> {
    pub fn take(&mut self) -> T {
        match std::mem::replace(self, StateHookInner::Taken) {
            StateHookInner::Default(default) => default,
            StateHookInner::Taken => panic!("StateHook::init called twice?"),
        }
    }
}

#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
pub struct StateBackend<T> {
    inner: Arc<ArcSwap<StateBackendInner<T>>>,
}

impl<T> StateBackend<T> {
    pub(crate) fn new(current: Arc<T>, previous: Option<Arc<T>>) -> Self {
        Self {
            inner: Arc::new(ArcSwap::from(Arc::new(StateBackendInner {
                current,
                previous,
            }))),
        }
    }

    pub(crate) fn load(&self) -> impl std::ops::Deref<Target = Arc<StateBackendInner<T>>> {
        self.inner.load()
    }

    pub(crate) fn store(&self, current: Arc<T>, previous: Option<Arc<T>>) {
        self.inner
            .store(Arc::new(StateBackendInner { current, previous }));
    }

    pub(crate) fn rcu(&self, f: impl Fn(&T) -> T, previous: Option<Arc<T>>) {
        self.inner.rcu(move |inner| {
            Arc::new(StateBackendInner {
                current: Arc::new(f(&inner.current)),
                previous: previous.clone(),
            })
        });
    }
}

impl<T> Clone for StateBackend<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
pub struct StateBackendInner<T> {
    pub(crate) current: Arc<T>,
    pub(crate) previous: Option<Arc<T>>,
}

impl<T, D: Deps> Hook<D> for StateHook<T>
where
    T: Any + Send + Sync,
{
    type Backend = StateBackend<T>;
    type Output = State<T>;
    fn init(&mut self, _index: usize, _deps: &D, _ui: &mut egui::Ui) -> Self::Backend {
        StateBackend::new(Arc::new(self.inner.take()), None)
    }
    fn hook(self, backend: &mut Self::Backend, _ui: &mut egui::Ui) -> Self::Output {
        State::new(backend)
    }
}

// set_next is a method of this struct instead of returns `(State<T>, impl Fn(impl Fn(T) -> T) -> ())`
/// The container of current state and previous state returned by `use_state`. Clone is cheap as
/// `Arc` is used internally. If you want to use the state like normal variable without cloning internal value,
/// you can use `state.into_var()` to get a `VarState` which implements `DerefMut` (but don't forget to call `var.set_next_self()`.
pub struct State<T> {
    current: Arc<T>,
    previous: Option<Arc<T>>,
    backend: StateBackend<T>,
}

#[test]
fn test_state_is_send_sync() {
    fn assert_send<T: Send + Sync>() {}
    assert_send::<State<i32>>();
}

impl<T> State<T> {
    pub(crate) fn new(backend: &mut StateBackend<T>) -> Self {
        let guard = backend.load();
        Self {
            current: guard.current.clone(),
            previous: guard.previous.clone(),
            backend: backend.clone(),
        }
    }

    /// Get the previous value of the state. This is useful for using in effect hooks.
    /// Even if set_state is called multiple times in a frame, this returns the value at the
    /// previous `use_state`. This helps to do some cleanup depending on the previous state in
    /// `use_effect`.
    pub fn previous(&self) -> Option<&T> {
        self.previous.as_deref()
    }

    /// Set the next value of the state that will be used in the next frame.
    pub fn set_next(&self, next: T) {
        self.backend
            .store(Arc::new(next), Some(self.current.clone()));
    }

    /// Set the next value of the state with a function that takes the current state or the next
    /// state if already set in the current frame with `set_next` or `update_next`.
    pub fn update_next(&self, f: impl Fn(&T) -> T) {
        self.backend.rcu(f, Some(self.current.clone()));
    }

    /// Get variable-like state. `var.set_next()` is required to update the next state.
    pub fn into_var(self) -> Var<T>
    where
        T: Clone,
    {
        self.into()
    }
}

impl<T> Clone for State<T> {
    fn clone(&self) -> Self {
        Self {
            current: self.current.clone(),
            previous: self.previous.clone(),
            backend: self.backend.clone(),
        }
    }
}

impl<T> std::ops::Deref for State<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.current
    }
}

macros::state_derive!(State);

#[test]
fn test_not_clonable_state() {
    struct NotClonable;
    let ctx = egui::Context::default();
    egui::Area::new("test").show(&ctx, |ui| {
        let mut hook = StateHook::new(NotClonable);
        let mut backend = hook.init(0, &(), ui);
        let state = Hook::<()>::hook(hook, &mut backend, ui);
        let _ = *state;
        let _ = state.previous();
    });
}

#[test]
fn test_default_state() {
    let ctx = egui::Context::default();
    egui::Area::new("test").show(&ctx, |ui| {
        let mut hook = StateHook::new(42);
        let mut backend = hook.init(0, &(), ui);
        let state = Hook::<()>::hook(hook, &mut backend, ui);
        assert_eq!(*state, 42);
        assert_eq!(state.previous(), None);
    });
}

#[test]
fn test_set_new_state() {
    let ctx = egui::Context::default();
    egui::Area::new("test").show(&ctx, |ui| {
        let mut hook = StateHook::new(42);
        let mut backend = hook.init(0, &(), ui);
        let state = Hook::<()>::hook(hook, &mut backend, ui);
        assert_eq!(*state, 42);
        assert_eq!(state.previous(), None);
        state.set_next(43);
        let hook = StateHook::new(42);
        let state = Hook::<()>::hook(hook, &mut backend, ui);
        assert_eq!(*state, 43);
        assert_eq!(state.previous(), Some(&42));
    });
}

#[test]
fn test_previous_value_with_multiple_set() {
    let ctx = egui::Context::default();
    egui::Area::new("test").show(&ctx, |ui| {
        let mut hook = StateHook::new(42);
        let mut backend = hook.init(0, &(), ui);
        let state = Hook::<()>::hook(hook, &mut backend, ui);
        assert_eq!(*state, 42);
        assert_eq!(state.previous(), None);
        state.set_next(43);
        state.set_next(44);
        let hook = StateHook::new(42);
        let state = Hook::<()>::hook(hook, &mut backend, ui);
        assert_eq!(*state, 44);
        // not 43
        assert_eq!(state.previous(), Some(&42));
    });
}

#[test]
fn test_update_next() {
    let ctx = egui::Context::default();
    egui::Area::new("test").show(&ctx, |ui| {
        let mut hook = StateHook::new(42);
        let mut backend = hook.init(0, &(), ui);
        let state = Hook::<()>::hook(hook, &mut backend, ui);

        state.update_next(|x| x + 1);
        state.update_next(|x| x + 1);
        state.update_next(|x| x + 1);

        let hook = StateHook::new(42);
        let state = Hook::<()>::hook(hook, &mut backend, ui);
        assert_eq!(*state, 45);
        // not 43 or 44
        assert_eq!(state.previous(), Some(&42));
    });
}

#[test]
fn test_update_can_use_value_from_set_next() {
    let ctx = egui::Context::default();
    egui::Area::new("test").show(&ctx, |ui| {
        let mut hook = StateHook::new(42);
        let mut backend = hook.init(0, &(), ui);
        let state = Hook::<()>::hook(hook, &mut backend, ui);

        state.set_next(100);
        state.update_next(|x| x + 1);

        let hook = StateHook::new(42);
        let state = Hook::<()>::hook(hook, &mut backend, ui);
        assert_eq!(*state, 101);
        assert_eq!(state.previous(), Some(&42));
    });
}
