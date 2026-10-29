use super::*;

#[test]
fn directory_tree_loads_lazily_and_reveals_the_selected_entry() {
    let mut state = state();
    state.workspace.active_mut().directory = PathBuf::from("root");

    let effects = update(&mut state, Action::TreeToggle);
    assert!(state.directory_tree.open);
    assert!(
        matches!(effects.last(), Some(Effect::LoadTreeChildren { parent, .. }) if parent == std::path::Path::new("root"))
    );

    let generation = state.directory_tree.generation;
    update(
        &mut state,
        Action::TreeChildrenLoaded {
            generation,
            parent: PathBuf::from("root"),
            entries: vec![TreeEntry {
                is_hidden: false,
                path: PathBuf::from("root/child"),
                name: "child".into(),
                depth: 0,
                is_directory: true,
                expanded: false,
                loading: false,
            }],
        },
    );

    state.directory_tree.selected = 1;
    assert!(matches!(
        update(&mut state, Action::TreeToggleSelected).as_slice(),
        [Effect::LoadTreeChildren { parent, .. }] if parent == std::path::Path::new("root/child")
    ));
    assert!(matches!(
        update(&mut state, Action::TreeOpenSelected).as_slice(),
        [Effect::RevealTreeEntry { path, is_directory: true, .. }] if path == std::path::Path::new("root/child")
    ));
}
