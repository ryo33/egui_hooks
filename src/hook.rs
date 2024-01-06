pub mod cleanup;
pub mod effect;
pub mod memo;
pub mod persisted_state;
pub mod state;

pub trait Hook<D> {
    type Backend: Send + Sync + 'static;
    type Output: 'static;
    /// Called when the hook is first called
    fn init(&mut self, index: usize, deps: &D, ui: &mut egui::Ui) -> Self::Backend;
    /// Called when the hook is called again
    fn hook(self, backend: &mut Self::Backend, ui: &mut egui::Ui) -> Self::Output;
}
