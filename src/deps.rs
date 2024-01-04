use dyn_clone::DynClone;
use std::any::Any;

pub trait Deps: Clone + PartialEq + Send + Sync + 'static {}

impl Deps for () {}

macro_rules! impl_deps {
    ($($name:ident),*) => {
        impl<$($name: Clone + PartialEq + Send + Sync + 'static),*> Deps for ($($name,)*) {}
    };
}
impl_deps!(A);
impl_deps!(A, B);
impl_deps!(A, B, C);
impl_deps!(A, B, C, D);
impl_deps!(A, B, C, D, E);
impl_deps!(A, B, C, D, E, F);
impl_deps!(A, B, C, D, E, F, G);
impl_deps!(A, B, C, D, E, F, G, H);
impl_deps!(A, B, C, D, E, F, G, H, I);
impl_deps!(A, B, C, D, E, F, G, H, I, J);
impl_deps!(A, B, C, D, E, F, G, H, I, J, K);
impl_deps!(A, B, C, D, E, F, G, H, I, J, K, L);

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
