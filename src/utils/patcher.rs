use std::path::PathBuf;

pub struct Patcher {
    pub root: PathBuf,
    pub upstream: PathBuf,
    pub patches: PathBuf,
    pub internal: PathBuf,
}
