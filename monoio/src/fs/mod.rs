//! Filesystem manipulation operations.

mod file;
pub use file::{remove_dir, remove_file, rename, File};

mod open_options;
pub use open_options::OpenOptions;
