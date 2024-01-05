use super::Hook;

pub struct MemoHook<F> {
    pub callback: F,
}

impl<T: Clone + Send + Sync + 'static, F: FnMut() -> T> Hook for MemoHook<F> {
    type Backend = T;
    type Output = T;
    fn init(&mut self, _ui: &mut egui::Ui) -> Self::Backend {
        (self.callback)()
    }
    fn hook(self, backend: &mut Self::Backend, _ui: &mut egui::Ui) -> Self::Output {
        backend.clone()
    }
}
