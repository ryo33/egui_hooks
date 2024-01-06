use std::any::Any;

pub trait Deps: Any + PartialEq + Send + Sync {
    fn partial_eq(&self, other: &BoxedDeps) -> bool;
}

impl<T: Any + PartialEq + Send + Sync> Deps for T {
    fn partial_eq(&self, other: &BoxedDeps) -> bool {
        other
            .downcast_ref::<Self>()
            .map_or(false, |other| self == other)
    }
}

pub(crate) type BoxedDeps = Box<dyn Any + Send + Sync>;
