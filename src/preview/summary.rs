use std::time::Duration;

use super::{
    DocumentPreview, LoadState, MediaMetadata, MetadataRow, PlaybackState, PreviewKind,
    PreviewState,
};

pub(super) fn metadata_rows(preview: &PreviewState) -> Vec<MetadataRow> {
    let mut rows = match &preview.metadata {
        LoadState::Ready(metadata) => metadata.rows(),
        LoadState::Unavailable(message) => vec![MetadataRow::field("Metadata", message)],
        LoadState::Loading => vec![MetadataRow::field("Metadata", "Loading…")],
        LoadState::Idle => Vec::new(),
    };

    if let LoadState::Ready(media) = &preview.media {
        append_media_rows(&mut rows, media, preview);
    }

    if let LoadState::Ready(document) = &preview.document {
        append_document_rows(&mut rows, document);
    }

    rows
}

fn append_media_rows(rows: &mut Vec<MetadataRow>, media: &MediaMetadata, preview: &PreviewState) {
    if media.kind == PreviewKind::Audio {
        replace_file_type(rows, "Audio");
    } else {
        rows.push(MetadataRow::field(
            "Resolution",
            format!("{} × {}", media.pixel_width, media.pixel_height),
        ));
    }

    if let Some(duration) = media.duration {
        rows.push(MetadataRow::field("Duration", format_duration(duration)));
    }
    if let Some(has_audio) = media.has_audio {
        rows.push(MetadataRow::field(
            "Audio",
            if has_audio {
                "Available"
            } else {
                "Not available"
            },
        ));
    }

    append_playback_rows(rows, media.kind, preview);
}

fn append_playback_rows(rows: &mut Vec<MetadataRow>, kind: PreviewKind, preview: &PreviewState) {
    if !matches!(kind, PreviewKind::Video | PreviewKind::Audio) {
        return;
    }

    rows.push(MetadataRow::field(
        "Playback",
        match preview.playback {
            PlaybackState::Stopped => "Stopped",
            PlaybackState::Starting => "Starting",
            PlaybackState::Playing => "Playing",
        },
    ));
    rows.push(MetadataRow::field(
        "Mute",
        if preview.muted { "Muted" } else { "Unmuted" },
    ));
    if kind == PreviewKind::Audio
        && let Some(position) = preview.playback_position
    {
        rows.push(MetadataRow::field("Position", format_duration(position)));
    }
}

fn append_document_rows(rows: &mut Vec<MetadataRow>, document: &DocumentPreview) {
    replace_file_type(rows, document.format.label());

    rows.push(MetadataRow::field(
        if document.format.is_book() {
            "Section"
        } else {
            "Page"
        },
        format!("{} / {}", document.current_page + 1, document.total_pages),
    ));
    if let (Some(width), Some(height)) = (document.page_width, document.page_height) {
        rows.push(MetadataRow::field(
            "Page size",
            format!("{width} × {height}"),
        ));
    }

    if let Some(title) = &document.title {
        rows.push(MetadataRow::field("Title", title));
    }
    if let Some(author) = &document.author {
        rows.push(MetadataRow::field("Author", author));
    }
}

fn replace_file_type(rows: &mut [MetadataRow], file_type: &str) {
    if let Some(MetadataRow::Field { value, .. }) = rows
        .iter_mut()
        .find(|row| matches!(row, MetadataRow::Field { label, .. } if label == "Type"))
    {
        *value = file_type.to_owned();
    }
}

fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.as_secs();
    let hours = total_seconds / 3600;
    let minutes = total_seconds % 3600 / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}
