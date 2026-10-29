pub mod archive_preview;
mod copy_file;
mod directory;
pub mod directory_preview;
pub mod edit;
#[cfg(test)]
mod faults;
pub mod finder;
pub mod metadata;
mod operation_progress;
mod operations;
mod rename;
pub mod text_preview;
pub mod tree;

pub use directory::{load_directory, load_snapshot, refresh_panels, reveal_finder_result};
pub use operation_progress::execute_batch_with_progress;
pub use operations::{execute_batch, execute_operation};

fn is_link(metadata: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0 // FILE_ATTRIBUTE_REPARSE_POINT
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

fn is_directory_link(metadata: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        is_link(metadata) && metadata.file_attributes() & 0x10 != 0
    }
    #[cfg(not(windows))]
    {
        let _ = metadata;
        false
    }
}
