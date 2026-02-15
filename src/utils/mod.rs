#[allow(dead_code)]
pub fn ensure_dir(path: &std::path::Path) -> std::path::PathBuf {
    std::fs::create_dir_all(path).ok();
    path.to_path_buf()
}
