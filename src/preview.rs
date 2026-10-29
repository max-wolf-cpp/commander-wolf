use std::path::{Path, PathBuf};
use std::time::Duration;

pub mod archive;
pub mod directory;
pub mod document;
pub mod metadata;
pub mod text;
pub mod viewer;

mod summary;

pub use archive::{ArchiveFormat, ArchivePreview, ArchivePreviewEntry};
pub use directory::{DirectoryPreview, DirectoryPreviewEntry};
pub use document::{
    DocumentFormat, DocumentOptions, DocumentPageContent, DocumentPreview, DocumentRequest,
    RasterFrame,
};
pub(crate) use metadata::MetadataLayout;
pub use metadata::{FileMetadata, MetadataRow, MetadataViewportState};
pub use text::TextPreview;
pub use viewer::{ImageViewerLayout, ImageViewerState, ViewerStatus};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PreviewKind {
    #[default]
    None,
    Image,
    Video,
    Audio,
    Text,
    Directory,
    Archive,
    Document,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PlaybackState {
    #[default]
    Stopped,
    Starting,
    Playing,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum LoadState<T> {
    #[default]
    Idle,
    Loading,
    Ready(T),
    Unavailable(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MediaMetadata {
    pub kind: PreviewKind,
    pub pixel_width: u32,
    pub pixel_height: u32,
    pub duration: Option<Duration>,
    pub has_audio: Option<bool>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PreviewOptions {
    pub media_percent: u16,
    pub viewer_max_percent: u16,
    pub use_internal_image_viewer: bool,
    pub documents: DocumentOptions,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreviewState {
    pub generation: u64,
    pub path: Option<PathBuf>,
    pub title: String,
    pub metadata: LoadState<FileMetadata>,
    pub media: LoadState<MediaMetadata>,
    pub surface_suspended: bool,
    pub text: LoadState<TextPreview>,
    pub directory: LoadState<DirectoryPreview>,
    pub archive: LoadState<ArchivePreview>,
    pub document: LoadState<DocumentPreview>,
    pub document_request_id: u64,
    pub document_page: usize,
    pub metadata_viewport: MetadataViewportState,
    pub kind: PreviewKind,
    pub playback: PlaybackState,
    pub muted: bool,
    pub has_audio: Option<bool>,
    pub playback_position: Option<Duration>,
    pub audio_levels: Vec<u16>,
}

impl Default for PreviewOptions {
    fn default() -> Self {
        Self {
            media_percent: 70,
            viewer_max_percent: 80,
            use_internal_image_viewer: true,
            documents: DocumentOptions::default(),
        }
    }
}

impl Default for PreviewState {
    fn default() -> Self {
        Self {
            generation: 0,
            path: None,
            title: String::new(),
            metadata: LoadState::Idle,
            media: LoadState::Idle,
            surface_suspended: false,
            text: LoadState::Idle,
            directory: LoadState::Idle,
            archive: LoadState::Idle,
            document: LoadState::Idle,
            document_request_id: 0,
            document_page: 0,
            metadata_viewport: MetadataViewportState::default(),
            kind: PreviewKind::None,
            playback: PlaybackState::Stopped,
            muted: false,
            has_audio: None,
            playback_position: None,
            audio_levels: vec![0; 64],
        }
    }
}

impl PreviewState {
    pub fn begin(&mut self, path: PathBuf, title: String) -> u64 {
        self.generation = self.generation.wrapping_add(1).max(1);
        self.path = Some(path);
        self.title = title;

        self.metadata = LoadState::Loading;
        self.media = LoadState::Loading;
        self.surface_suspended = false;
        self.text = LoadState::Idle;
        self.directory = LoadState::Idle;
        self.archive = LoadState::Idle;
        self.document = LoadState::Idle;

        self.document_page = 0;
        self.metadata_viewport = MetadataViewportState::default();
        self.kind = PreviewKind::None;

        self.playback = PlaybackState::Stopped;
        self.has_audio = None;
        self.playback_position = None;
        self.audio_levels.fill(0);

        self.generation
    }

    pub fn clear(&mut self) {
        let muted = self.muted;
        let generation = self.generation.wrapping_add(1).max(1);

        *self = Self {
            generation,
            muted,
            ..Self::default()
        };
    }

    pub fn matches(&self, generation: u64, path: &Path) -> bool {
        self.generation == generation && self.path.as_deref() == Some(path)
    }

    pub fn media_owns_surface(&self) -> bool {
        !self.surface_suspended && self.surface_content_ready()
    }

    pub fn needs_surface_restore(&self) -> bool {
        self.surface_suspended && self.surface_content_ready()
    }

    fn surface_content_ready(&self) -> bool {
        matches!(
            self.media,
            LoadState::Ready(MediaMetadata {
                kind: PreviewKind::Image | PreviewKind::Video,
                ..
            })
        ) || matches!(
            self.document,
            LoadState::Ready(DocumentPreview {
                content: DocumentPageContent::Raster {
                    surface_ready: true,
                    ..
                },
                ..
            })
        )
    }

    pub fn metadata_rows(&self) -> Vec<MetadataRow> {
        summary::metadata_rows(self)
    }

    pub(crate) fn metadata_layout(&self, width: u16) -> MetadataLayout {
        MetadataLayout::new(self.metadata_rows(), width)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_selection_resets_scroll_and_invalidates_the_previous_generation() {
        let mut preview = PreviewState::default();
        preview.metadata_viewport.offset = 8;

        let first = preview.begin("first.png".into(), "first.png".into());
        let second = preview.begin("second.png".into(), "second.png".into());

        assert_ne!(first, second);
        assert_eq!(preview.metadata_viewport.offset, 0);
        assert!(preview.matches(second, Path::new("second.png")));
        assert!(!preview.matches(first, Path::new("first.png")));
    }

    #[test]
    fn media_surface_is_protected_only_after_a_payload_is_ready() {
        let mut preview = PreviewState {
            media: LoadState::Loading,
            ..PreviewState::default()
        };
        assert!(!preview.media_owns_surface());

        preview.media = LoadState::Ready(MediaMetadata {
            kind: PreviewKind::Image,
            pixel_width: 1,
            pixel_height: 1,
            duration: None,
            has_audio: None,
        });
        assert!(preview.media_owns_surface());

        let metadata = preview.media.clone();
        preview.surface_suspended = true;
        assert!(!preview.media_owns_surface());
        assert!(preview.needs_surface_restore());
        assert_eq!(preview.media, metadata);
        preview.surface_suspended = false;

        preview.media = LoadState::Ready(MediaMetadata {
            kind: PreviewKind::Audio,
            pixel_width: 0,
            pixel_height: 0,
            duration: Some(Duration::from_secs(3)),
            has_audio: Some(true),
        });
        assert!(!preview.media_owns_surface());
        preview.surface_suspended = true;
        assert!(!preview.needs_surface_restore());
    }

    #[test]
    fn audio_metadata_has_playback_without_resolution() {
        let mut preview = PreviewState {
            metadata: LoadState::Ready(FileMetadata {
                path: "track.flac".into(),
                file_type: "Regular file".into(),
                size: "1 B".into(),
                created: "Unavailable".into(),
                modified: "Unavailable".into(),
                permissions: "Read/write".into(),
                extension: Some("flac".into()),
                hard_links: None,
            }),
            media: LoadState::Ready(MediaMetadata {
                kind: PreviewKind::Audio,
                pixel_width: 0,
                pixel_height: 0,
                duration: Some(Duration::from_secs(65)),
                has_audio: Some(true),
            }),
            kind: PreviewKind::Audio,
            ..PreviewState::default()
        };
        preview.playback = PlaybackState::Playing;

        let rows = preview.metadata_rows();

        assert!(rows.iter().any(|row| matches!(
            row,
            MetadataRow::Field { label, value } if label == "Type" && value == "Audio"
        )));
        assert!(!rows.iter().any(|row| matches!(
            row,
            MetadataRow::Field { label, .. } if label == "Resolution"
        )));
        assert!(rows.iter().any(|row| matches!(
            row,
            MetadataRow::Field { label, value } if label == "Playback" && value == "Playing"
        )));
    }

    #[test]
    fn document_metadata_reports_page_dimensions_and_book_identity() {
        let preview = PreviewState {
            metadata: LoadState::Ready(FileMetadata {
                path: "book.epub".into(),
                file_type: "Regular file".into(),
                size: "1 KiB".into(),
                created: "Unavailable".into(),
                modified: "Unavailable".into(),
                permissions: "Read/write".into(),
                extension: Some("epub".into()),
                hard_links: None,
            }),
            document: LoadState::Ready(DocumentPreview {
                format: DocumentFormat::Epub,
                current_page: 2,
                total_pages: 8,
                page_width: Some(600),
                page_height: Some(800),
                title: Some("Book title".into()),
                author: Some("Book author".into()),
                content: DocumentPageContent::Text(Vec::new()),
            }),
            ..PreviewState::default()
        };
        let rows = preview.metadata_rows();

        for (label, value) in [
            ("Type", "EPUB"),
            ("Section", "3 / 8"),
            ("Page size", "600 × 800"),
            ("Title", "Book title"),
            ("Author", "Book author"),
        ] {
            assert!(rows.iter().any(|row| matches!(
                row,
                MetadataRow::Field { label: actual_label, value: actual_value }
                    if actual_label == label && actual_value == value
            )));
        }
    }
}
