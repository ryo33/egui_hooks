use std::{
    any::Any,
    borrow::Borrow,
    fmt::{Debug, Display, Formatter},
    sync::Arc,
};

use arc_swap::ArcSwap;

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

pub enum StateHookInner<T: Any> {
    Default(T),
    Taken,
}

pub struct StateBackend<T> {
    current: Arc<T>,
    previous: Option<Arc<T>>,
}

impl<T> Hook for StateHook<T>
where
    T: Any + Send + Sync,
{
    type Backend = Arc<ArcSwap<StateBackend<T>>>;
    type Output = State<T>;
    fn init(&mut self, _ui: &mut egui::Ui) -> Self::Backend {
        let inner = std::mem::replace(&mut self.inner, StateHookInner::Taken);
        let default = match inner {
            StateHookInner::Default(default) => default,
            StateHookInner::Taken => panic!("StateHook::init called twice?"),
        };
        let default = Arc::new(default);
        Arc::new(ArcSwap::from(Arc::new(StateBackend {
            current: default.clone(),
            previous: None,
        })))
    }
    fn hook(self, backend: &mut Self::Backend, _ui: &mut egui::Ui) -> Self::Output {
        let guard = backend.load();
        State {
            current: guard.current.clone(),
            previous: guard.previous.clone(),
            backend: backend.clone(),
        }
    }
}

// set_next is a method of this struct instead of returns `(State<T>, impl Fn(impl Fn(T) -> T) -> ())`
/// The container of current state and previous state returned by `use_state`. Clone is cheap as
/// `Arc` is used internally.
pub struct State<T> {
    current: Arc<T>,
    previous: Option<Arc<T>>,
    backend: Arc<ArcSwap<StateBackend<T>>>,
}

impl<T> State<T> {
    /// Get the previous value of the state. This is useful for using in effect hooks.
    /// Even if set_state is called multiple times in a frame, this returns the value at the
    /// previous `use_state`. This helps to do some cleanup depending on the previous state in
    /// `use_effect`.
    pub fn previous(&self) -> Option<&T> {
        self.previous.as_deref()
    }

    /// Set the next value of the state that will be used in the next frame.
    pub fn set_next(&self, next: T) {
        self.backend.store(Arc::new(StateBackend {
            current: Arc::new(next),
            previous: Some(self.current.clone()),
        }));
    }

    /// Set the next value of the state with a function that takes the current state or the next
    /// state if already set in the current frame with `set_next` or `update_next`.
    pub fn update_next(&self, f: impl Fn(&T) -> T) {
        self.backend.rcu(|inner| {
            Arc::new(StateBackend {
                current: Arc::new(f(&inner.current)),
                previous: Some(self.current.clone()),
            })
        });
    }
}

impl<T> std::ops::Deref for State<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.current.deref()
    }
}

impl<T> Borrow<T> for State<T> {
    fn borrow(&self) -> &T {
        self.current.borrow()
    }
}

impl<T> AsRef<T> for State<T> {
    fn as_ref(&self) -> &T {
        self.current.as_ref()
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

impl<T: Display> Display for State<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.current.fmt(f)
    }
}

impl<T: Debug> Debug for State<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.current.fmt(f)
    }
}

impl<T: PartialEq> PartialEq for State<T> {
    fn eq(&self, other: &Self) -> bool {
        self.current.eq(&other.current)
    }
}

impl<T: Eq> Eq for State<T> {}
impl<T: PartialOrd> PartialOrd for State<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.current.partial_cmp(&other.current)
    }
}
impl<T: Ord> Ord for State<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.current.cmp(&other.current)
    }
}

impl<T: std::hash::Hash> std::hash::Hash for State<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.current.hash(state);
    }
}

#[test]
fn test_not_clonable_state() {
    struct NotClonable;
    let ctx = egui::Context::default();
    egui::Area::new("test").show(&ctx, |ui| {
        let mut hook = StateHook::new(NotClonable);
        let mut backend = hook.init(ui);
        let state = hook.hook(&mut backend, ui);
        let _ = *state;
        let _ = state.previous();
    });
}

#[test]
fn test_default_state() {
    let ctx = egui::Context::default();
    egui::Area::new("test").show(&ctx, |ui| {
        let mut hook = StateHook::new(42);
        let mut backend = hook.init(ui);
        let state = hook.hook(&mut backend, ui);
        assert_eq!(*state, 42);
        assert_eq!(state.previous(), None);
    });
}

#[test]
fn test_set_new_state() {
    let ctx = egui::Context::default();
    egui::Area::new("test").show(&ctx, |ui| {
        let mut hook = StateHook::new(42);
        let mut backend = hook.init(ui);
        let state = hook.hook(&mut backend, ui);
        assert_eq!(*state, 42);
        assert_eq!(state.previous(), None);
        state.set_next(43);
        let hook = StateHook::new(42);
        let state = hook.hook(&mut backend, ui);
        assert_eq!(*state, 43);
        assert_eq!(state.previous(), Some(&42));
    });
}

#[test]
fn test_previous_value_with_multiple_set() {
    let ctx = egui::Context::default();
    egui::Area::new("test").show(&ctx, |ui| {
        let mut hook = StateHook::new(42);
        let mut backend = hook.init(ui);
        let state = hook.hook(&mut backend, ui);
        assert_eq!(*state, 42);
        assert_eq!(state.previous(), None);
        state.set_next(43);
        state.set_next(44);
        let hook = StateHook::new(42);
        let state = hook.hook(&mut backend, ui);
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
        let mut backend = hook.init(ui);
        let state = hook.hook(&mut backend, ui);

        state.update_next(|x| x + 1);
        state.update_next(|x| x + 1);
        state.update_next(|x| x + 1);

        let hook = StateHook::new(42);
        let state = hook.hook(&mut backend, ui);
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
        let mut backend = hook.init(ui);
        let state = hook.hook(&mut backend, ui);

        state.set_next(100);
        state.update_next(|x| x + 1);

        let hook = StateHook::new(42);
        let state = hook.hook(&mut backend, ui);
        assert_eq!(*state, 101);
        assert_eq!(state.previous(), Some(&42));
    });
}
