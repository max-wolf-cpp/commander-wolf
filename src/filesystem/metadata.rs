use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::preview::FileMetadata;

#[cfg(windows)]
use windows::hard_link_count;

pub fn load(path: &Path) -> Result<FileMetadata, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("Cannot read metadata for '{}': {error}", path.display()))?;
    let size = if metadata.file_type().is_file() {
        let bytes = metadata.len();
        format!("{} ({} bytes)", format_size(bytes), grouped_integer(bytes))
    } else if metadata.file_type().is_dir() {
        "Not applicable".to_owned()
    } else {
        "Unavailable".to_owned()
    };

    Ok(FileMetadata {
        path: path.display().to_string(),
        file_type: file_type_name(&metadata).to_owned(),
        size,
        created: format_optional_time(metadata.created().ok()),
        modified: format_optional_time(metadata.modified().ok()),
        permissions: format_permissions(&metadata),
        extension: path
            .extension()
            .and_then(|value| value.to_str())
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
        hard_links: hard_link_count(path, &metadata),
    })
}

fn file_type_name(metadata: &fs::Metadata) -> &'static str {
    let file_type = metadata.file_type();
    if file_type.is_file() {
        "Regular file"
    } else if file_type.is_dir() {
        "Directory"
    } else if file_type.is_symlink() {
        "Symbolic link"
    } else {
        special_file_type(&file_type)
    }
}

#[cfg(unix)]
fn special_file_type(file_type: &fs::FileType) -> &'static str {
    use std::os::unix::fs::FileTypeExt;
    if file_type.is_block_device() {
        "Block device"
    } else if file_type.is_char_device() {
        "Character device"
    } else if file_type.is_fifo() {
        "FIFO"
    } else if file_type.is_socket() {
        "Socket"
    } else {
        "Unknown"
    }
}

#[cfg(not(unix))]
fn special_file_type(_: &fs::FileType) -> &'static str {
    "Unknown"
}

fn format_size(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = KIB * 1024;
    const GIB: u64 = MIB * 1024;
    if bytes >= GIB {
        format!("{:.2} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.2} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.2} KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{bytes} B")
    }
}

fn grouped_integer(value: u64) -> String {
    let digits = value.to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, character) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(character);
    }

    grouped
}

fn format_optional_time(time: Option<SystemTime>) -> String {
    time.and_then(format_time)
        .unwrap_or_else(|| "Unavailable".to_owned())
}

fn format_time(time: SystemTime) -> Option<String> {
    let seconds = time.duration_since(UNIX_EPOCH).ok()?.as_secs();
    let days = i64::try_from(seconds / 86_400).ok()?;
    let (year, month, day) = civil_from_days(days);

    let seconds_of_day = seconds % 86_400;
    let hour = seconds_of_day / 3600;
    let minute = seconds_of_day % 3600 / 60;
    let second = seconds_of_day % 60;

    Some(format!(
        "{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02} UTC"
    ))
}

fn civil_from_days(days_since_epoch: i64) -> (i64, i64, i64) {
    let days = days_since_epoch + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    let mut year = year_of_era + era * 400;
    year += i64::from(month <= 2);

    (year, month, day)
}

#[cfg(unix)]
fn format_permissions(metadata: &fs::Metadata) -> String {
    use std::os::unix::fs::{FileTypeExt, MetadataExt};
    let file_type = metadata.file_type();
    let mut value = String::with_capacity(10);
    value.push(if file_type.is_dir() {
        'd'
    } else if file_type.is_symlink() {
        'l'
    } else if file_type.is_block_device() {
        'b'
    } else if file_type.is_char_device() {
        'c'
    } else if file_type.is_fifo() {
        'p'
    } else if file_type.is_socket() {
        's'
    } else {
        '-'
    });

    let mode = metadata.mode();
    for (read, write, execute, special, lower, upper) in [
        (0o400, 0o200, 0o100, 0o4000, 's', 'S'),
        (0o040, 0o020, 0o010, 0o2000, 's', 'S'),
        (0o004, 0o002, 0o001, 0o1000, 't', 'T'),
    ] {
        value.push(if mode & read != 0 { 'r' } else { '-' });
        value.push(if mode & write != 0 { 'w' } else { '-' });
        value.push(match (mode & execute != 0, mode & special != 0) {
            (true, true) => lower,
            (false, true) => upper,
            (true, false) => 'x',
            (false, false) => '-',
        });
    }

    value
}

#[cfg(windows)]
fn format_permissions(metadata: &fs::Metadata) -> String {
    if metadata.permissions().readonly() {
        "Windows read-only".to_owned()
    } else {
        "Windows read/write".to_owned()
    }
}

#[cfg(not(any(unix, windows)))]
fn format_permissions(metadata: &fs::Metadata) -> String {
    if metadata.permissions().readonly() {
        "Read-only".to_owned()
    } else {
        "Read/write".to_owned()
    }
}

#[cfg(unix)]
fn hard_link_count(_: &Path, metadata: &fs::Metadata) -> Option<u64> {
    use std::os::unix::fs::MetadataExt;
    Some(metadata.nlink())
}

#[cfg(windows)]
mod windows {
    use std::ffi::c_void;
    use std::fs;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    const FILE_READ_ATTRIBUTES: u32 = 0x0080;
    const SHARE_ALL: u32 = 0x0000_0001 | 0x0000_0002 | 0x0000_0004;
    const OPEN_EXISTING: u32 = 3;
    const BACKUP_SEMANTICS: u32 = 0x0200_0000;
    const OPEN_REPARSE_POINT: u32 = 0x0020_0000;

    type Handle = *mut c_void;

    #[repr(C)]
    struct FileTime {
        low: u32,
        high: u32,
    }

    #[repr(C)]
    struct ByHandleFileInformation {
        attributes: u32,
        creation_time: FileTime,
        last_access_time: FileTime,
        last_write_time: FileTime,
        volume_serial_number: u32,
        file_size_high: u32,
        file_size_low: u32,
        number_of_links: u32,
        file_index_high: u32,
        file_index_low: u32,
    }

    unsafe extern "system" {
        fn CreateFileW(
            file_name: *const u16,
            desired_access: u32,
            share_mode: u32,
            security_attributes: *mut c_void,
            creation_disposition: u32,
            flags_and_attributes: u32,
            template_file: Handle,
        ) -> Handle;
        fn GetFileInformationByHandle(
            file: Handle,
            information: *mut ByHandleFileInformation,
        ) -> i32;
        fn CloseHandle(object: Handle) -> i32;
    }

    pub(super) fn hard_link_count(path: &Path, _: &fs::Metadata) -> Option<u64> {
        let wide = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        let handle = unsafe {
            CreateFileW(
                wide.as_ptr(),
                FILE_READ_ATTRIBUTES,
                SHARE_ALL,
                std::ptr::null_mut(),
                OPEN_EXISTING,
                BACKUP_SEMANTICS | OPEN_REPARSE_POINT,
                std::ptr::null_mut(),
            )
        };
        if handle as isize == -1 {
            return None;
        }

        let mut information = std::mem::MaybeUninit::<ByHandleFileInformation>::uninit();
        let succeeded =
            unsafe { GetFileInformationByHandle(handle, information.as_mut_ptr()) != 0 };
        unsafe { CloseHandle(handle) };
        succeeded.then(|| u64::from(unsafe { information.assume_init() }.number_of_links))
    }
}

#[cfg(not(any(unix, windows)))]
fn hard_link_count(_: &Path, _: &fs::Metadata) -> Option<u64> {
    None
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;
    use crate::preview::MetadataRow;

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "commander-wolf-metadata-{}-{}",
                std::process::id(),
                NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();

            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn regular_file_metadata_has_stable_order_and_exact_size() {
        let root = TestDirectory::new();
        let path = root.0.join("данные file.bin");
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(&vec![0; 1280]).unwrap();

        let metadata = load(&path).unwrap();
        assert_eq!(metadata.file_type, "Regular file");
        assert_eq!(metadata.size, "1.25 KiB (1,280 bytes)");
        assert_eq!(metadata.extension.as_deref(), Some("bin"));

        let labels = metadata
            .rows()
            .into_iter()
            .filter_map(|row| match row {
                MetadataRow::Field { label, .. } => Some(label),
                MetadataRow::Separator => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            &labels[..6],
            ["Path", "Type", "Size", "Created", "Modified", "Permissions"]
        );
    }

    #[test]
    fn directory_size_and_unavailable_creation_are_explicit() {
        let root = TestDirectory::new();
        let metadata = load(&root.0).unwrap();
        assert_eq!(metadata.file_type, "Directory");
        assert_eq!(metadata.size, "Not applicable");
        assert_eq!(format_optional_time(None), "Unavailable");
        assert!(metadata.modified.ends_with(" UTC"));
    }

    #[test]
    fn missing_path_returns_a_typed_error() {
        let error = load(Path::new("missing/不存在")).unwrap_err();
        assert!(error.contains("Cannot read metadata"));
    }

    #[test]
    fn hard_link_count_is_reported_when_the_platform_exposes_it() {
        let root = TestDirectory::new();
        let original = root.0.join("original.bin");
        let linked = root.0.join("linked.bin");
        fs::write(&original, "linked bytes").unwrap();
        fs::hard_link(&original, &linked).unwrap();
        assert!(
            load(&linked)
                .unwrap()
                .hard_links
                .is_some_and(|count| count >= 2)
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlink_metadata_does_not_follow_the_target() {
        use std::os::unix::fs::symlink;
        let root = TestDirectory::new();
        let target = root.0.join("target");
        fs::write(&target, "target bytes").unwrap();
        let link = root.0.join("link");
        symlink(&target, &link).unwrap();
        let metadata = load(&link).unwrap();
        assert_eq!(metadata.file_type, "Symbolic link");
        assert!(metadata.permissions.starts_with('l'));
    }

    #[cfg(unix)]
    #[test]
    fn unix_socket_is_reported_as_a_special_file() {
        use std::os::unix::net::UnixListener;
        let root = TestDirectory::new();
        let path = root.0.join("preview.sock");
        let _listener = UnixListener::bind(&path).unwrap();
        let metadata = load(&path).unwrap();
        assert_eq!(metadata.file_type, "Socket");
        assert!(metadata.permissions.starts_with('s'));
    }
}
