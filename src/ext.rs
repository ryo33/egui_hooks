use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use egui::util::cache::{ComputerMut, FrameCache};

use crate::{
    deps::{Deps, DynDeps},
    hook::{memo::MemoHook, state::StateHook, Hook},
};

pub trait UseHookExt {
    fn use_hook<T: Hook, D: Deps>(&mut self, hook: T, deps: D) -> T::Output;
    fn use_state<T: Clone + Send + Sync + 'static, D: Deps>(
        &mut self,
        default: T,
        deps: D,
    ) -> (Arc<T>, Box<dyn Fn(T)>);
    fn use_memo<T: Clone + Send + Sync + 'static, F: FnMut() -> T, D: Deps>(
        &mut self,
        callback: F,
        deps: D,
    ) -> T;
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
        let id = self.make_persistent_id("egui_hooks_internal");
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
        self.data_mut(|data| {
            let (backend, old_deps) = data
                .get_temp_mut_or_insert_with::<(T::Backend, Box<dyn DynDeps>)>(
                    id.with(hook_index),
                    || (hook.init(), boxed_deps.clone()),
                );
            if !old_deps.partial_eq(boxed_deps.as_ref()) {
                *backend = hook.init();
                *old_deps = boxed_deps;
            }
            hook.hook(backend)
        })
    }

    fn use_state<T: Clone + Send + Sync + 'static, D: Deps>(
        &mut self,
        default: T,
        deps: D,
    ) -> (Arc<T>, Box<dyn Fn(T)>) {
        self.use_hook(StateHook { default }, deps)
    }

    fn use_memo<T: Clone + Send + Sync + 'static, F: FnMut() -> T, D: Deps>(
        &mut self,
        callback: F,
        deps: D,
    ) -> T {
        self.use_hook(MemoHook { callback }, deps)
    }
}
