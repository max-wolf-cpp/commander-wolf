use super::*;

#[cfg(unix)]
#[test]
fn deleting_a_symlink_never_deletes_its_target() {
    use std::os::unix::fs::symlink;

    let root = TestDirectory::new();
    let target = root.0.join("target.txt");
    let link = root.0.join("link.txt");
    fs::write(&target, "keep").unwrap();
    symlink(&target, &link).unwrap();

    let outcome = execute_operation(&request(
        FileOperation::Delete {
            source: link.clone(),
        },
        &root.0,
    ));
    assert_eq!(
        outcome.status,
        crate::file_operations::OperationStatus::Success
    );

    assert!(!link.exists());
    assert_eq!(fs::read_to_string(target).unwrap(), "keep");
}

#[cfg(windows)]
#[test]
#[ignore = "requires Windows Developer Mode or symlink privilege"]
fn platform_symlink_deletion() {
    use std::os::windows::fs::symlink_file;

    let root = TestDirectory::new();
    let target = root.0.join("target.txt");
    let link = root.0.join("link.txt");
    fs::write(&target, "keep").unwrap();
    symlink_file(&target, &link).expect("environment requires symlink creation privilege");

    let outcome = execute_operation(&request(
        FileOperation::Delete {
            source: link.clone(),
        },
        &root.0,
    ));
    assert_eq!(
        outcome.status,
        crate::file_operations::OperationStatus::Success
    );

    assert!(!link.exists());
    assert_eq!(fs::read_to_string(target).unwrap(), "keep");
}

#[test]
#[cfg_attr(
    windows,
    ignore = "requires Windows Developer Mode or symlink privilege"
)]
fn platform_directory_and_broken_links_preserve_targets_and_copy_cleans_staging() {
    let root = TestDirectory::new();
    let target = root.0.join("target");
    let link = root.0.join("link");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("payload"), "keep").unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(&target, &link)
        .expect("environment requires directory symlink privilege");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&target, &link).unwrap();
    let entries = crate::filesystem::tree::read_children(&root.0, true).unwrap();
    assert!(
        !entries
            .iter()
            .find(|entry| entry.path == link)
            .unwrap()
            .is_directory
    );

    let outcome = execute_operation(&request(
        FileOperation::Delete {
            source: link.clone(),
        },
        &root.0,
    ));
    assert_eq!(
        outcome.status,
        OperationStatus::Success,
        "{}",
        outcome.message
    );

    assert_eq!(fs::read_to_string(target.join("payload")).unwrap(), "keep");
    let missing = root.0.join("absent");
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(&missing, &link)
        .expect("environment requires file symlink privilege");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&missing, &link).unwrap();
    let destination = root.0.join("copy");

    let outcome = execute_operation(&request(
        FileOperation::Copy {
            source: link.clone(),
            destination: destination.clone(),
            overwrite: false,
        },
        &root.0,
    ));
    assert_eq!(outcome.status, OperationStatus::Failure);
    assert!(outcome.message.contains("symlink"));
    assert!(!destination.exists());
    assert!(artifacts(&root.0).is_empty());
    assert!(fs::symlink_metadata(&link).is_ok());

    let outcome = execute_operation(&request(
        FileOperation::Delete {
            source: link.clone(),
        },
        &root.0,
    ));
    assert_eq!(outcome.status, OperationStatus::Success);
    assert!(fs::symlink_metadata(link).is_err());
}

#[cfg(windows)]
#[test]
#[ignore = "requires Windows with junction support on the temporary filesystem"]
fn platform_junction_is_a_leaf_and_deletion_preserves_target() {
    use std::os::windows::process::CommandExt;
    let root = TestDirectory::new();
    let target = root.0.join("target");
    let link = root.0.join("junction");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("payload"), "keep").unwrap();

    let result = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(&link)
        .arg(&target)
        .creation_flags(0x08000000)
        .output()
        .expect("environment requires cmd mklink");
    assert!(
        result.status.success(),
        "junction creation failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let entries = crate::filesystem::tree::expand_all(&root.0, true).unwrap();
    assert!(
        !entries
            .iter()
            .any(|entry| entry.path == link.join("payload"))
    );

    let outcome = execute_operation(&request(FileOperation::Delete { source: link }, &root.0));
    assert_eq!(
        outcome.status,
        OperationStatus::Success,
        "{}",
        outcome.message
    );

    assert_eq!(fs::read_to_string(target.join("payload")).unwrap(), "keep");
}
