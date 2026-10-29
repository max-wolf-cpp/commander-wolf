use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::file_operations::{
    FileOperation, FileOperationBatchRequest, FileOperationRequest, OperationStatus,
    PanelRefreshRequest, RefreshSelection,
};
use crate::workspace::PanelId;

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "commander-wolf-archives-{}-{suffix}",
            std::process::id()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn zip_round_trip_preserves_nested_empty_hidden_and_unicode_entries() {
    let root = TestDirectory::new();
    let directory = root.0.join("目录 with spaces");
    fs::create_dir_all(directory.join("empty")).unwrap();
    let content = "Commander Wolf\n".repeat(4096);
    fs::write(directory.join("новый файл.txt"), &content).unwrap();
    fs::write(directory.join(".hidden"), "hidden").unwrap();
    let other = root.0.join("other.txt");
    fs::write(&other, "other").unwrap();
    let archive = root.0.join("backup.zip");
    let outcome = execute(
        &root.0,
        FileOperation::CreateArchive {
            sources: vec![directory, other],
            destination: archive.clone(),
        },
    );
    assert_eq!(
        outcome.status,
        OperationStatus::Success,
        "{}",
        outcome.message
    );

    let mut zip = zip::ZipArchive::new(fs::File::open(&archive).unwrap()).unwrap();
    assert_eq!(
        zip.by_name("目录 with spaces/новый файл.txt")
            .unwrap()
            .compression(),
        zip::CompressionMethod::Deflated
    );
    assert!(fs::metadata(&archive).unwrap().len() < content.len() as u64);
    let output = root.0.join("unpacked");
    let outcome = execute(
        &root.0,
        FileOperation::ExtractArchive {
            source: archive,
            destination: output.clone(),
        },
    );
    assert_eq!(
        outcome.status,
        OperationStatus::Success,
        "{}",
        outcome.message
    );
    assert_eq!(
        fs::read_to_string(output.join("目录 with spaces/новый файл.txt")).unwrap(),
        content
    );
    assert_eq!(
        fs::read_to_string(output.join("目录 with spaces/.hidden")).unwrap(),
        "hidden"
    );
    assert_eq!(
        fs::read_to_string(output.join("other.txt")).unwrap(),
        "other"
    );
    assert!(output.join("目录 with spaces/empty").is_dir());
    assert_no_staging(&root.0);
}

#[test]
fn existing_destinations_are_never_overwritten_or_merged() {
    let root = TestDirectory::new();
    let source = root.0.join("source.txt");
    fs::write(&source, "source").unwrap();
    let archive = root.0.join("existing.zip");
    fs::write(&archive, "existing").unwrap();
    let outcome = execute(
        &root.0,
        FileOperation::CreateArchive {
            sources: vec![source],
            destination: archive.clone(),
        },
    );
    assert_eq!(outcome.status, OperationStatus::Failure);
    assert_eq!(fs::read_to_string(&archive).unwrap(), "existing");

    zip_file(&archive, &[("data", b"new")]);
    let output = root.0.join("existing");
    fs::create_dir(&output).unwrap();
    fs::write(output.join("data"), "old").unwrap();
    let outcome = execute(
        &root.0,
        FileOperation::ExtractArchive {
            source: archive,
            destination: output.clone(),
        },
    );
    assert_eq!(outcome.status, OperationStatus::Failure);
    assert_eq!(fs::read_to_string(output.join("data")).unwrap(), "old");
    assert_no_staging(&root.0);
}

#[test]
fn unsafe_zip_paths_and_file_directory_conflicts_roll_back_every_entry() {
    let root = TestDirectory::new();
    let archive = root.0.join("bad.zip");
    let output = root.0.join("output");
    for name in [
        "../escape.txt",
        "folder/../../escape.txt",
        "..\\escape.txt",
        "/absolute.txt",
        "C:\\escape.txt",
        "folder/file:stream",
    ] {
        zip_file(&archive, &[("first.txt", b"first"), (name, b"bad")]);
        let outcome = execute(
            &root.0,
            FileOperation::ExtractArchive {
                source: archive.clone(),
                destination: output.clone(),
            },
        );
        assert_eq!(
            outcome.status,
            OperationStatus::Failure,
            "{name}: {}",
            outcome.message
        );
        assert!(!outcome.filesystem_changed);
        assert!(!output.exists());
        assert!(!root.0.join("escape.txt").exists());
        assert_no_staging(&root.0);
    }

    for entries in [
        vec![("a", b"data".as_slice()), ("a/b", b"child")],
        vec![("a/b", b"child".as_slice()), ("a", b"data")],
    ] {
        zip_file(&archive, &entries);
        let outcome = execute(
            &root.0,
            FileOperation::ExtractArchive {
                source: archive.clone(),
                destination: output.clone(),
            },
        );
        assert_eq!(outcome.status, OperationStatus::Failure);
        assert!(!output.exists());
        assert_no_staging(&root.0);
    }
}

#[test]
fn zip_links_are_rejected_without_creating_output() {
    let root = TestDirectory::new();
    let archive = root.0.join("link.zip");
    let mut writer = zip::ZipWriter::new(fs::File::create(&archive).unwrap());
    writer
        .add_symlink(
            "link",
            "../outside",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
    writer.finish().unwrap();
    let output = root.0.join("output");
    assert_eq!(
        execute(
            &root.0,
            FileOperation::ExtractArchive {
                source: archive,
                destination: output.clone()
            }
        )
        .status,
        OperationStatus::Failure
    );
    assert!(!output.exists());
    assert_no_staging(&root.0);
}

#[test]
fn declared_extraction_size_is_checked_before_any_payload_is_written() {
    let root = TestDirectory::new();
    let archive = root.0.join("large.zip");
    let mut writer = zip::ZipWriter::new(fs::File::create(&archive).unwrap());
    for index in 0..18 {
        writer
            .start_file(
                format!("{index}.txt"),
                zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Stored),
            )
            .unwrap();
    }
    writer.finish().unwrap();
    let mut bytes = fs::read(&archive).unwrap();
    let headers = bytes
        .windows(4)
        .enumerate()
        .filter_map(|(index, signature)| (signature == b"PK\x01\x02").then_some(index))
        .collect::<Vec<_>>();
    assert_eq!(headers.len(), 18);
    for index in headers {
        bytes[index + 24..index + 28].copy_from_slice(&4_000_000_000u32.to_le_bytes());
    }
    fs::write(&archive, bytes).unwrap();

    let output = root.0.join("output");
    let outcome = execute(
        &root.0,
        FileOperation::ExtractArchive {
            source: archive,
            destination: output.clone(),
        },
    );
    assert_eq!(outcome.status, OperationStatus::Failure);
    assert!(outcome.message.contains("64 GiB"), "{}", outcome.message);
    assert!(!output.exists());
    assert_no_staging(&root.0);
}

#[test]
fn a_destination_created_during_work_is_preserved_on_publication() {
    use crate::file_operations::progress::OperationObserver;
    struct RacingDestination(PathBuf);
    impl OperationObserver for RacingDestination {
        fn phase(&mut self, phase: &str) {
            if phase == "Publishing" {
                fs::write(&self.0, "created by another process").unwrap();
            }
        }
    }

    let root = TestDirectory::new();
    let source = root.0.join("source");
    fs::write(&source, "archive data").unwrap();
    let destination = root.0.join("archive.zip");
    let request = FileOperationRequest {
        operation: FileOperation::CreateArchive {
            sources: vec![source],
            destination: destination.clone(),
        },
        protected_directory: root.0.clone(),
        refresh: refresh(&root.0),
    };
    let outcome = super::super::execute_operation_observed(
        &request,
        &mut RacingDestination(destination.clone()),
    );
    assert_eq!(outcome.status, OperationStatus::Failure);
    assert_eq!(
        fs::read_to_string(&destination).unwrap(),
        "created by another process"
    );
    assert_no_staging(&root.0);
}

#[cfg(windows)]
#[test]
fn reserved_windows_names_and_case_aliases_are_rejected() {
    let root = TestDirectory::new();
    let source = root.0.join("bad.zip");
    let output = root.0.join("output");
    for name in [
        "NUL.txt",
        "CON",
        "folder/COM1",
        "folder/COM¹",
        "CON .txt",
        "trailing.",
        "trailing ",
    ] {
        zip_file(&source, &[(name, b"data")]);
        assert_eq!(
            execute(
                &root.0,
                FileOperation::ExtractArchive {
                    source: source.clone(),
                    destination: output.clone()
                }
            )
            .status,
            OperationStatus::Failure
        );
        assert!(!output.exists());
        assert_no_staging(&root.0);
    }
    zip_file(&source, &[("Data.txt", b"first"), ("data.txt", b"second")]);
    assert_eq!(
        execute(
            &root.0,
            FileOperation::ExtractArchive {
                source,
                destination: output.clone()
            }
        )
        .status,
        OperationStatus::Failure
    );
    assert!(!output.exists());
}

#[test]
fn damaged_zip_payloads_roll_back_on_checksum_failure() {
    let root = TestDirectory::new();
    let archive = root.0.join("bad.zip");
    zip_file(&archive, &[("data", b"UNIQUE-CONTENT")]);
    let mut bytes = fs::read(&archive).unwrap();
    let position = bytes
        .windows(14)
        .position(|slice| slice == b"UNIQUE-CONTENT")
        .unwrap();
    bytes[position] ^= 1;
    fs::write(&archive, bytes).unwrap();
    let output = root.0.join("output");
    let outcome = execute(
        &root.0,
        FileOperation::ExtractArchive {
            source: archive,
            destination: output.clone(),
        },
    );
    assert_eq!(outcome.status, OperationStatus::Failure);
    assert!(!output.exists());
    assert_no_staging(&root.0);
}

#[test]
fn tar_extracts_data_and_rejects_links() {
    let root = TestDirectory::new();
    for link in [false, true] {
        let archive = root.0.join("fixture.tar");
        let mut builder = tar::Builder::new(fs::File::create(&archive).unwrap());
        let mut header = tar::Header::new_gnu();
        header.set_mode(0o755);
        header.set_size(if link { 0 } else { 4 });
        if link {
            header.set_entry_type(tar::EntryType::Symlink);
            header.set_link_name("../outside").unwrap();
        }
        header.set_cksum();
        builder
            .append_data(
                &mut header,
                "folder/data.txt",
                if link { b"".as_slice() } else { b"Wolf" },
            )
            .unwrap();
        builder.finish().unwrap();
        drop(builder);
        let output = root.0.join(if link { "bad" } else { "good" });
        let outcome = execute(
            &root.0,
            FileOperation::ExtractArchive {
                source: archive,
                destination: output.clone(),
            },
        );
        assert_eq!(
            outcome.status,
            if link {
                OperationStatus::Failure
            } else {
                OperationStatus::Success
            },
            "{}",
            outcome.message
        );
        if link {
            assert!(!output.exists());
        } else {
            assert_eq!(fs::read(output.join("folder/data.txt")).unwrap(), b"Wolf");
        }
        assert_no_staging(&root.0);
    }
}

#[test]
fn seven_zip_extracts_a_deterministic_lzma2_fixture() {
    // Generated from data.txt with 7-Zip; timestamps are omitted. No external tool is needed by the test.
    let encoded = "377ABCAF271C0004069D4C4E1B000000000000004E000000000000009C75455C010016436F6D6D616E64657220576F6C6620617263686976650A000104060001091B00070B010001212101000C1700080A013C84CA2B00000501190C00000000000000000000000011130064006100740061002E007400780074000000190015060100202000000000";
    let bytes = (0..encoded.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&encoded[index..index + 2], 16).unwrap())
        .collect::<Vec<_>>();
    let root = TestDirectory::new();
    let source = root.0.join("fixture.7z");
    fs::write(&source, bytes).unwrap();
    let output = root.0.join("output");
    let outcome = execute(
        &root.0,
        FileOperation::ExtractArchive {
            source,
            destination: output.clone(),
        },
    );
    assert_eq!(
        outcome.status,
        OperationStatus::Success,
        "{}",
        outcome.message
    );
    assert_eq!(
        fs::read_to_string(output.join("data.txt")).unwrap(),
        "Commander Wolf archive\n"
    );
    assert_no_staging(&root.0);
}

#[test]
fn creating_inside_a_selected_directory_changes_nothing() {
    let root = TestDirectory::new();
    let source = root.0.join("selected");
    fs::create_dir(&source).unwrap();
    let destination = source.join("archive.zip");
    let outcome = execute(
        &root.0,
        FileOperation::CreateArchive {
            sources: vec![source.clone()],
            destination: destination.clone(),
        },
    );
    assert_eq!(outcome.status, OperationStatus::Failure);
    assert!(!destination.exists());
    assert_no_staging(&source);
}

#[test]
fn cancellation_during_creation_and_extraction_removes_temporary_output() {
    let root = TestDirectory::new();
    let source = root.0.join("large.txt");
    fs::write(&source, vec![b'x'; 1024 * 1024]).unwrap();
    let archive = root.0.join("cancelled.zip");
    let operation = FileOperation::CreateArchive {
        sources: vec![source.clone()],
        destination: archive.clone(),
    };
    let outcome = cancel_during_copy(&root.0, operation.clone());
    assert_eq!(outcome.status, OperationStatus::Failure);
    assert!(!archive.exists());
    assert_eq!(fs::metadata(&source).unwrap().len(), 1024 * 1024);
    assert_no_staging(&root.0);

    assert_eq!(execute(&root.0, operation).status, OperationStatus::Success);
    let output = root.0.join("output");
    let outcome = cancel_during_copy(
        &root.0,
        FileOperation::ExtractArchive {
            source: archive,
            destination: output.clone(),
        },
    );
    assert_eq!(outcome.status, OperationStatus::Failure);
    assert!(!output.exists());
    assert_no_staging(&root.0);
}

#[test]
fn archive_progress_uses_uncompressed_totals_and_reaches_completion() {
    let root = TestDirectory::new();
    let source = root.0.join("data.txt");
    fs::write(&source, vec![b'x'; 1024 * 1024]).unwrap();
    let archive = root.0.join("archive.zip");
    for operation in [
        FileOperation::CreateArchive {
            sources: vec![source],
            destination: archive.clone(),
        },
        FileOperation::ExtractArchive {
            source: archive,
            destination: root.0.join("output"),
        },
    ] {
        let batch = batch(&root.0, operation);
        let mut progress = Vec::new();
        let outcome = crate::filesystem::execute_batch_with_progress(
            &batch,
            &AtomicBool::new(false),
            &mut |update| progress.push(update),
        );
        assert_eq!(
            outcome.status,
            OperationStatus::Success,
            "{}",
            outcome.message
        );
        let last = progress.last().unwrap();
        assert_eq!(last.total, 1024 * 1024);
        assert_eq!(last.total_done, last.total);
        assert!(
            progress
                .iter()
                .any(|update| update.file_done > 0 && update.file_done < update.file_total)
        );
    }
}

#[cfg(unix)]
#[test]
fn source_symlinks_are_not_followed_and_executable_permissions_round_trip() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let root = TestDirectory::new();
    let source = root.0.join("script");
    fs::write(&source, "#!/bin/sh\n").unwrap();
    fs::set_permissions(&source, fs::Permissions::from_mode(0o755)).unwrap();
    let archive = root.0.join("script.zip");
    assert_eq!(
        execute(
            &root.0,
            FileOperation::CreateArchive {
                sources: vec![source.clone()],
                destination: archive.clone()
            }
        )
        .status,
        OperationStatus::Success
    );
    let output = root.0.join("output");
    assert_eq!(
        execute(
            &root.0,
            FileOperation::ExtractArchive {
                source: archive,
                destination: output.clone()
            }
        )
        .status,
        OperationStatus::Success
    );
    assert_eq!(
        fs::metadata(output.join("script"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o755
    );

    let link = root.0.join("link");
    symlink(&source, &link).unwrap();
    let destination = root.0.join("link.zip");
    assert_eq!(
        execute(
            &root.0,
            FileOperation::CreateArchive {
                sources: vec![link],
                destination: destination.clone()
            }
        )
        .status,
        OperationStatus::Failure
    );
    assert!(!destination.exists());
    assert_no_staging(&root.0);
}

fn execute(root: &Path, operation: FileOperation) -> crate::file_operations::FileOperationOutcome {
    crate::filesystem::execute_operation(&FileOperationRequest {
        operation,
        protected_directory: root.to_owned(),
        refresh: refresh(root),
    })
}

fn batch(root: &Path, operation: FileOperation) -> FileOperationBatchRequest {
    FileOperationBatchRequest {
        operations: vec![operation],
        source_panel: PanelId::Left,
        protected_directory: root.to_owned(),
        refresh: refresh(root),
    }
}

fn refresh(root: &Path) -> [PanelRefreshRequest; 2] {
    [PanelId::Left, PanelId::Right].map(|panel| PanelRefreshRequest {
        panel,
        directory: root.to_owned(),
        selection: RefreshSelection {
            preferred: None,
            fallback_index: 0,
        },
    })
}

fn cancel_during_copy(
    root: &Path,
    operation: FileOperation,
) -> crate::file_operations::FileOperationOutcome {
    let cancel = AtomicBool::new(false);
    crate::filesystem::execute_batch_with_progress(
        &batch(root, operation),
        &cancel,
        &mut |progress| {
            if progress.file_done > 0 {
                cancel.store(true, Ordering::Relaxed);
            }
        },
    )
}

fn zip_file(path: &Path, entries: &[(&str, &[u8])]) {
    let mut archive = zip::ZipWriter::new(fs::File::create(path).unwrap());
    for (name, data) in entries {
        archive
            .start_file(
                *name,
                zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Stored),
            )
            .unwrap();
        archive.write_all(data).unwrap();
    }
    archive.finish().unwrap();
}

fn assert_no_staging(root: &Path) {
    assert!(fs::read_dir(root).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".commander-wolf-stage-")
    }));
}
