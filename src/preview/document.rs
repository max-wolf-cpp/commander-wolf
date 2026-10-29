use std::path::{Path, PathBuf};
use std::sync::Arc;

mod book;
mod cbz;
mod djvu;
mod input;
mod pdf;
mod raster;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DocumentFormat {
    Pdf,
    DjVu,
    Cbz,
    Epub,
    Mobi,
    Azw3,
    Kfx,
    Fb2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DocumentOptions {
    pub enabled: bool,
    pub cache_pages: usize,
    pub max_render_pixels: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DocumentPageContent {
    Raster {
        width: u32,
        height: u32,
        surface_ready: bool,
    },
    Text(Vec<String>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentPreview {
    pub format: DocumentFormat,
    pub current_page: usize,
    pub total_pages: usize,
    pub page_width: Option<u32>,
    pub page_height: Option<u32>,
    pub title: Option<String>,
    pub author: Option<String>,
    pub content: DocumentPageContent,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RasterFrame {
    pub width: u32,
    pub height: u32,
    pub stride: usize,
    pub pixels: Arc<[u8]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentRequest {
    pub generation: u64,
    pub request_id: u64,
    pub path: PathBuf,
    pub format: DocumentFormat,
    pub page: usize,
    pub pixel_width: u32,
    pub pixel_height: u32,
    pub text_columns: u16,
    pub text_rows: u16,
    pub options: DocumentOptions,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentOutput {
    pub preview: DocumentPreview,
    pub raster: Option<RasterFrame>,
}

impl DocumentFormat {
    pub fn from_path(path: &Path) -> Option<Self> {
        let extension = path.extension()?.to_str()?.to_ascii_lowercase();

        match extension.as_str() {
            "pdf" => Some(Self::Pdf),
            "djvu" | "djv" => Some(Self::DjVu),
            "cbz" => Some(Self::Cbz),
            "epub" => Some(Self::Epub),
            "mobi" | "prc" => Some(Self::Mobi),
            "azw3" => Some(Self::Azw3),
            "kfx" => Some(Self::Kfx),
            "fb2" => Some(Self::Fb2),
            _ => None,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Pdf => "PDF",
            Self::DjVu => "DjVu",
            Self::Cbz => "CBZ",
            Self::Epub => "EPUB",
            Self::Mobi => "MOBI",
            Self::Azw3 => "AZW3",
            Self::Kfx => "KFX",
            Self::Fb2 => "FB2",
        }
    }

    pub const fn is_book(self) -> bool {
        matches!(
            self,
            Self::Epub | Self::Mobi | Self::Azw3 | Self::Kfx | Self::Fb2
        )
    }
}

impl Default for DocumentOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            cache_pages: 3,
            max_render_pixels: 8_000_000,
        }
    }
}

pub fn load(request: &DocumentRequest) -> Result<DocumentOutput, String> {
    input::ensure_size(&request.path)?;
    if request.options.max_render_pixels == 0 {
        return Err("Document render pixel limit must be positive".into());
    }

    match request.format {
        DocumentFormat::Pdf => pdf::load(request),
        DocumentFormat::DjVu => djvu::load(request),
        DocumentFormat::Cbz => cbz::load(request),
        format if format.is_book() => book::load(request),
        _ => Err("Unsupported document format".into()),
    }
}

fn non_empty(value: &str) -> Option<String> {
    (!value.trim().is_empty()).then(|| value.trim().to_owned())
}

#[cfg(test)]
mod tests;
