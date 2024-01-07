pub mod cleanup;
pub mod effect;
pub mod ephemeral_kv;
pub mod global_state;
pub mod kv;
pub mod memo;
pub mod persisted_state;
pub mod state;
pub mod two_frame_kv;

/// The hook interfame. It needs the type parameter `D` to create a hook that depends on the deps.
pub trait Hook<D> {
    type Backend: Send + Sync + 'static;
    type Output;
    /// Called when the hook is first called
    fn init(
        &mut self,
        index: usize,
        deps: &D,
        backend: Option<Self::Backend>,
        ui: &mut egui::Ui,
    ) -> Self::Backend;
    /// Called when the hook is called again
    fn hook(self, backend: &mut Self::Backend, ui: &mut egui::Ui) -> Self::Output;
}
