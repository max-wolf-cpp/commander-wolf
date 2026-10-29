use super::*;

#[test]
fn copies_files_and_directories_without_overwrite() {
    let root = TestDirectory::new();
    let source = root.0.join("source");
    let destination = root.0.join("copy");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("狼.txt"), "contents").unwrap();

    let outcome = execute_operation(&request(
        FileOperation::Copy {
            source: source.clone(),
            destination: destination.clone(),
            overwrite: false,
        },
        &root.0,
    ));
    assert_eq!(
        outcome.status,
        crate::file_operations::OperationStatus::Success
    );

    assert_eq!(
        fs::read_to_string(destination.join("狼.txt")).unwrap(),
        "contents"
    );

    let conflict = execute_operation(&request(
        FileOperation::Copy {
            source,
            destination,
            overwrite: false,
        },
        &root.0,
    ));
    assert_eq!(
        conflict.status,
        crate::file_operations::OperationStatus::Failure
    );
}

#[test]
fn explicitly_confirmed_copy_replaces_existing_files_and_directories() {
    let root = TestDirectory::new();
    let source_file = root.0.join("new.txt");
    let destination_file = root.0.join("existing.txt");
    fs::write(&source_file, "new").unwrap();
    fs::write(&destination_file, "old").unwrap();

    let outcome = execute_operation(&request(
        FileOperation::Copy {
            source: source_file,
            destination: destination_file.clone(),
            overwrite: true,
        },
        &root.0,
    ));
    assert_eq!(outcome.status, OperationStatus::Success);
    assert_eq!(fs::read_to_string(&destination_file).unwrap(), "new");

    let source_directory = root.0.join("new-directory");
    let destination_directory = root.0.join("existing-directory");
    fs::create_dir(&source_directory).unwrap();
    fs::create_dir(&destination_directory).unwrap();
    fs::write(source_directory.join("new.txt"), "new").unwrap();
    fs::write(destination_directory.join("old.txt"), "old").unwrap();

    let outcome = execute_operation(&request(
        FileOperation::Copy {
            source: source_directory,
            destination: destination_directory.clone(),
            overwrite: true,
        },
        &root.0,
    ));
    assert_eq!(outcome.status, OperationStatus::Success);
    assert_eq!(
        fs::read_to_string(destination_directory.join("new.txt")).unwrap(),
        "new"
    );

    assert!(!destination_directory.join("old.txt").exists());
    assert!(fs::read_dir(&root.0).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".commander-wolf-stage-")
    }));
}

#[test]
fn publication_preserves_platform_metadata_without_hard_links() {
    let root = TestDirectory::new();
    let source = root.0.join("source");
    let dest = root.0.join("dest");
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .attributes(0x2 | 0x4)
            .open(&source)
            .unwrap();
    }
    fs::write(&source, "private contents").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&source, fs::Permissions::from_mode(0o600)).unwrap();
    }

    let result = execute_operation(&request(
        FileOperation::Copy {
            source: source.clone(),
            destination: dest.clone(),
            overwrite: false,
        },
        &root.0,
    ));
    assert_eq!(
        result.status,
        OperationStatus::Success,
        "{}",
        result.message
    );

    assert_eq!(fs::read(&source).unwrap(), fs::read(&dest).unwrap());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&dest).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        assert_eq!(fs::metadata(&dest).unwrap().file_attributes() & 0x6, 0x6);
    }
    assert!(artifacts(&root.0).is_empty());
}

#[cfg(windows)]
#[test]
fn observed_copy_preserves_alternate_streams() {
    let root = TestDirectory::new();
    let source = root.0.join("source.txt");
    let destination = root.0.join("destination.txt");
    fs::write(&source, "main contents").unwrap();
    fs::write(root.0.join("source.txt:details"), "alternate contents")
        .expect("This Windows test requires a filesystem with alternate stream support");

    let outcome = execute_operation(&request(
        FileOperation::Copy {
            source,
            destination: destination.clone(),
            overwrite: false,
        },
        &root.0,
    ));
    assert_eq!(
        outcome.status,
        OperationStatus::Success,
        "{}",
        outcome.message
    );

    assert_eq!(fs::read_to_string(destination).unwrap(), "main contents");
    assert_eq!(
        fs::read_to_string(root.0.join("destination.txt:details")).unwrap(),
        "alternate contents"
    );
}
