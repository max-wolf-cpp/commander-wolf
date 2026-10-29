use std::collections::VecDeque;
use std::time::{Duration, Instant};

use super::cache::{cached, file_stamp, insert_cache};
use super::*;
use crate::preview::document::{
    DocumentFormat, DocumentOptions, DocumentPageContent, DocumentPreview, RasterFrame,
};
use crate::test_support::TempDirectory;

mod cache;
mod worker;

fn request(path: std::path::PathBuf) -> DocumentRequest {
    DocumentRequest {
        generation: 1,
        request_id: 1,
        path,
        format: DocumentFormat::Fb2,
        page: 0,
        pixel_width: 80,
        pixel_height: 40,
        text_columns: 20,
        text_rows: 4,
        options: DocumentOptions::default(),
    }
}

fn output(request: &DocumentRequest) -> DocumentOutput {
    DocumentOutput {
        preview: DocumentPreview {
            format: request.format,
            current_page: request.page,
            total_pages: 1,
            page_width: None,
            page_height: None,
            title: None,
            author: None,
            content: DocumentPageContent::Text(vec![format!("request {}", request.request_id)]),
        },
        raster: None,
    }
}

fn event(runtime: &DocumentRuntime) -> Action {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let mut actions = runtime.poll();
        assert!(actions.len() <= 1, "duplicate terminal result");
        if let Some(action) = actions.pop() {
            return action;
        }

        assert!(Instant::now() < deadline, "document remained in Loading");
        thread::yield_now();
    }
}
