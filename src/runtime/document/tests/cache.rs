use super::*;

#[test]
fn cache_invalidates_refresh_geometry_options_replacement_and_deletion() {
    let temp = TempDirectory::new();
    let mut request = request(temp.join("book"));
    std::fs::write(&request.path, "first").unwrap();

    let mut cache = VecDeque::new();
    insert_cache(
        &mut cache,
        &request,
        file_stamp(&request).as_ref(),
        output(&request),
        true,
    );
    assert!(cached(&request, &cache).is_some());

    let original = request.clone();
    for change in 0..7 {
        request = original.clone();
        match change {
            0 => request.generation += 1,
            1 => request.pixel_width += 1,
            2 => request.pixel_height += 1,
            3 => request.text_columns += 1,
            4 => request.text_rows += 1,
            5 => request.options.max_render_pixels = 1,
            _ => request.format = DocumentFormat::Pdf,
        }
        assert!(cached(&request, &cache).is_none());
    }

    request = original;
    std::fs::write(&request.path, "replacement with different bytes").unwrap();
    assert!(cached(&request, &cache).is_none());
    std::fs::remove_file(&request.path).unwrap();
    assert!(cached(&request, &cache).is_none());
}

#[test]
fn live_requests_never_reuse_deleted_or_replaced_file_contents() {
    let temp = TempDirectory::new();
    let path = temp.join("book");
    std::fs::write(&path, "original").unwrap();

    let runtime = DocumentRuntime::with_loader(|request| {
        let text = std::fs::read_to_string(&request.path).map_err(|error| error.to_string())?;
        let mut output = output(request);
        output.preview.content = DocumentPageContent::Text(vec![text]);
        Ok(output)
    });

    let mut request = request(path.clone());
    for (id, expected) in [
        (1, "original"),
        (2, "new contents after refresh"),
        (4, "recreated"),
    ] {
        if id != 1 {
            std::fs::write(&path, expected).unwrap();
            request.generation += 1;
        }
        request.request_id = id;
        runtime.start(request.clone());
        assert!(
            matches!(event(&runtime), Action::PreviewDocumentLoaded { preview, .. } if preview.content == DocumentPageContent::Text(vec![expected.into()]))
        );
        if id == 2 {
            std::fs::remove_file(&path).unwrap();
            request.request_id = 3;
            runtime.start(request.clone());
            assert!(matches!(
                event(&runtime),
                Action::PreviewDocumentFailed { request_id: 3, .. }
            ));
        }
    }
}

#[test]
fn cache_budget_keeps_current_page_and_releases_evicted_rasters() {
    let temp = TempDirectory::new();
    let mut request = request(temp.join("book"));
    std::fs::write(&request.path, "fixture").unwrap();
    request.options.cache_pages = 1;

    let mut cache = VecDeque::new();
    let mut frame = output(&request);
    let pixels: Arc<[u8]> = Arc::from([1, 2, 3, 255]);
    let weak = Arc::downgrade(&pixels);
    frame.raster = Some(RasterFrame {
        width: 1,
        height: 1,
        stride: 4,
        pixels,
    });
    insert_cache(
        &mut cache,
        &request,
        file_stamp(&request).as_ref(),
        frame,
        true,
    );

    let mut next = request.clone();
    next.page = 1;
    insert_cache(
        &mut cache,
        &next,
        file_stamp(&next).as_ref(),
        output(&next),
        false,
    );
    assert_eq!(cache.len(), 1);
    assert!(cached(&request, &cache).is_some());
    assert!(weak.upgrade().is_some());

    insert_cache(
        &mut cache,
        &next,
        file_stamp(&next).as_ref(),
        output(&next),
        true,
    );
    assert!(weak.upgrade().is_none());
}
