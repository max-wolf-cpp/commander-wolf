mod associations;
mod file_manager;
mod process;

use std::path::Path;

pub use associations::{Application, FileAssociations};
pub use file_manager::reveal_in_file_manager;
pub use process::{open_external, run_editor_in_directory, run_text_editor};

pub fn uses_internal_image_viewer(path: &Path) -> bool {
    let Some(extension) = path.extension() else {
        return false;
    };

    // This is application routing policy; keep it aligned with Fox Media Term's image formats.
    matches!(
        extension.to_string_lossy().to_ascii_lowercase().as_str(),
        "jpg"
            | "jpeg"
            | "png"
            | "webp"
            | "tga"
            | "bmp"
            | "psd"
            | "gif"
            | "hdr"
            | "pic"
            | "pnm"
            | "ppm"
            | "pgm"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_viewer_extensions_match_native_image_support() {
        for path in ["wolf.jpg", "wolf.PNG", "wolf.psd", "wolf.pgm", "wolf.WEBP"] {
            assert!(uses_internal_image_viewer(Path::new(path)), "{path}");
        }
        for path in [
            "wolf.mp4",
            "wolf.avif",
            "wolf.heic",
            "wolf.svg",
            "wolf.txt",
            "wolf",
        ] {
            assert!(!uses_internal_image_viewer(Path::new(path)), "{path}");
        }
    }
}
