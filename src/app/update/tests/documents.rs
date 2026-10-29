use super::*;

#[test]
fn document_failure_leaves_loading_and_stale_failure_cannot_replace_new_request() {
    let mut state = state();
    state.workspace.preview_visible = true;
    let path = PathBuf::from("book.pdf");
    let generation = state.preview.begin(path.clone(), "book.pdf".into());

    for message in [
        "decoder failed",
        "Document decoder panicked",
        "Document worker disconnected",
    ] {
        state.preview.document_request_id += 1;
        state.preview.document = LoadState::Loading;
        let request_id = state.preview.document_request_id;

        update(
            &mut state,
            Action::PreviewDocumentFailed {
                generation,
                request_id: request_id - 1,
                path: path.clone(),
                message: "stale failure".into(),
            },
        );
        assert!(matches!(state.preview.document, LoadState::Loading));

        update(
            &mut state,
            Action::PreviewDocumentFailed {
                generation,
                request_id,
                path: path.clone(),
                message: message.into(),
            },
        );
        assert!(
            matches!(&state.preview.document, LoadState::Unavailable(error) if error == message)
        );
    }
}

#[test]
fn document_navigation_clamps_counts_and_ignores_stale_requests() {
    let mut state = state();
    state.workspace.preview_visible = true;
    let path = PathBuf::from("book.pdf");
    let generation = state.preview.begin(path.clone(), "book.pdf".into());
    state.preview.document_request_id = 7;
    let document = crate::preview::DocumentPreview {
        format: DocumentFormat::Pdf,
        current_page: 1,
        total_pages: 4,
        page_width: Some(600),
        page_height: Some(800),
        title: None,
        author: None,
        content: DocumentPageContent::Text(vec!["page".into()]),
    };

    update(
        &mut state,
        Action::PreviewDocumentLoaded {
            generation,
            request_id: 6,
            path: path.clone(),
            preview: document.clone(),
            raster: None,
        },
    );
    assert!(matches!(state.preview.document, LoadState::Idle));

    update(
        &mut state,
        Action::PreviewDocumentLoaded {
            generation,
            request_id: 7,
            path: path.clone(),
            preview: document,
            raster: None,
        },
    );
    let effects = update(&mut state, Action::PreviewPageNext { count: 20 });
    let Some(Effect::StartDocumentPreview(request)) = effects.last() else {
        panic!("document request expected")
    };

    assert_eq!(request.page, 3);

    state.preview.document = LoadState::Ready(crate::preview::DocumentPreview {
        format: DocumentFormat::Pdf,
        current_page: 0,
        total_pages: 4,
        page_width: None,
        page_height: None,
        title: None,
        author: None,
        content: DocumentPageContent::Text(Vec::new()),
    });
    assert!(update(&mut state, Action::PreviewPagePrevious { count: 1 }).is_empty());
    assert!(
        state
            .notification
            .as_ref()
            .is_some_and(|notification| { notification.message.contains("first document page") })
    );

    state.preview.document = LoadState::Loading;
    state.preview.document_page = 2;
    let previous_request = state.preview.document_request_id;
    let effects = update(
        &mut state,
        Action::Resize {
            size: TerminalSize {
                columns: 120,
                rows: 40,
            },
            panel_rows: 35,
            finder_rows: 20,
            tree_rows: 20,
        },
    );
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, Effect::CancelDocumentPreview))
    );
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::StartDocumentPreview(request)
            if request.page == 2 && request.request_id > previous_request
    )));
}

#[test]
fn document_counts_larger_than_isize_keep_the_requested_direction() {
    for count in [isize::MAX as usize, isize::MAX as usize + 1, usize::MAX] {
        for forward in [false, true] {
            let mut state = state();
            state.workspace.preview_visible = true;
            state.preview.begin("book.fb2".into(), "book.fb2".into());
            state.preview.document = LoadState::Ready(crate::preview::DocumentPreview {
                format: DocumentFormat::Fb2,
                current_page: 1,
                total_pages: 4,
                page_width: None,
                page_height: None,
                title: None,
                author: None,
                content: DocumentPageContent::Text(vec!["page".into()]),
            });

            let action = if forward {
                Action::PreviewPageNext { count }
            } else {
                Action::PreviewPagePrevious { count }
            };
            let effects = update(&mut state, action);
            assert!(
                matches!(effects.last(), Some(Effect::StartDocumentPreview(request)) if request.page == if forward { 3 } else { 0 })
            );
        }
    }
}
