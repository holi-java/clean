pub struct RmDirGuard<T: AsRef<std::path::Path>>(pub T);

impl<T: AsRef<std::path::Path>> Drop for RmDirGuard<T> {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(self.0.as_ref());
    }
}
