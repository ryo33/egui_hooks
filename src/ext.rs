use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use egui::util::{
    cache::{ComputerMut, FrameCache},
    id_type_map::SerializableAny,
};

use crate::{
    deps::Deps,
    dispatcher::Dispatcher,
    hook::{
        cleanup::CleanupHook,
        effect::EffectHook,
        memo::MemoHook,
        persistent_state::PersistentStateHook,
        state::{State, StateHook},
        Hook,
    },
};

pub trait UseHookExt<D: Deps> {
    fn use_hook_as<T: Hook<D>>(&mut self, id: egui::Id, hook: T, deps: D) -> T::Output;
    fn use_hook<T: Hook<D>>(&mut self, hook: T, deps: D) -> T::Output;
    fn use_state<T: Clone + Send + Sync + 'static>(&mut self, default: T, deps: D) -> State<T>;
    fn use_persistent_state<T: SerializableAny>(&mut self, default: T, deps: D) -> State<T>;
    fn use_memo<T: Clone + Send + Sync + 'static, F: FnMut() -> T>(
        &mut self,
        callback: F,
        deps: D,
    ) -> T;
    fn use_effect<F: FnOnce() + Send + Sync>(&mut self, callback: F, deps: D);
    fn use_cleanup<F: FnOnce() + Send + Sync + 'static>(&mut self, callback: F, deps: D);
}

/// The hook context for this frame in
///
/// Though using "cache", this is not a kind of cache, and this is a hidden context for hooks
/// exactly for the current frame. So frame ID is contained in the key, and the value never be
/// cached between frames.
type HookStorageForThisFrame = FrameCache<ExtContext, HookStorageFactory>;

#[derive(Default)]
struct HookStorageFactory {}
impl ComputerMut<HookContextId, ExtContext> for HookStorageFactory {
    fn compute(&mut self, _key: HookContextId) -> ExtContext {
        Default::default()
    }
}

/// The ID to get the hook context for this frame for the given widget id
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct HookContextId {
    frame: u64,
    id: egui::Id,
}

#[derive(Default, Clone)]
struct ExtContext {
    next_hook_index: Arc<AtomicUsize>,
}

impl<D: Deps> UseHookExt<D> for egui::Ui {
    fn use_hook_as<T: Hook<D>>(&mut self, id: egui::Id, mut hook: T, deps: D) -> T::Output {
        // Get hook index
        let context_id = HookContextId {
            frame: self.ctx().frame_nr(),
            id,
        };
        let hook_index = self.memory_mut(|memory| {
            let cache = memory.caches.cache::<HookStorageForThisFrame>();
            let context = cache.get(context_id);
            context.next_hook_index.fetch_add(1, Ordering::SeqCst)
        });
        let dispatcher = Dispatcher::from_ui(self);
        dispatcher.may_advance_frame(self.ctx().frame_nr());
        let (mut backend, deps) =
            if let Some((backend, old_deps)) = dispatcher.get_backend::<T, D>(id, hook_index) {
                if deps.partial_eq(&old_deps) {
                    (backend, old_deps)
                } else {
                    // The dependencies are changed, so we need to re-initialize the hook
                    (hook.init(hook_index, &deps, self), Box::new(deps) as _)
                }
            } else {
                (hook.init(hook_index, &deps, self), Box::new(deps) as _)
            };
        let output = hook.hook(&mut backend, self);
        dispatcher.push_backend::<T, D>(id, hook_index, backend, deps);
        output
    }
    fn use_hook<T: Hook<D>>(&mut self, hook: T, deps: D) -> T::Output {
        let id = self.id();
        self.use_hook_as(id, hook, deps)
    }

    /// Returns a state that is initialized with the given default value.
    /// A state is resetted to new default value when the dependencies are changed.
    /// If you need to mutabe the state as like normal variable, put `.into_var()` after this.
    ///
    /// # Example
    /// ```
    /// let ctx = egui::Context::default();
    /// egui::Area::new("test").show(&ctx, |ui| {
    ///     use egui_hooks::UseHookExt as _;
    ///     let mut state = ui.use_state(42, ());
    ///     let mut var_state = ui.use_state(42, ()).into_var();
    /// });
    /// ```
    fn use_state<T: Clone + Send + Sync + 'static>(&mut self, default: T, deps: D) -> State<T> {
        self.use_hook(StateHook::new(default), deps)
    }

    fn use_persistent_state<T: SerializableAny>(&mut self, default: T, deps: D) -> State<T> {
        self.use_hook(PersistentStateHook::new(default), deps)
    }

    fn use_memo<T: Clone + Send + Sync + 'static, F: FnMut() -> T>(
        &mut self,
        callback: F,
        deps: D,
    ) -> T {
        self.use_hook(MemoHook { callback }, deps)
    }

    fn use_effect<F: FnOnce() + Send + Sync>(&mut self, callback: F, deps: D) {
        self.use_hook(EffectHook { callback }, deps);
    }

    fn use_cleanup<F: FnOnce() + Send + Sync + 'static>(&mut self, callback: F, deps: D) {
        self.use_hook(CleanupHook::new(callback), deps)
    }
}
