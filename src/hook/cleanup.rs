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
    fn init(
        &mut self,
        _hook_index: usize,
        _deps: &D,
        _backend: Option<Self::Backend>,
        ui: &mut egui::Ui,
    ) -> Self::Backend {
        let id = ui.id();
        let dispatcher = Dispatcher::from_ctx(ui.ctx());
        dispatcher.register_cleanup(id, self.f.take().unwrap().into());
    }
    #[inline]
    fn hook(self, _backend: &mut Self::Backend, _ui: &mut egui::Ui) -> Self::Output {}
}

#[test]
fn cleanup() {
    use crate::UseHookExt;
    let ctx = egui::Context::default();
    let called = std::sync::Arc::new(egui::mutex::Mutex::new(Vec::new()));

    let _ = ctx.run(Default::default(), |ctx| {
        egui::Area::new("test".into()).show(ctx, |ui| {
            let cloned = called.clone();
            ui.use_cleanup(move || cloned.lock().push(()), ());
        });

        // not called yet since this is the first frame
        assert_eq!(called.lock().len(), 0);
    });

    let _ = ctx.run(Default::default(), |ctx| {
        // ensure the advance of frame
        egui::Area::new("test2".into()).show(ctx, |ui| {
            ui.use_state(|| 0u32, ());
        });

        // not called yet since this is the second frame
        assert_eq!(called.lock().len(), 0);
    });

    let _ = ctx.run(Default::default(), |ctx| {
        // ensure the advance of frame
        egui::Area::new("test2".into()).show(ctx, |ui| {
            ui.use_state(|| 0u32, ());
        });

        // called since this is the third frame
        assert_eq!(called.lock().len(), 1);
    });
}
