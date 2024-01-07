use super::Hook;

pub struct MemoHook<F> {
    pub callback: F,
}

impl<T: Clone + Send + Sync + 'static, F: FnMut() -> T, D> Hook<D> for MemoHook<F> {
    type Backend = T;
    type Output = T;
    #[inline]
    fn init(
        &mut self,
        _index: usize,
        _deps: &D,
        _backend: Option<Self::Backend>,
        _ui: &mut egui::Ui,
    ) -> Self::Backend {
        (self.callback)()
    }
    #[inline]
    fn hook(self, backend: &mut Self::Backend, _ui: &mut egui::Ui) -> Self::Output {
        backend.clone()
    }
}
