use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use egui::util::cache::{ComputerMut, FrameCache};

use crate::{
    deps::Deps,
    dispatcher::GLOBAL_DISPATCHER,
    hook::{
        effect::EffectHook,
        memo::MemoHook,
        state::{State, StateHook},
        Hook,
    },
};

pub trait UseHookExt {
    fn use_hook<T: Hook, D: Deps>(&mut self, hook: T, deps: D) -> T::Output;
    fn use_state<T: Clone + Send + Sync + 'static, D: Deps>(
        &mut self,
        default: T,
        deps: D,
    ) -> State<T>;
    fn use_memo<T: Clone + Send + Sync + 'static, F: FnMut() -> T, D: Deps>(
        &mut self,
        callback: F,
        deps: D,
    ) -> T;
    fn use_effect<'a, F: FnOnce() + Send + Sync + 'a, D: Deps>(&mut self, callback: F, deps: D);
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

impl UseHookExt for egui::Ui {
    fn use_hook<T: Hook, D: Deps>(&mut self, mut hook: T, deps: D) -> T::Output {
        let id = self.id();
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
        let boxed_deps = Box::new(deps);
        GLOBAL_DISPATCHER.may_advance_frame(self.ctx().frame_nr());
        let mut backend =
            if let Some((backend, old_deps)) = GLOBAL_DISPATCHER.get_backend::<T>(id, hook_index) {
                if old_deps.partial_eq(boxed_deps.as_ref()) {
                    backend
                } else {
                    // The dependencies are changed, so we need to re-initialize the hook
                    hook.init(self)
                }
            } else {
                hook.init(self)
            };
        let output = hook.hook(&mut backend, self);
        GLOBAL_DISPATCHER.push_backend::<T>(id, backend, boxed_deps);
        output
    }

    fn use_state<T: Clone + Send + Sync + 'static, D: Deps>(
        &mut self,
        default: T,
        deps: D,
    ) -> State<T> {
        self.use_hook(StateHook::new(default), deps)
    }

    fn use_memo<T: Clone + Send + Sync + 'static, F: FnMut() -> T, D: Deps>(
        &mut self,
        callback: F,
        deps: D,
    ) -> T {
        self.use_hook(MemoHook { callback }, deps)
    }

    fn use_effect<'a, F: FnOnce() + Send + Sync + 'a, D: Deps>(&mut self, callback: F, deps: D) {
        self.use_hook(EffectHook { callback }, deps);
    }
}
