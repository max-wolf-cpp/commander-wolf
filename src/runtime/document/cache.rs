use std::collections::VecDeque;

use crate::preview::document::{DocumentOutput, DocumentRequest};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FileStamp {
    length: u64,
    modified: Option<std::time::SystemTime>,
    created: Option<std::time::SystemTime>,
}

pub(super) struct CacheEntry {
    pub(super) request: DocumentRequest,
    pub(super) stamp: FileStamp,
    pub(super) output: DocumentOutput,
}

pub(super) fn file_stamp(request: &DocumentRequest) -> Option<FileStamp> {
    let metadata = std::fs::metadata(&request.path).ok()?;

    Some(FileStamp {
        length: metadata.len(),
        modified: metadata.modified().ok(),
        created: metadata.created().ok(),
    })
}

pub(super) fn cache_matches(
    entry: &CacheEntry,
    request: &DocumentRequest,
    stamp: Option<&FileStamp>,
) -> bool {
    let old = &entry.request;

    stamp == Some(&entry.stamp)
        && old.path == request.path
        && old.generation == request.generation
        && old.format == request.format
        && old.options == request.options
        && old.pixel_width == request.pixel_width
        && old.pixel_height == request.pixel_height
        && old.text_columns == request.text_columns
        && old.text_rows == request.text_rows
}

pub(super) fn cached(
    request: &DocumentRequest,
    cache: &VecDeque<CacheEntry>,
) -> Option<DocumentOutput> {
    let stamp = file_stamp(request);

    cache
        .iter()
        .find(|entry| {
            entry.request.page == request.page && cache_matches(entry, request, stamp.as_ref())
        })
        .map(|entry| entry.output.clone())
}

pub(super) fn insert_cache(
    cache: &mut VecDeque<CacheEntry>,
    request: &DocumentRequest,
    stamp: Option<&FileStamp>,
    output: DocumentOutput,
    current: bool,
) {
    let Some(stamp) = stamp else {
        return;
    };

    if file_stamp(request).as_ref() != Some(stamp) {
        return;
    }

    cache.retain(|entry| entry.request.page != request.page);
    let entry = CacheEntry {
        request: request.clone(),
        stamp: stamp.clone(),
        output,
    };
    if current {
        cache.push_front(entry);
    } else {
        cache.push_back(entry);
    }

    cache.truncate(request.options.cache_pages);
}
