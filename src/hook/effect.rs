use crate::deps::Deps;

use super::Hook;

pub struct EffectHook<F> {
    pub callback: F,
}

impl<'a, F: FnOnce() + Send + Sync + 'a, D: Deps> Hook<D> for EffectHook<F> {
    type Backend = bool;
    type Output = ();
    fn init(&mut self, _index: usize, _deps: &D, _ui: &mut egui::Ui) -> Self::Backend {
        true
    }
    fn hook(self, backend: &mut Self::Backend, _ui: &mut egui::Ui) -> Self::Output {
        if *backend {
            (self.callback)();
            *backend = false;
        }
    }
}

pub struct EffectHookWithCleanup<F> {
    pub callback: F,
}
