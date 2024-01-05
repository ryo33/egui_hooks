use std::{any::Any, sync::Arc};

use arc_swap::ArcSwap;

use super::Hook;

pub struct StateHook<T: Any> {
    pub(crate) default: T,
}

impl<T> Hook for StateHook<T>
where
    T: Any + Clone + Send + Sync,
{
    type Backend = Arc<ArcSwap<T>>;
    type Output = (Arc<T>, Box<dyn Fn(T)>);
    fn init(&mut self, _ui: &mut egui::Ui) -> Self::Backend {
        Arc::new(Arc::new(self.default.clone()).into())
    }
    fn hook(self, backend: &mut Self::Backend, _ui: &mut egui::Ui) -> Self::Output {
        let backend = backend.clone();
        (
            backend.load().clone(),
            Box::new(move |value| {
                backend.store(Arc::new(value));
            }),
        )
    }
}
