use std::io;
use std::path::Path;

// A prior existence check is not a no-clobber guarantee. Publication and user
// renames must use an atomic exclusive operation, including directory renames.
pub(super) fn exclusive(source: &Path, destination: &Path) -> io::Result<()> {
    platform(source, destination)
}

#[cfg(windows)]
fn platform(source: &Path, destination: &Path) -> io::Result<()> {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn MoveFileExW(source: *const u16, destination: *const u16, flags: u32) -> i32;
    }

    let source = wide_path(source)?;
    let destination = wide_path(destination)?;

    // No MOVEFILE_REPLACE_EXISTING or COPY_ALLOWED: crossing devices is handled
    // by the application's cancellable copy/remove path.
    if unsafe { MoveFileExW(source.as_ptr(), destination.as_ptr(), 0) } == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn platform(source: &Path, destination: &Path) -> io::Result<()> {
    use std::os::unix::ffi::OsStrExt;

    let source = std::ffi::CString::new(source.as_os_str().as_bytes())?;
    let destination = std::ffi::CString::new(destination.as_os_str().as_bytes())?;

    #[cfg(target_os = "linux")]
    let result = unsafe {
        libc::renameat2(
            libc::AT_FDCWD,
            source.as_ptr(),
            libc::AT_FDCWD,
            destination.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    #[cfg(target_os = "macos")]
    let result =
        unsafe { libc::renamex_np(source.as_ptr(), destination.as_ptr(), libc::RENAME_EXCL) };

    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn platform(_source: &Path, _destination: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Atomic exclusive rename is unavailable on this platform",
    ))
}

#[cfg(windows)]
fn wide_path(path: &Path) -> io::Result<Vec<u16>> {
    use std::os::windows::ffi::OsStrExt;

    let name = path.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Cannot rename a filesystem root",
        )
    })?;

    // Canonicalizing only the parent provides the extended Windows path
    // prefix without following a final symlink (including a broken one).
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty());
    let path = parent.unwrap_or(Path::new(".")).canonicalize()?.join(name);
    let mut value: Vec<u16> = path.as_os_str().encode_wide().collect();
    if value.contains(&0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Path contains a null",
        ));
    }

    value.push(0);

    Ok(value)
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn exclusive_rename_supports_long_paths_and_preserves_existing_destination() {
        let temp = crate::test_support::TempDirectory::new();
        let parent = temp
            .0
            .join("long-directory".repeat(12))
            .join("nested".repeat(20));
        std::fs::create_dir_all(&parent).unwrap();
        let source = parent.join("source");
        let destination = parent.join("destination");
        std::fs::write(&source, "original").unwrap();
        std::fs::write(&destination, "keep").unwrap();

        assert!(exclusive(&source, &destination).is_err());
        assert_eq!(std::fs::read(&source).unwrap(), b"original");
        assert_eq!(std::fs::read(&destination).unwrap(), b"keep");

        std::fs::remove_file(&destination).unwrap();
        exclusive(&source, &destination).unwrap();

        assert!(!source.exists());
        assert_eq!(std::fs::read(&destination).unwrap(), b"original");
    }
}
