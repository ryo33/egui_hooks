mod macros;
mod var;

pub use var::Var;

use std::{any::Any, sync::Arc};

use arc_swap::ArcSwap;

use crate::deps::Deps;

use super::Hook;

pub struct StateHook<F> {
    inner: StateHookInner<F>,
}

impl<T, F: FnOnce() -> T> StateHook<F> {
    #[inline]
    pub fn new(default: F) -> Self {
        Self {
            inner: StateHookInner::Default(default),
        }
    }
}

pub(crate) enum StateHookInner<T> {
    Default(T),
    Taken,
}

impl<F> StateHookInner<F> {
    #[inline]
    pub fn take(&mut self) -> F {
        match std::mem::replace(self, StateHookInner::Taken) {
            StateHookInner::Default(default) => default,
            StateHookInner::Taken => panic!("StateHook::init called twice?"),
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StateBackend<T> {
    inner: Arc<ArcSwap<StateBackendInner<T>>>,
}

impl<T> StateBackend<T> {
    #[inline]
    pub(crate) fn new(current: Arc<T>, previous: Option<Arc<T>>) -> Self {
        Self {
            inner: Arc::new(ArcSwap::from(Arc::new(StateBackendInner {
                current,
                previous,
            }))),
        }
    }

    #[inline]
    pub(crate) fn load(&self) -> impl std::ops::Deref<Target = Arc<StateBackendInner<T>>> {
        self.inner.load()
    }

    #[inline]
    pub(crate) fn store(&self, current: Arc<T>, previous: Option<Arc<T>>) {
        self.inner
            .store(Arc::new(StateBackendInner { current, previous }));
    }

    #[inline]
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
    #[inline]
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StateBackendInner<T> {
    pub(crate) current: Arc<T>,
    pub(crate) previous: Option<Arc<T>>,
}

impl<T, F: FnOnce() -> T, D: Deps> Hook<D> for StateHook<F>
where
    T: Any + Send + Sync,
{
    type Backend = StateBackend<T>;
    type Output = State<T>;
    #[inline]
    fn init(
        &mut self,
        _index: usize,
        _deps: &D,
        backend: Option<Self::Backend>,
        _ui: &mut egui::Ui,
    ) -> Self::Backend {
        let init = Arc::new(self.inner.take()());
        if let Some(backend) = backend {
            let guard = backend.load();
            backend.store(init, Some(guard.current.clone()));
            backend
        } else {
            StateBackend::new(init, None)
        }
    }
    #[inline]
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
    set_state: SetState<T>,
}

pub struct SetState<T> {
    _phantom: std::marker::PhantomData<T>,
    // This field is required to save the previous value of the state.
    current: Arc<T>,
    backend: StateBackend<T>,
}

#[test]
fn test_state_is_send_sync() {
    fn assert_send<T: Send + Sync>() {}
    assert_send::<State<i32>>();
    assert_send::<SetState<i32>>();
}

impl<T> SetState<T> {
    #[inline]
    pub(crate) fn new(current: Arc<T>, backend: StateBackend<T>) -> Self {
        Self {
            _phantom: std::marker::PhantomData,
            current,
            backend,
        }
    }

    /// Set the next value of the state that will be used in the next frame.
    #[inline]
    pub fn set_next(&self, next: T) {
        self.backend
            .store(Arc::new(next), Some(self.current.clone()));
    }

    /// Set the next value of the state with a function that takes the current state or the next
    /// state if already set in the current frame with `set_next` or `update_next`.
    #[inline]
    pub fn update_next(&self, f: impl Fn(&T) -> T) {
        self.backend.rcu(f, Some(self.current.clone()));
    }
}

impl<T> State<T> {
    #[inline]
    pub(crate) fn new(backend: &StateBackend<T>) -> Self {
        let guard = backend.load();
        Self {
            current: guard.current.clone(),
            previous: guard.previous.clone(),
            set_state: SetState::new(guard.current.clone(), backend.clone()),
        }
    }

    /// Get the previous value of the state. This is useful for using in effect hooks.
    /// Even if set_state is called multiple times in a frame, this returns the value at the
    /// previous `use_state`. This helps to do some cleanup depending on the previous state in
    /// `use_effect`.
    #[inline]
    pub fn previous(&self) -> Option<&T> {
        self.previous.as_deref()
    }

    /// Set the next value of the state that will be used in the next frame.
    #[inline]
    pub fn set_next(&self, next: T) {
        self.set_state.set_next(next);
    }

    /// Set the next value of the state with a function that takes the current state or the next
    /// state if already set in the current frame with `set_next` or `update_next`.
    #[inline]
    pub fn update_next(&self, f: impl Fn(&T) -> T) {
        self.set_state.update_next(f);
    }

    /// Get variable-like state. `var.set_next()` is required to update the next state.
    #[inline]
    pub fn into_var(self) -> Var<T>
    where
        T: Clone,
    {
        self.into()
    }
}

impl<T> Clone for State<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            current: self.current.clone(),
            previous: self.previous.clone(),
            set_state: self.set_state.clone(),
        }
    }
}

impl<T> Clone for SetState<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            _phantom: std::marker::PhantomData,
            current: self.current.clone(),
            backend: self.backend.clone(),
        }
    }
}

impl<T> std::ops::Deref for State<T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.current
    }
}

macros::state_derive!(State);

#[test]
fn test_not_clonable_state() {
    struct NotClonable;
    let ctx = egui::Context::default();
    let _ = ctx.run(Default::default(), |ctx| {
        egui::Area::new("test".into()).show(ctx, |ui| {
            let mut hook = StateHook::new(|| NotClonable);
            let mut backend = hook.init(0, &(), None, ui);
            let state = Hook::<()>::hook(hook, &mut backend, ui);
            let _ = *state;
            let _ = state.previous();
        });
    });
}

#[test]
fn test_default_state() {
    let ctx = egui::Context::default();
    let _ = ctx.run(Default::default(), |ctx| {
        egui::Area::new("test".into()).show(ctx, |ui| {
            let mut hook = StateHook::new(|| 42);
            let mut backend = hook.init(0, &(), None, ui);
            let state = Hook::<()>::hook(hook, &mut backend, ui);
            assert_eq!(*state, 42);
            assert_eq!(state.previous(), None);
        });
    });
}

#[test]
fn test_set_new_state() {
    let ctx = egui::Context::default();
    let _ = ctx.run(Default::default(), |ctx| {
        egui::Area::new("test".into()).show(ctx, |ui| {
            let mut hook = StateHook::new(|| 42);
            let mut backend = hook.init(0, &(), None, ui);
            let state = Hook::<()>::hook(hook, &mut backend, ui);
            assert_eq!(*state, 42);
            assert_eq!(state.previous(), None);
            state.set_next(43);
            let hook = StateHook::new(|| 42);
            let state = Hook::<()>::hook(hook, &mut backend, ui);
            assert_eq!(*state, 43);
            assert_eq!(state.previous(), Some(&42));
        });
    });
}

#[test]
fn test_previous_value_with_multiple_set() {
    let ctx = egui::Context::default();
    let _ = ctx.run(Default::default(), |ctx| {
        egui::Area::new("test".into()).show(ctx, |ui| {
            let mut hook = StateHook::new(|| 42);
            let mut backend = hook.init(0, &(), None, ui);
            let state = Hook::<()>::hook(hook, &mut backend, ui);
            assert_eq!(*state, 42);
            assert_eq!(state.previous(), None);
            state.set_next(43);
            state.set_next(44);
            let hook = StateHook::new(|| 42);
            let state = Hook::<()>::hook(hook, &mut backend, ui);
            assert_eq!(*state, 44);
            // not 43
            assert_eq!(state.previous(), Some(&42));
        });
    });
}

#[test]
fn test_update_next() {
    let ctx = egui::Context::default();
    let _ = ctx.run(Default::default(), |ctx| {
        egui::Area::new("test".into()).show(ctx, |ui| {
            let mut hook = StateHook::new(|| 42);
            let mut backend = hook.init(0, &(), None, ui);
            let state = Hook::<()>::hook(hook, &mut backend, ui);

            state.update_next(|x| x + 1);
            state.update_next(|x| x + 1);
            state.update_next(|x| x + 1);

            let hook = StateHook::new(|| 42);
            let state = Hook::<()>::hook(hook, &mut backend, ui);
            assert_eq!(*state, 45);
            // not 43 or 44
            assert_eq!(state.previous(), Some(&42));
        });
    });
}

#[test]
fn test_update_can_use_value_from_set_next() {
    let ctx = egui::Context::default();
    let _ = ctx.run(Default::default(), |ctx| {
        egui::Area::new("test".into()).show(ctx, |ui| {
            let mut hook = StateHook::new(|| 42);
            let mut backend = hook.init(0, &(), None, ui);
            let state = Hook::<()>::hook(hook, &mut backend, ui);

            state.set_next(100);
            state.update_next(|x| x + 1);

            let hook = StateHook::new(|| 42);
            let state = Hook::<()>::hook(hook, &mut backend, ui);
            assert_eq!(*state, 101);
            assert_eq!(state.previous(), Some(&42));
        });
    });
}

#[test]
fn use_previous_backend_on_init() {
    let ctx = egui::Context::default();
    let _ = ctx.run(Default::default(), |ctx| {
        egui::Area::new("test".into()).show(ctx, |ui| {
            let mut hook = StateHook::new(|| 42);
            let mut backend = hook.init(0, &(), Some(StateBackend::new(Arc::new(100), None)), ui);
            let state = Hook::<()>::hook(hook, &mut backend, ui);
            assert_eq!(*state, 42);
            assert_eq!(state.previous(), Some(&100));
        });
    });
}
