#[repr(transparent)]
pub struct UnsafeSync<T>(T);

unsafe impl<T> Send for UnsafeSync<T> {}
unsafe impl<T> Sync for UnsafeSync<T> {}

impl<T> UnsafeSync<T> {
    pub unsafe fn new(x: T) -> Self {
        UnsafeSync(x)
    }

    pub fn get(&self) -> &T {
        &self.0
    }
}
