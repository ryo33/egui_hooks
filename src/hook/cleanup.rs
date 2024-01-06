use crate::{deps::Deps, dispatcher::Dispatcher};

use super::Hook;

pub struct CleanupHook {
    // Option is used to Option::take
    f: Option<Box<dyn FnOnce() + Send + Sync + 'static>>,
}

impl CleanupHook {
    #[inline]
    pub fn new(f: impl FnOnce() + Send + Sync + 'static) -> Self {
        Self {
            f: Some(Box::new(f)),
        }
    }
}

impl<D: Deps> Hook<D> for CleanupHook {
    type Backend = ();
    type Output = ();
    #[inline]
    fn init(&mut self, _hook_index: usize, _deps: &D, ui: &mut egui::Ui) -> Self::Backend {
        let id = ui.id();
        let dispatcher = Dispatcher::from_ui(ui);
        dispatcher.register_cleanup(id, self.f.take().unwrap().into());
    }
    #[inline]
    fn hook(self, _backend: &mut Self::Backend, _ui: &mut egui::Ui) -> Self::Output {}
}

#[test]
fn cleanup() {
    let ctx = egui::Context::default();
    egui::Area::new("test").show(&ctx, |ui| {
        let dispatcher = Dispatcher::from_ui(ui);

        let called = std::sync::Arc::new(egui::mutex::Mutex::new(Vec::new()));
        let cloned = called.clone();
        use crate::UseHookExt;
        ui.use_cleanup(
            move || {
                cloned.lock().push(());
            },
            (),
        );

        // not called yet since this is the first frame
        assert_eq!(called.lock().len(), 0);

        ui.ctx().begin_frame(Default::default());
        dispatcher.may_advance_frame(100);
        // not called yet since this is the second frame
        assert_eq!(called.lock().len(), 0);

        ui.ctx().begin_frame(Default::default());
        dispatcher.may_advance_frame(101);
        // called since this is the third frame
        assert_eq!(called.lock().len(), 1);
    });
}
