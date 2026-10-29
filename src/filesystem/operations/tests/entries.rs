use super::*;

#[test]
fn create_file_never_truncates_an_existing_file() {
    let root = TestDirectory::new();
    let created = root.0.join("created.txt");
    assert_eq!(
        execute_operation(&request(
            FileOperation::CreateFile {
                destination: created.clone(),
            },
            &root.0,
        ))
        .status,
        crate::file_operations::OperationStatus::Success
    );

    assert_eq!(fs::metadata(created).unwrap().len(), 0);
    let path = root.0.join("existing.txt");
    fs::write(&path, "keep").unwrap();

    let outcome = execute_operation(&request(
        FileOperation::CreateFile {
            destination: path.clone(),
        },
        &root.0,
    ));
    assert_eq!(
        outcome.status,
        crate::file_operations::OperationStatus::Failure
    );

    assert_eq!(fs::read_to_string(path).unwrap(), "keep");
}

#[test]
fn moves_renames_creates_and_recursively_deletes() {
    let root = TestDirectory::new();
    let source = root.0.join("move.txt");
    let moved = root.0.join("moved.txt");
    fs::write(&source, "move").unwrap();
    assert_eq!(
        execute_operation(&request(
            FileOperation::Move {
                source: source.clone(),
                destination: moved.clone(),
                overwrite: false,
            },
            &root.0,
        ))
        .status,
        crate::file_operations::OperationStatus::Success
    );

    assert!(!source.exists());

    let renamed = root.0.join("новое имя.txt");
    assert_eq!(
        execute_operation(&request(
            FileOperation::Rename {
                source: moved,
                destination: renamed.clone(),
            },
            &root.0,
        ))
        .status,
        crate::file_operations::OperationStatus::Success
    );
    let directory = root.0.join("created");
    assert_eq!(
        execute_operation(&request(
            FileOperation::CreateDirectory {
                destination: directory.clone(),
            },
            &root.0,
        ))
        .status,
        crate::file_operations::OperationStatus::Success
    );
    fs::create_dir(directory.join("nested")).unwrap();
    fs::write(directory.join("nested/file"), "data").unwrap();
    assert_eq!(
        execute_operation(&request(
            FileOperation::Delete {
                source: directory.clone(),
            },
            &root.0,
        ))
        .status,
        crate::file_operations::OperationStatus::Success
    );

    assert!(!directory.exists());
    assert!(renamed.exists());

    let conflict_source = root.0.join("rename-source.txt");
    fs::write(&conflict_source, "source").unwrap();

    let conflict = execute_operation(&request(
        FileOperation::Rename {
            source: conflict_source.clone(),
            destination: renamed,
        },
        &root.0,
    ));
    assert_eq!(
        conflict.status,
        crate::file_operations::OperationStatus::Failure
    );

    assert!(conflict_source.exists());
}

#[test]
fn disappearing_source_returns_a_typed_failure() {
    let root = TestDirectory::new();

    let outcome = execute_operation(&request(
        FileOperation::Delete {
            source: root.0.join("already-gone"),
        },
        &root.0,
    ));
    assert_eq!(
        outcome.status,
        crate::file_operations::OperationStatus::Failure
    );

    assert!(!outcome.filesystem_changed);
    assert!(outcome.message.contains("already-gone"));
}

#[test]
fn rejects_self_descendants_and_protects_the_other_panel_directory() {
    let root = TestDirectory::new();
    let source = root.0.join("source");
    let inside = source.join("inside");
    fs::create_dir(&source).unwrap();
    fs::create_dir(&inside).unwrap();
    let copy = request(
        FileOperation::Copy {
            source: source.clone(),
            destination: inside.join("copy"),
            overwrite: false,
        },
        &root.0,
    );

    let outcome = execute_operation(&copy);
    assert_eq!(
        outcome.status,
        crate::file_operations::OperationStatus::Failure
    );

    assert!(outcome.message.contains("own subdirectory"));

    for operation in [
        FileOperation::Move {
            source: source.clone(),
            destination: root.0.join("moved"),
            overwrite: false,
        },
        FileOperation::Delete {
            source: source.clone(),
        },
        FileOperation::Rename {
            source: source.clone(),
            destination: root.0.join("renamed"),
        },
    ] {
        let mut protected = request(operation, &root.0);
        protected.protected_directory = inside.clone();

        let outcome = execute_operation(&protected);
        assert_eq!(
            outcome.status,
            crate::file_operations::OperationStatus::Failure
        );

        assert!(outcome.message.contains("other panel"));
    }
}

#[test]
fn batch_attempts_every_independent_operation_and_reports_partial_results() {
    let root = TestDirectory::new();
    let source = root.0.join("source");
    let destination = root.0.join("destination");
    fs::create_dir_all(&source).unwrap();
    fs::create_dir_all(&destination).unwrap();
    fs::write(source.join("blocked.txt"), "blocked").unwrap();
    fs::write(source.join("copied.txt"), "copied").unwrap();
    fs::write(destination.join("blocked.txt"), "existing").unwrap();
    let refresh = [PanelId::Left, PanelId::Right].map(|panel| PanelRefreshRequest {
        panel,
        directory: root.0.clone(),
        selection: RefreshSelection {
            preferred: None,
            fallback_index: 0,
        },
    });

    let outcome = execute_batch(&FileOperationBatchRequest {
        operations: vec![
            FileOperation::Copy {
                source: source.join("blocked.txt"),
                destination: destination.join("blocked.txt"),
                overwrite: false,
            },
            FileOperation::Copy {
                source: source.join("copied.txt"),
                destination: destination.join("copied.txt"),
                overwrite: false,
            },
        ],
        source_panel: PanelId::Left,
        protected_directory: destination.clone(),
        refresh,
    });

    assert_eq!(outcome.status, OperationStatus::Partial);
    assert!(outcome.filesystem_changed);
    assert!(outcome.message.contains("1 succeeded, 1 failed"));
    assert_eq!(
        fs::read_to_string(destination.join("copied.txt")).unwrap(),
        "copied"
    );
}
