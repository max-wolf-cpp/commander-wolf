use super::*;

#[test]
fn explicitly_confirmed_move_replaces_the_destination() {
    let root = TestDirectory::new();
    let source = root.0.join("source.txt");
    let destination = root.0.join("destination.txt");
    fs::write(&source, "new contents").unwrap();
    fs::write(&destination, "old contents").unwrap();

    let outcome = execute_operation(&request(
        FileOperation::Move {
            source: source.clone(),
            destination: destination.clone(),
            overwrite: true,
        },
        &root.0,
    ));

    assert_eq!(
        outcome.status,
        crate::file_operations::OperationStatus::Success
    );

    assert!(!source.exists());
    assert_eq!(fs::read_to_string(destination).unwrap(), "new contents");
    assert_eq!(fs::read_dir(&root.0).unwrap().count(), 1);
}

#[test]
fn move_overwrite_failures_preserve_originals_or_report_the_backup() {
    for steps in [
        vec!["backup"],
        vec!["move"],
        vec!["move", "restore"],
        vec!["cleanup"],
    ] {
        let root = TestDirectory::new();
        let source = root.0.join("source");
        let destination = root.0.join("destination");
        fs::write(&source, "new").unwrap();
        fs::write(&destination, "old").unwrap();
        let restore_failed = steps.contains(&"restore");
        let moved = steps.contains(&"cleanup");

        let outcome = faults::with(
            move |phase, _| {
                if steps.contains(&phase) {
                    Err(io::Error::other(format!("injected {phase}")))
                } else {
                    Ok(())
                }
            },
            || {
                execute_operation(&request(
                    FileOperation::Move {
                        source: source.clone(),
                        destination: destination.clone(),
                        overwrite: true,
                    },
                    &root.0,
                ))
            },
        );

        assert_eq!(
            outcome.status,
            if moved || restore_failed {
                OperationStatus::Partial
            } else {
                OperationStatus::Failure
            }
        );

        assert_eq!(outcome.filesystem_changed, moved || restore_failed);
        if moved {
            assert!(!source.exists());
            assert_eq!(fs::read(&destination).unwrap(), b"new");
        } else {
            assert_eq!(fs::read(&source).unwrap(), b"new");
            if restore_failed {
                assert!(!destination.exists());
            } else {
                assert_eq!(fs::read(&destination).unwrap(), b"old");
            }
        }

        let leftovers = artifacts(&root.0);
        assert_eq!(leftovers.len(), usize::from(moved || restore_failed));
        for path in leftovers {
            assert_eq!(fs::read(&path).unwrap(), b"old");
            assert!(outcome.message.contains(path.to_str().unwrap()));
        }
    }
}

#[test]
fn cross_device_move_reports_actual_partial_source_deletion() {
    let root = TestDirectory::new();
    let source = root.0.join("source");
    let dest = root.0.join("dest");
    fs::create_dir(&source).unwrap();
    for name in ["a", "b"] {
        fs::write(source.join(name), name).unwrap();
    }
    let mut removed = 0;

    let outcome = faults::with(
        move |phase, path| {
            if phase == "remove_source" && path.is_file() {
                removed += 1;
                if removed == 2 {
                    return Err(io::Error::other("injected source deletion"));
                }
            }
            Ok(())
        },
        || {
            move_path_with(
                &source,
                &dest,
                |_, _| Err(io::Error::from(io::ErrorKind::CrossesDevices)),
                remove_source_observed,
                &mut NoopObserver,
            )
        },
    );

    assert_eq!(outcome.status, OperationStatus::Partial);
    assert!(outcome.filesystem_changed);
    assert_eq!(fs::read_dir(&source).unwrap().count(), 1);
    for name in ["a", "b"] {
        assert_eq!(fs::read_to_string(dest.join(name)).unwrap(), name);
    }
    assert!(artifacts(&root.0).is_empty());
}

#[test]
fn cross_device_move_and_partial_delete_are_deterministic() {
    let root = TestDirectory::new();
    let source = root.0.join("source.txt");
    let destination = root.0.join("destination.txt");
    fs::write(&source, "contents").unwrap();

    let outcome = move_path_with(
        &source,
        &destination,
        |_, _| Err(io::Error::from(io::ErrorKind::CrossesDevices)),
        |_, _| Err(io::Error::from(io::ErrorKind::PermissionDenied)),
        &mut NoopObserver,
    );

    assert_eq!(
        outcome.status,
        crate::file_operations::OperationStatus::Partial
    );

    assert!(source.exists());
    assert_eq!(fs::read_to_string(destination).unwrap(), "contents");
}
