use super::*;

#[test]
fn transfer_errors_preserve_both_originals_and_report_failed_cleanup() {
    for step in ["read", "write"] {
        for cleanup_fails in [false, true] {
            let root = TestDirectory::new();
            let source = root.0.join("source");
            let dest = root.0.join("dest");
            let bytes = vec![47; 1024 * 1024];
            fs::write(&source, &bytes).unwrap();
            fs::write(&dest, "original").unwrap();

            let outcome = faults::with(
                move |phase, _| {
                    if phase == step || (cleanup_fails && phase == "cleanup") {
                        Err(io::Error::other(format!("injected {phase}")))
                    } else {
                        Ok(())
                    }
                },
                || {
                    execute_operation(&request(
                        FileOperation::Copy {
                            source: source.clone(),
                            destination: dest.clone(),
                            overwrite: true,
                        },
                        &root.0,
                    ))
                },
            );

            assert_ne!(outcome.status, OperationStatus::Success);
            assert!(outcome.message.contains(step));
            assert_eq!(fs::read(&source).unwrap(), bytes);
            assert_eq!(fs::read_to_string(&dest).unwrap(), "original");

            let leftovers = artifacts(&root.0);
            assert_eq!(outcome.filesystem_changed, !leftovers.is_empty());
            if !cleanup_fails {
                assert!(leftovers.is_empty());
            }
            for path in leftovers {
                assert!(outcome.message.contains(path.to_str().unwrap()));
            }
        }
    }
}

#[test]
fn publication_and_rollback_failures_preserve_recoverable_data() {
    for steps in [
        vec!["backup"],
        vec!["publish"],
        vec!["publish", "restore"],
        vec!["cleanup"],
    ] {
        let root = TestDirectory::new();
        let source = root.0.join("source");
        let dest = root.0.join("dest");
        fs::write(&source, "new").unwrap();
        fs::write(&dest, "old").unwrap();
        let restore_failed = steps.contains(&"restore");
        let cleanup_failed = steps.contains(&"cleanup");

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
                    FileOperation::Copy {
                        source: source.clone(),
                        destination: dest.clone(),
                        overwrite: true,
                    },
                    &root.0,
                ))
            },
        );

        assert_eq!(fs::read_to_string(&source).unwrap(), "new");
        assert_ne!(outcome.status, OperationStatus::Success);
        assert_eq!(outcome.filesystem_changed, restore_failed || cleanup_failed);
        if restore_failed {
            assert!(!dest.exists());
        } else {
            assert_eq!(
                fs::read_to_string(&dest).unwrap(),
                if cleanup_failed { "new" } else { "old" }
            );
        }

        let leftovers = artifacts(&root.0);
        assert_eq!(
            leftovers.len(),
            usize::from(restore_failed || cleanup_failed)
        );
        for path in leftovers {
            assert_eq!(fs::read_to_string(&path).unwrap(), "old");
            assert!(outcome.message.contains(path.to_str().unwrap()));
        }
    }
}

#[test]
fn destination_races_never_overwrite_new_files_or_directories() {
    for directory in [false, true] {
        for kind in 0..3 {
            let root = TestDirectory::new();
            let source = root.0.join("source");
            let dest = root.0.join("dest");
            if directory {
                fs::create_dir(&source).unwrap();
                fs::write(source.join("payload"), "source").unwrap();
            } else {
                fs::write(&source, "source").unwrap();
            }
            let operation = match kind {
                0 => FileOperation::Copy {
                    source: source.clone(),
                    destination: dest.clone(),
                    overwrite: false,
                },
                1 => FileOperation::Move {
                    source: source.clone(),
                    destination: dest.clone(),
                    overwrite: false,
                },
                _ => FileOperation::Rename {
                    source: source.clone(),
                    destination: dest.clone(),
                },
            };

            let outcome = faults::with(
                move |phase, path| {
                    if phase == "validated" {
                        if directory {
                            fs::create_dir(path)?;
                            fs::write(path.join("payload"), "racer")?;
                        } else {
                            fs::write(path, "racer")?;
                        }
                    }
                    Ok(())
                },
                || execute_operation(&request(operation, &root.0)),
            );

            assert_eq!(
                outcome.status,
                OperationStatus::Failure,
                "{}",
                outcome.message
            );

            assert!(!outcome.filesystem_changed);
            assert_eq!(
                fs::read_to_string(if directory {
                    source.join("payload")
                } else {
                    source
                })
                .unwrap(),
                "source"
            );

            assert_eq!(
                fs::read_to_string(if directory {
                    dest.join("payload")
                } else {
                    dest
                })
                .unwrap(),
                "racer"
            );

            assert!(artifacts(&root.0).is_empty());
        }
    }
}

#[test]
fn rollback_does_not_delete_a_racing_destination() {
    let root = TestDirectory::new();
    let source = root.0.join("source");
    let dest = root.0.join("dest");
    fs::write(&source, "new").unwrap();
    fs::write(&dest, "old").unwrap();

    let outcome = faults::with(
        |phase, path| {
            if phase == "publish" {
                fs::write(path, "racer")?;
            }
            Ok(())
        },
        || {
            execute_operation(&request(
                FileOperation::Copy {
                    source: source.clone(),
                    destination: dest.clone(),
                    overwrite: true,
                },
                &root.0,
            ))
        },
    );

    assert!(outcome.filesystem_changed);
    assert_eq!(fs::read_to_string(&source).unwrap(), "new");
    assert_eq!(fs::read_to_string(&dest).unwrap(), "racer");

    let backup = artifacts(&root.0);
    assert_eq!(backup.len(), 1);
    assert_eq!(fs::read_to_string(&backup[0]).unwrap(), "old");
    assert!(outcome.message.contains(backup[0].to_str().unwrap()));
}

#[test]
fn failed_publication_and_cleanup_reports_the_retained_staging_contents() {
    let root = TestDirectory::new();
    let source = root.0.join("source");
    let dest = root.0.join("dest");
    fs::write(&source, "recover this data").unwrap();

    let outcome = faults::with(
        |phase, _| {
            if matches!(phase, "publish" | "cleanup") {
                Err(io::Error::other("controlled failure"))
            } else {
                Ok(())
            }
        },
        || {
            execute_operation(&request(
                FileOperation::Copy {
                    source: source.clone(),
                    destination: dest.clone(),
                    overwrite: false,
                },
                &root.0,
            ))
        },
    );

    assert_eq!(outcome.status, OperationStatus::Partial);
    assert!(outcome.filesystem_changed);
    assert!(!dest.exists());
    assert_eq!(fs::read_to_string(source).unwrap(), "recover this data");

    let retained = artifacts(&root.0);
    assert_eq!(retained.len(), 1);
    assert_eq!(
        fs::read_to_string(&retained[0]).unwrap(),
        "recover this data"
    );

    assert!(outcome.message.contains(retained[0].to_str().unwrap()));
}
