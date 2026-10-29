use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;

use crate::app::Action;
use crate::preview::document::{DocumentOutput, DocumentRequest};

use super::cache::{CacheEntry, FileStamp, cache_matches, cached, file_stamp, insert_cache};

pub(super) enum WorkerMessage {
    Render(DocumentRequest),
    Shutdown,
}

pub(super) struct WorkerResult {
    pub(super) request: DocumentRequest,
    pub(super) output: Result<DocumentOutput, String>,
}

pub(super) fn run(
    requests: mpsc::Receiver<WorkerMessage>,
    results: mpsc::Sender<WorkerResult>,
    latest: &AtomicU64,
    mut loader: impl FnMut(&DocumentRequest) -> Result<DocumentOutput, String>,
) {
    let mut cache = VecDeque::new();
    while let Ok(message) = requests.recv() {
        let WorkerMessage::Render(mut request) = message else {
            break;
        };

        while let Ok(next) = requests.try_recv() {
            match next {
                WorkerMessage::Render(next) => request = next,
                WorkerMessage::Shutdown => return,
            }
        }

        if latest.load(Ordering::Acquire) != request.request_id {
            continue;
        }

        render_request(request, &mut cache, &mut loader, latest, &results);
    }
}

impl WorkerResult {
    pub(super) fn into_action(self) -> Action {
        let request = self.request;

        match self.output {
            Ok(output) => Action::PreviewDocumentLoaded {
                generation: request.generation,
                request_id: request.request_id,
                path: request.path,
                preview: output.preview,
                raster: output.raster,
            },
            Err(message) => Action::PreviewDocumentFailed {
                generation: request.generation,
                request_id: request.request_id,
                path: request.path,
                message,
            },
        }
    }
}

fn render_request(
    request: DocumentRequest,
    cache: &mut VecDeque<CacheEntry>,
    loader: &mut impl FnMut(&DocumentRequest) -> Result<DocumentOutput, String>,
    latest: &AtomicU64,
    results: &mpsc::Sender<WorkerResult>,
) {
    let stamp = file_stamp(&request);
    cache.retain(|entry: &CacheEntry| cache_matches(entry, &request, stamp.as_ref()));
    let output = cached(&request, cache)
        .map(Ok)
        .unwrap_or_else(|| load_safely(loader, &request));

    if latest.load(Ordering::Acquire) != request.request_id {
        return;
    }

    if let Ok(output) = &output {
        insert_cache(cache, &request, stamp.as_ref(), output.clone(), true);
    }

    let total_pages = output.as_ref().map_or(0, |value| value.preview.total_pages);
    let _ = results.send(WorkerResult {
        request: request.clone(),
        output,
    });

    prefetch_neighbors(&request, total_pages, stamp.as_ref(), cache, loader, latest);
}

fn prefetch_neighbors(
    request: &DocumentRequest,
    total_pages: usize,
    stamp: Option<&FileStamp>,
    cache: &mut VecDeque<CacheEntry>,
    loader: &mut impl FnMut(&DocumentRequest) -> Result<DocumentOutput, String>,
    latest: &AtomicU64,
) {
    for page in [request.page.checked_sub(1), request.page.checked_add(1)]
        .into_iter()
        .flatten()
    {
        if latest.load(Ordering::Acquire) != request.request_id {
            break;
        }

        if request.options.cache_pages <= 1 || page >= total_pages {
            continue;
        }

        let adjacent = DocumentRequest {
            page,
            ..request.clone()
        };
        if cached(&adjacent, cache).is_some() {
            continue;
        }

        // Library decoders are synchronous. Cancellation prevents subsequent
        // work and delivery, but cannot interrupt a decoder already executing.
        if let Ok(output) = load_safely(loader, &adjacent)
            && latest.load(Ordering::Acquire) == request.request_id
            && output.preview.current_page == page
        {
            insert_cache(cache, &adjacent, stamp, output, false);
        }
    }
}

fn load_safely(
    loader: &mut impl FnMut(&DocumentRequest) -> Result<DocumentOutput, String>,
    request: &DocumentRequest,
) -> Result<DocumentOutput, String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| loader(request)))
        .unwrap_or_else(|_| Err("Document decoder panicked".into()))
}
