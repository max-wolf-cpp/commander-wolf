use std::path::{Path, PathBuf};

use ratatui::layout::Rect;
use ratatui::text::Line;

use super::MediaMetadata;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MediaGeometry {
    pub uses_pixel_graphics: bool,
    pub cell_pixel_width: u32,
    pub cell_pixel_height: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ImageViewerLayout {
    pub modal: Rect,
    pub media: Rect,
    pub pixel_width: u32,
    pub pixel_height: u32,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum ViewerStatus {
    #[default]
    Closed,
    Probing,
    Preparing,
    Ready,
    Unavailable(String),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ImageViewerState {
    pub generation: u64,
    pub path: Option<PathBuf>,
    pub title: String,
    pub status: ViewerStatus,
    pub layout: Option<ImageViewerLayout>,
}

impl ImageViewerState {
    pub fn open(&mut self, path: PathBuf, title: String) -> u64 {
        self.generation = self.generation.wrapping_add(1).max(1);
        self.path = Some(path);
        self.title = title;
        self.status = ViewerStatus::Probing;
        self.layout = None;

        self.generation
    }

    pub fn close(&mut self) {
        let generation = self.generation.wrapping_add(1).max(1);

        *self = Self {
            generation,
            ..Self::default()
        };
    }

    pub fn is_open(&self) -> bool {
        self.status != ViewerStatus::Closed
    }

    pub fn matches(&self, generation: u64, path: &Path) -> bool {
        self.generation == generation && self.path.as_deref() == Some(path)
    }
}

pub fn calculate_layout(
    terminal: Rect,
    media: &MediaMetadata,
    title: &str,
    maximum_percent: u16,
    geometry: MediaGeometry,
) -> Result<ImageViewerLayout, String> {
    if media.pixel_width == 0
        || media.pixel_height == 0
        || terminal.width < 3
        || terminal.height < 3
    {
        return Err("The terminal or image is too small for the image viewer".into());
    }

    let maximum_columns = percentage(terminal.width, maximum_percent)
        .max(3)
        .min(terminal.width);
    let maximum_rows = percentage(terminal.height, maximum_percent)
        .max(3)
        .min(terminal.height);
    let content_columns = maximum_columns.saturating_sub(2).max(1);
    let content_rows = maximum_rows.saturating_sub(2).max(1);
    let (pixel_capacity_width, pixel_capacity_height) =
        geometry.pixel_capacity(content_columns, content_rows);

    let (pixel_width, pixel_height) = fit_without_upscale(
        media.pixel_width,
        media.pixel_height,
        pixel_capacity_width,
        pixel_capacity_height,
    );

    let (image_columns, image_rows) = geometry.cell_size(pixel_width, pixel_height);
    let modal = centered_modal(
        terminal,
        (maximum_columns, maximum_rows),
        (image_columns, image_rows),
        title,
    );
    let media = centered_media(modal, image_columns, image_rows);

    Ok(ImageViewerLayout {
        modal,
        media,
        pixel_width,
        pixel_height,
    })
}

impl MediaGeometry {
    fn pixel_capacity(self, columns: u16, rows: u16) -> (u32, u32) {
        let width = if self.uses_pixel_graphics {
            u32::from(columns).saturating_mul(self.cell_pixel_width.max(1))
        } else {
            u32::from(columns)
        };
        let height = if self.uses_pixel_graphics {
            u32::from(rows).saturating_mul(self.cell_pixel_height.max(1))
        } else {
            u32::from(rows).saturating_mul(2)
        };

        (width, height)
    }

    fn cell_size(self, pixel_width: u32, pixel_height: u32) -> (u32, u32) {
        let image_columns = if self.uses_pixel_graphics {
            divide_ceil(pixel_width, self.cell_pixel_width.max(1))
        } else {
            pixel_width
        };
        let image_rows = if self.uses_pixel_graphics {
            divide_ceil(pixel_height, self.cell_pixel_height.max(1))
        } else {
            divide_ceil(pixel_height, 2)
        };

        (image_columns, image_rows)
    }
}

fn centered_modal(
    terminal: Rect,
    (maximum_columns, maximum_rows): (u16, u16),
    (image_columns, image_rows): (u32, u32),
    title: &str,
) -> Rect {
    let title_columns = Line::from(title).width().min(usize::from(u16::MAX)) as u16;
    let modal_width = maximum_columns.min(
        u16::try_from(image_columns)
            .unwrap_or(u16::MAX)
            .saturating_add(2)
            .max(title_columns.saturating_add(7)),
    );
    let modal_height = maximum_rows.min(
        u16::try_from(image_rows)
            .unwrap_or(u16::MAX)
            .saturating_add(2),
    );

    Rect::new(
        terminal.x + terminal.width.saturating_sub(modal_width) / 2,
        terminal.y + terminal.height.saturating_sub(modal_height) / 2,
        modal_width,
        modal_height,
    )
}

fn centered_media(modal: Rect, image_columns: u32, image_rows: u32) -> Rect {
    let inner = Rect::new(
        modal.x.saturating_add(1),
        modal.y.saturating_add(1),
        modal.width.saturating_sub(2),
        modal.height.saturating_sub(2),
    );
    let image_columns = u16::try_from(image_columns)
        .unwrap_or(u16::MAX)
        .min(inner.width);
    let image_rows = u16::try_from(image_rows)
        .unwrap_or(u16::MAX)
        .min(inner.height);

    Rect::new(
        inner.x + inner.width.saturating_sub(image_columns) / 2,
        inner.y + inner.height.saturating_sub(image_rows) / 2,
        image_columns,
        image_rows,
    )
}

fn percentage(value: u16, percent: u16) -> u16 {
    (u32::from(value) * u32::from(percent) / 100)
        .try_into()
        .unwrap_or(u16::MAX)
}

fn divide_ceil(value: u32, divisor: u32) -> u32 {
    value / divisor + u32::from(!value.is_multiple_of(divisor))
}

fn fit_without_upscale(
    width: u32,
    height: u32,
    maximum_width: u32,
    maximum_height: u32,
) -> (u32, u32) {
    if width <= maximum_width && height <= maximum_height {
        return (width, height);
    }

    if u64::from(maximum_width) * u64::from(height) <= u64::from(maximum_height) * u64::from(width)
    {
        (
            maximum_width.max(1),
            (u64::from(height) * u64::from(maximum_width) / u64::from(width)).max(1) as u32,
        )
    } else {
        (
            (u64::from(width) * u64::from(maximum_height) / u64::from(height)).max(1) as u32,
            maximum_height.max(1),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preview::PreviewKind;

    fn image(width: u32, height: u32) -> MediaMetadata {
        MediaMetadata {
            kind: PreviewKind::Image,
            pixel_width: width,
            pixel_height: height,
            duration: None,
            has_audio: None,
        }
    }

    #[test]
    fn pixel_layout_does_not_upscale_small_images() {
        let layout = calculate_layout(
            Rect::new(0, 0, 120, 40),
            &image(80, 40),
            "画像.png",
            80,
            MediaGeometry {
                uses_pixel_graphics: true,
                cell_pixel_width: 10,
                cell_pixel_height: 20,
            },
        )
        .unwrap();

        assert_eq!((layout.pixel_width, layout.pixel_height), (80, 40));
        assert_eq!((layout.media.width, layout.media.height), (8, 2));
    }

    #[test]
    fn text_layout_uses_half_block_geometry_and_bounds_large_images() {
        let layout = calculate_layout(
            Rect::new(0, 0, 100, 50),
            &image(400, 400),
            "large.png",
            80,
            MediaGeometry {
                uses_pixel_graphics: false,
                cell_pixel_width: 1,
                cell_pixel_height: 1,
            },
        )
        .unwrap();

        assert!(layout.modal.width <= 80);
        assert!(layout.modal.height <= 40);
        assert_eq!(layout.pixel_width, u32::from(layout.media.width));
        assert!(layout.pixel_height <= u32::from(layout.media.height) * 2);
    }

    #[test]
    fn tiny_terminals_fail_safely() {
        assert!(
            calculate_layout(
                Rect::new(0, 0, 2, 2),
                &image(10, 10),
                "x",
                80,
                MediaGeometry {
                    uses_pixel_graphics: true,
                    cell_pixel_width: 8,
                    cell_pixel_height: 16,
                },
            )
            .is_err()
        );
    }
}
