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
        ephemeral_kv::{EphemeralKv, EphemeralKvHook},
        kv::{Kv, KvHook, PersistedKvHook},
        memo::MemoHook,
        persisted_state::PersistedStateHook,
        state::{State, StateHook},
        two_frame_kv::{PersistedTwoFrameKvHook, TwoFrameKv, TwoFrameKvHook},
        Hook,
    },
};

pub trait UseHookExt {
    /// Use a hook in the context of a widget with the given id.
    fn use_hook_as<T: Hook<D>, D: Deps>(&mut self, id: egui::Id, hook: T, deps: D) -> T::Output;
    fn use_hook<T: Hook<D>, D: Deps>(&mut self, hook: T, deps: D) -> T::Output;
    fn use_state<T: Clone + Send + Sync + 'static, D: Deps>(
        &mut self,
        default: impl FnOnce() -> T,
        deps: D,
    ) -> State<T>;
    fn use_persisted_state<T: SerializableAny, D: Deps>(
        &mut self,
        default: impl FnOnce() -> T,
        deps: D,
    ) -> State<T>;
    fn use_memo<T: Clone + Send + Sync + 'static, F: FnMut() -> T, D: Deps>(
        &mut self,
        callback: F,
        deps: D,
    ) -> T;
    fn use_effect<F: FnOnce() + Send + Sync, D: Deps>(&mut self, callback: F, deps: D);
    fn use_cleanup<F: FnOnce() + Send + Sync + 'static, D: Deps>(&mut self, callback: F, deps: D);
    fn use_kv<K: Send + Sync + 'static, V: Send + Sync + 'static>(&mut self) -> Kv<K, V>;
    fn use_persisted_kv<K: SerializableAny + Eq + std::hash::Hash, V: SerializableAny>(
        &mut self,
    ) -> Kv<K, V>;
    fn use_2f_kv<
        K: Clone + Eq + std::hash::Hash + Send + Sync + 'static,
        V: Send + Sync + 'static,
    >(
        &mut self,
    ) -> TwoFrameKv<K, V>;
    fn use_persisted_2f_kv<K: Clone + Eq + std::hash::Hash + SerializableAny, V: SerializableAny>(
        &mut self,
    ) -> TwoFrameKv<K, V>;
    fn use_ephemeral_kv<K: Eq + std::hash::Hash + Send + Sync + 'static, V: Send + Sync + 'static>(
        &mut self,
    ) -> EphemeralKv<K, V>;
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
    #[inline]
    fn use_hook_as<T: Hook<D>, D: Deps>(
        &mut self,
        id: egui::Id,
        mut hook: T,
        deps: D,
    ) -> T::Output {
        // Get hook index
        let context_id = HookContextId {
            frame: self.ctx().cumulative_pass_nr(),
            id,
        };
        let hook_index = self.memory_mut(|memory| {
            let cache = memory.caches.cache::<HookStorageForThisFrame>();
            let context = cache.get(context_id);
            context.next_hook_index.fetch_add(1, Ordering::SeqCst)
        });
        let dispatcher = Dispatcher::from_ctx(self.ctx());
        dispatcher.may_advance_frame(self.ctx().cumulative_pass_nr());
        let (mut backend, deps) =
            if let Some((backend, old_deps)) = dispatcher.get_backend::<T, D>(id, hook_index) {
                if deps.partial_eq(&old_deps) {
                    (backend, old_deps)
                } else {
                    // The dependencies are changed, so we need to re-initialize the hook
                    (
                        hook.init(hook_index, &deps, Some(backend), self),
                        Box::new(deps) as _,
                    )
                }
            } else {
                (
                    hook.init(hook_index, &deps, None, self),
                    Box::new(deps) as _,
                )
            };
        let output = hook.hook(&mut backend, self);
        dispatcher.push_backend::<T, D>(id, hook_index, backend, deps);
        output
    }

    #[inline]
    fn use_hook<T: Hook<D>, D: Deps>(&mut self, hook: T, deps: D) -> T::Output {
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
    /// let _ = ctx.run(Default::default(), |ctx| {
    ///     egui::Area::new("test".into()).show(ctx, |ui| {
    ///         use egui_hooks::UseHookExt as _;
    ///         let mut state = ui.use_state(|| 42, ());
    ///         let mut var_state = ui.use_state(|| 42, ()).into_var();
    ///     });
    /// });
    /// ```
    #[inline]
    fn use_state<T: Clone + Send + Sync + 'static, D: Deps>(
        &mut self,
        default: impl FnOnce() -> T,
        deps: D,
    ) -> State<T> {
        self.use_hook(StateHook::new(default), deps)
    }

    #[inline]
    fn use_persisted_state<T: SerializableAny, D: Deps>(
        &mut self,
        default: impl FnOnce() -> T,
        deps: D,
    ) -> State<T> {
        self.use_hook(PersistedStateHook::new(default), deps)
    }

    #[inline]
    fn use_memo<T: Clone + Send + Sync + 'static, F: FnMut() -> T, D: Deps>(
        &mut self,
        callback: F,
        deps: D,
    ) -> T {
        self.use_hook(MemoHook { callback }, deps)
    }

    #[inline]
    fn use_effect<F: FnOnce() + Send + Sync, D: Deps>(&mut self, callback: F, deps: D) {
        self.use_hook(EffectHook { callback }, deps);
    }

    #[inline]
    fn use_cleanup<F: FnOnce() + Send + Sync + 'static, D: Deps>(&mut self, callback: F, deps: D) {
        self.use_hook(CleanupHook::new(callback), deps)
    }

    #[inline]
    fn use_kv<K: Send + Sync + 'static, V: Send + Sync + 'static>(&mut self) -> Kv<K, V> {
        self.use_hook(KvHook::new(), ())
    }

    #[inline]
    fn use_persisted_kv<K: SerializableAny + Eq + std::hash::Hash, V: SerializableAny>(
        &mut self,
    ) -> Kv<K, V> {
        self.use_hook(PersistedKvHook::new(), ())
    }

    #[inline]
    fn use_2f_kv<
        K: Clone + Eq + std::hash::Hash + Send + Sync + 'static,
        V: Send + Sync + 'static,
    >(
        &mut self,
    ) -> TwoFrameKv<K, V> {
        self.use_hook(TwoFrameKvHook::new(), ())
    }

    #[inline]
    fn use_persisted_2f_kv<
        K: Clone + Eq + std::hash::Hash + SerializableAny,
        V: SerializableAny,
    >(
        &mut self,
    ) -> TwoFrameKv<K, V> {
        self.use_hook(PersistedTwoFrameKvHook::new(), ())
    }

    #[inline]
    fn use_ephemeral_kv<
        K: Eq + std::hash::Hash + Send + Sync + 'static,
        V: Send + Sync + 'static,
    >(
        &mut self,
    ) -> EphemeralKv<K, V> {
        self.use_hook(EphemeralKvHook::new(), ())
    }
}
