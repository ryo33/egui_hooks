use dyn_clone::DynClone;
use std::any::Any;

pub trait Deps: Clone + PartialEq + Send + Sync + 'static {}

impl<T: Clone + PartialEq + Send + Sync + 'static> Deps for T {}

pub trait DynDeps: DynClone + Send + Sync + 'static {
    fn partial_eq(&self, other: &dyn Any) -> bool;
}

impl<T: Clone + PartialEq + Send + Sync + 'static> DynDeps for T {
    fn partial_eq(&self, other: &dyn Any) -> bool {
        if let Some(other) = other.downcast_ref::<T>() {
            self == other
        } else {
            false
        }
    }
}

dyn_clone::clone_trait_object!(DynDeps);
