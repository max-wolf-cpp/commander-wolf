use super::*;

#[test]
fn decoder_error_and_panic_are_terminal_and_next_request_recovers() {
    let runtime = DocumentRuntime::with_loader(|request| match request.request_id {
        1 => panic!("controlled decoder panic"),
        2 => Err("damaged fixture".into()),
        _ => Ok(output(request)),
    });

    let mut request = request("missing.fb2".into());
    for id in 1..=3 {
        request.request_id = id;
        runtime.start(request.clone());
        let action = event(&runtime);
        if id < 3 {
            assert!(
                matches!(action, Action::PreviewDocumentFailed { request_id, .. } if request_id == id)
            );
        } else {
            assert!(matches!(
                action,
                Action::PreviewDocumentLoaded { request_id: 3, .. }
            ));
        }
        assert!(runtime.poll().is_empty());
    }
}

#[test]
fn latest_request_wins_and_cancel_does_not_claim_to_interrupt_decoder() {
    let (entered, calls) = mpsc::channel();
    let (release, gate) = mpsc::channel();
    let runtime = DocumentRuntime::with_loader(move |request| {
        entered.send(request.request_id).unwrap();
        if request.request_id == 1 {
            gate.recv().unwrap();
        }
        Ok(output(request))
    });

    let mut request = request("fixture.fb2".into());
    runtime.start(request.clone());
    assert_eq!(calls.recv_timeout(Duration::from_secs(5)).unwrap(), 1);

    runtime.cancel();
    assert!(runtime.poll().is_empty());

    request.request_id = 2;
    runtime.start(request.clone());
    request.request_id = 3;
    runtime.start(request);
    release.send(()).unwrap();
    assert_eq!(calls.recv_timeout(Duration::from_secs(5)).unwrap(), 3);
    assert!(matches!(
        event(&runtime),
        Action::PreviewDocumentLoaded { request_id: 3, .. }
    ));
    assert!(calls.try_recv().is_err());
}

#[test]
fn shutdown_joins_active_decoder_and_disconnection_reports_once() {
    let (entered, calls) = mpsc::channel();
    let (release, gate) = mpsc::channel();
    let mut runtime = DocumentRuntime::with_loader(move |request| {
        entered.send(()).unwrap();
        gate.recv().unwrap();
        Ok(output(request))
    });

    runtime.start(request("fixture.fb2".into()));
    calls.recv_timeout(Duration::from_secs(5)).unwrap();

    let shutdown = thread::spawn(move || {
        runtime.shutdown();
        runtime
    });
    release.send(()).unwrap();
    let runtime = shutdown.join().unwrap();
    assert!(runtime.handle.is_none());
    assert!(runtime.poll().is_empty());

    runtime.start(request("fixture.fb2".into()));
    assert!(
        matches!(event(&runtime), Action::PreviewDocumentFailed { message, .. } if message.contains("disconnected"))
    );
    assert!(runtime.poll().is_empty());
}

#[test]
fn new_request_preempts_remaining_prefetch_after_active_call_returns() {
    let (entered, calls) = mpsc::channel();
    let (release, gate) = mpsc::channel();
    let runtime = DocumentRuntime::with_loader(move |request| {
        entered.send((request.request_id, request.page)).unwrap();
        if request.request_id == 1 && request.page == 0 {
            gate.recv().unwrap();
        }
        let mut result = output(request);
        result.preview.total_pages = 3;
        Ok(result)
    });

    let mut request = request("fixture.fb2".into());
    request.page = 1;
    runtime.start(request.clone());
    assert_eq!(calls.recv_timeout(Duration::from_secs(5)).unwrap(), (1, 1));
    assert_eq!(calls.recv_timeout(Duration::from_secs(5)).unwrap(), (1, 0));

    request.request_id = 2;
    request.page = 2;
    request.options.cache_pages = 1;
    runtime.start(request);
    release.send(()).unwrap();
    assert_eq!(calls.recv_timeout(Duration::from_secs(5)).unwrap(), (2, 2));
    assert!(matches!(
        event(&runtime),
        Action::PreviewDocumentLoaded { request_id: 2, .. }
    ));
    assert!(calls.try_recv().is_err());
}
