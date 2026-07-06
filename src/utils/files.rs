use std::path::PathBuf;

use log::error;

pub fn base_dir() -> std::path::PathBuf {
    dirs::data_dir()
        .expect("could not get home dir")
        .join("lilith")
}
pub fn bin_dir() -> std::path::PathBuf {
    base_dir().join("bin")
}
pub fn config_path() -> PathBuf {
    base_dir().join("launcher.json")
}

pub fn init_lilith_dir() {
    /* small shortcut */
    match std::fs::create_dir_all(bin_dir()) {
        Ok(_) => {}
        Err(e) => {
            error!(target:"init_lilith_dir", "could not create lilith dir {e}")
        }
    };
}
