use super::Hook;

pub struct EffectHook<F> {
    pub callback: F,
}

impl<'a, F: FnOnce() + Send + Sync + 'a> Hook for EffectHook<F> {
    type Backend = bool;
    type Output = ();
    fn init(&mut self, _ui: &mut egui::Ui) -> Self::Backend {
        true
    }
    fn hook(self, backend: &mut Self::Backend, _ui: &mut egui::Ui) -> Self::Output {
        if *backend {
            (self.callback)();
            *backend = false;
        }
    }
}
