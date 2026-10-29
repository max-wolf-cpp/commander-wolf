use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek};
use std::path::Path;

use super::paths::EntryPaths;
use crate::file_operations::progress::OperationObserver;
use crate::preview::ArchiveFormat;

struct Output<'a> {
    root: &'a Path,
    paths: EntryPaths,
    observer: &'a mut dyn OperationObserver,
}

pub(super) fn unpack(
    source: &Path,
    destination: &Path,
    observer: &mut dyn OperationObserver,
) -> Result<(), String> {
    let format = ArchiveFormat::from_path(source)
        .ok_or("Unsupported archive; supported formats are ZIP, TAR and 7z")?;
    let file = File::open(source).map_err(|error| error.to_string())?;
    let mut output = Output {
        root: destination,
        paths: EntryPaths::default(),
        observer,
    };

    match format {
        ArchiveFormat::Zip => zip(file, &mut output),
        ArchiveFormat::Tar => tar(file, &mut output),
        ArchiveFormat::SevenZip => seven_zip(file, &mut output),
    }
}

fn zip(file: File, output: &mut Output<'_>) -> Result<(), String> {
    let mut archive = zip::ZipArchive::new(file).map_err(|error| error.to_string())?;
    let mut total = 0u64;
    if archive.len() > super::MAX_ENTRIES {
        return Err("Archive exceeds the 100,000 entry limit".into());
    }
    for index in 0..archive.len() {
        output.observer.check_cancelled()?;
        let entry = archive
            .by_index_raw(index)
            .map_err(|error| error.to_string())?;
        total = add_size(total, entry.size())?;
    }
    output.observer.set_total(total);
    output.observer.phase("Extracting");

    for index in 0..archive.len() {
        output.observer.check_cancelled()?;
        let mut entry = archive.by_index(index).map_err(|error| error.to_string())?;
        let name = entry.name().to_owned();
        let directory = entry.is_dir();
        let size = entry.size();
        let mode = entry.unix_mode();
        reject_special_mode(mode)?;
        output.entry(&name, directory, size, mode, &mut entry)?;
    }
    Ok(())
}

fn tar(mut file: File, output: &mut Output<'_>) -> Result<(), String> {
    let mut total = 0u64;
    {
        let mut archive = tar::Archive::new(&mut file);
        for (index, entry) in archive
            .entries_with_seek()
            .map_err(|error| error.to_string())?
            .enumerate()
        {
            output.observer.check_cancelled()?;
            if index >= super::MAX_ENTRIES {
                return Err("Archive exceeds the 100,000 entry limit".into());
            }
            let entry = entry.map_err(|error| error.to_string())?;
            total = add_size(total, entry.size())?;
        }
    }
    file.rewind().map_err(|error| error.to_string())?;
    output.observer.set_total(total);
    output.observer.phase("Extracting");

    let mut archive = tar::Archive::new(file);
    for entry in archive.entries().map_err(|error| error.to_string())? {
        output.observer.check_cancelled()?;
        let mut entry = entry.map_err(|error| error.to_string())?;
        let kind = entry.header().entry_type();
        if !(kind.is_file() || kind.is_dir()) {
            return Err("Archive links and special entries are not supported".into());
        }
        let name = entry
            .path()
            .map_err(|error| error.to_string())?
            .into_owned()
            .into_os_string()
            .into_string()
            .map_err(|_| "Archive path is not Unicode")?;
        let mode = entry.header().mode().map_err(|error| error.to_string())?;
        let size = entry.size();
        output.entry(&name, kind.is_dir(), size, Some(mode), &mut entry)?;
    }
    Ok(())
}

fn seven_zip(file: File, output: &mut Output<'_>) -> Result<(), String> {
    let mut archive = sevenz_rust2::ArchiveReader::new(file, sevenz_rust2::Password::empty())
        .map_err(|error| error.to_string())?;
    if archive.archive().files.len() > super::MAX_ENTRIES {
        return Err("Archive exceeds the 100,000 entry limit".into());
    }
    let mut total = 0u64;
    for entry in &archive.archive().files {
        output.observer.check_cancelled()?;
        total = add_size(total, entry.size())?;
        if entry.is_anti_item() || entry.windows_attributes() & 0x400 != 0 {
            return Err("Archive links, reparse points and anti-items are not supported".into());
        }
        reject_special_mode(Some(entry.windows_attributes() >> 16))?;
    }
    output.observer.set_total(total);
    output.observer.phase("Extracting");

    archive
        .for_each_entries(|entry, reader| {
            output
                .entry(
                    entry.name(),
                    entry.is_directory(),
                    entry.size(),
                    (entry.windows_attributes() >> 16 != 0)
                        .then_some(entry.windows_attributes() >> 16),
                    reader,
                )
                .map_err(|error| sevenz_rust2::Error::Other(error.into()))?;
            Ok(true)
        })
        .map_err(|error| error.to_string())
}

impl Output<'_> {
    fn entry(
        &mut self,
        name: &str,
        directory: bool,
        size: u64,
        mode: Option<u32>,
        reader: &mut dyn Read,
    ) -> Result<(), String> {
        self.observer.check_cancelled()?;
        let relative = self.paths.register(name, directory)?;
        let destination = self.root.join(relative);
        self.observer.begin_file(&destination, size);
        if directory {
            if size != 0 {
                return Err("Archive directory contains file data".into());
            }
            fs::create_dir_all(&destination).map_err(|error| error.to_string())?;
            self.observer.finish_file();
            return Ok(());
        }

        let parent = destination.parent().ok_or("Archive entry has no parent")?;
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&destination)
            .map_err(|error| format!("Cannot create archive entry '{name}': {error}"))?;
        super::copy_stream(reader, &mut file, size, self.observer)?;
        apply_permissions(&destination, mode)
    }
}

fn add_size(total: u64, size: u64) -> Result<u64, String> {
    let total = total
        .checked_add(size.max(1))
        .ok_or("Archive size overflow")?;
    if total > super::MAX_EXTRACTED_BYTES {
        return Err("Archive exceeds the 64 GiB extraction limit".into());
    }
    Ok(total)
}

fn reject_special_mode(mode: Option<u32>) -> Result<(), String> {
    match mode.unwrap_or(0) & 0o170000 {
        0 | 0o100000 | 0o040000 => Ok(()),
        _ => Err("Archive links and special entries are not supported".into()),
    }
}

fn apply_permissions(path: &Path, mode: Option<u32>) -> Result<(), String> {
    #[cfg(unix)]
    if let Some(mode) = mode {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(mode & 0o777))
            .map_err(|error| error.to_string())?;
    }
    let _ = (path, mode);
    Ok(())
}
