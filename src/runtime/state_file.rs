use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

pub(super) fn write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    static NEXT: AtomicU64 = AtomicU64::new(0);

    fs::create_dir_all(path.parent().expect("state file has a parent"))?;
    let temporary = path.with_file_name(format!(
        "{}-{}-{}.tmp",
        path.file_name()
            .expect("state file has a name")
            .to_string_lossy(),
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));

    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let result = (|| {
        file.write_all(bytes)?;
        file.sync_all()
    })();
    drop(file);

    // Replace only after the complete new state has been written successfully.
    let result = result.and_then(|()| fs::rename(&temporary, path));
    if result.is_err() {
        fs::remove_file(&temporary)?;
    }

    result
}
