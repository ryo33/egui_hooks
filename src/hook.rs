pub mod effect;
pub mod memo;
pub mod state;

use egui::util::id_type_map::SerializableAny;

pub trait Hook {
    type Backend: Clone + Send + Sync + 'static;
    type Output: 'static;
    /// Called when the hook is first called
    fn init(&mut self, ui: &mut egui::Ui) -> Self::Backend;
    /// Called when the hook is called again
    fn hook(self, backend: &mut Self::Backend, ui: &mut egui::Ui) -> Self::Output;
}

pub trait SerializableHook: Hook
where
    Self::Backend: SerializableAny,
{
}

impl<T> SerializableHook for T
where
    T: Hook,
    T::Backend: SerializableAny,
{
}
