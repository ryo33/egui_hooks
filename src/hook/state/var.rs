use std::ops::{Deref, DerefMut};

use super::State;

/// A version of `State` that can be used like a normal variable. It sends the next value on drop.
///
/// This struct is not `Clone` because it would lead multiple dirty states, so use `var.state()` to
/// get the cloned value of the internal state.
pub struct Var<T> {
    // Option is used to drop the value.
    current: Option<T>,
    state: State<T>,
}

super::macros::state_derive!(Var);

impl<T: Clone> From<State<T>> for Var<T> {
    #[inline]
    fn from(value: State<T>) -> Self {
        Self {
            current: Some(value.current.as_ref().clone()),
            state: value,
        }
    }
}

impl<T> Deref for Var<T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &Self::Target {
        self.current.as_ref().unwrap()
    }
}

impl<T> DerefMut for Var<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.current.as_mut().unwrap()
    }
}

impl<T> Var<T> {
    /// Clone the state and return it.
    #[inline]
    pub fn state(&self) -> State<T> {
        self.state.clone()
    }

    #[inline]
    pub fn previous(&self) -> Option<&T> {
        self.state.previous()
    }
}

impl<T> Drop for Var<T> {
    #[inline]
    fn drop(&mut self) {
        let next = self.current.take().unwrap();
        self.state.set_next(next);
    }
}

#[test]
fn test_drop() {
    let ctx = egui::Context::default();
    let _ = ctx.run(Default::default(), |ctx| {
        egui::Area::new("test".into()).show(ctx, |ui| {
            use crate::UseHookExt as _;
            let mut var = ui.use_state(|| 42, ()).into_var();
            let state = var.state();
            *var = 43;
            assert_eq!(*state.set_state.backend.load().current, 42);
            drop(var);
            assert_eq!(*state.set_state.backend.load().current, 43);
        });
    });
}
