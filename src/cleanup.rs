pub trait Cleanup: Send + Sync + 'static {
    fn cleanup(&mut self);
}

impl<T: FnOnce() + Send + Sync + 'static> Cleanup for Option<T> {
    #[inline]
    fn cleanup(&mut self) {
        if let Some(f) = self.take() {
            f();
        }
    }
}

impl<T: FnOnce() + Send + Sync + 'static> From<T> for Box<dyn Cleanup> {
    #[inline]
    fn from(f: T) -> Self {
        Box::new(Some(f))
    }
}

impl Default for Box<dyn Cleanup> {
    #[inline]
    fn default() -> Self {
        Box::new(Some(|| {}))
    }
}
