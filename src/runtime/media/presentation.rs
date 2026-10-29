use std::path::Path;

use fox_media_term::{MediaKind, PixelFormat, Protocol, Region};

use crate::app::Action;
use crate::preview::viewer::{MediaGeometry, calculate_layout};
use crate::preview::{ImageViewerLayout, RasterFrame};
use crate::terminal::TerminalSize;
use crate::theme::Theme;
use crate::workspace::PanelId;

use super::super::terminal_writer::TerminalWriter;
use super::{MediaRuntime, MediaSurface, media_metadata};

impl MediaRuntime {
    pub fn prepare_preview(
        &mut self,
        generation: u64,
        path: &Path,
        panel: PanelId,
        region: Region,
        writer: &mut TerminalWriter,
    ) -> Result<Action, String> {
        self.video = None;
        self.audio = None;

        let info = fox_media_term::probe_media(path).map_err(|error| error.to_string())?;
        if info.kind == MediaKind::Audio {
            self.surface = None;
            return Ok(Action::PreviewMediaPrepared {
                generation,
                path: path.to_path_buf(),
                metadata: media_metadata(info),
            });
        }

        let (prepared_kind, payload) = fox_media_term::prepare_preview(
            path,
            self.environment.protocol,
            self.environment.cell_pixel_width,
            self.environment.cell_pixel_height,
            region,
        )
        .map_err(|error| error.to_string())?;
        if prepared_kind != info.kind {
            return Err("Fox Media Term returned inconsistent preview media kinds".into());
        }

        writer.write_media(&payload)?;
        self.surface = Some(MediaSurface::Preview { panel, region });

        Ok(Action::PreviewMediaPrepared {
            generation,
            path: path.to_path_buf(),
            metadata: media_metadata(info),
        })
    }

    pub fn prepare_raster(
        &mut self,
        raster: &RasterFrame,
        panel: PanelId,
        region: Region,
        writer: &mut TerminalWriter,
    ) -> Result<(), String> {
        self.video = None;
        self.audio = None;

        let payload = fox_media_term::prepare_raster(
            &raster.pixels,
            raster.width,
            raster.height,
            raster.stride,
            PixelFormat::Rgba8,
            self.environment.protocol,
            self.environment.cell_pixel_width,
            self.environment.cell_pixel_height,
            region,
        )
        .map_err(|error| error.to_string())?;

        writer.write_media(&payload)?;
        self.surface = Some(MediaSurface::Preview { panel, region });

        Ok(())
    }

    pub fn maximum_pixel_size(&self, region: Region, maximum_pixels: u64) -> (u32, u32) {
        let pixel_graphics = matches!(
            self.environment.protocol,
            Protocol::Kitty | Protocol::ITerm2 | Protocol::Sixel
        );
        let mut width = if pixel_graphics {
            region.columns.max(1) as u32 * self.environment.cell_pixel_width.max(1) as u32
        } else {
            region.columns.max(1) as u32
        };
        let mut height = if pixel_graphics {
            region.rows.max(1) as u32 * self.environment.cell_pixel_height.max(1) as u32
        } else {
            region.rows.max(1) as u32 * 2
        };

        let pixels = u64::from(width) * u64::from(height);
        if pixels > maximum_pixels {
            let scale = (maximum_pixels as f64 / pixels as f64).sqrt();
            width = (width as f64 * scale).floor().max(1.0) as u32;
            height = (height as f64 * scale).floor().max(1.0) as u32;
        }

        (width, height)
    }

    pub fn probe_viewer(
        &self,
        generation: u64,
        path: &Path,
        title: &str,
        size: TerminalSize,
        maximum_percent: u16,
    ) -> Result<Action, String> {
        let info = fox_media_term::probe_media(path)
            .map_err(|error| format!("Cannot open the selected entry as an image: {error}"))?;
        if info.kind != MediaKind::Image {
            return Err("Videos cannot be opened in the modal image viewer".into());
        }

        let metadata = media_metadata(info);
        let layout = calculate_layout(
            size.area(),
            &metadata,
            title,
            maximum_percent,
            MediaGeometry {
                uses_pixel_graphics: matches!(
                    self.environment.protocol,
                    Protocol::Kitty | Protocol::ITerm2 | Protocol::Sixel
                ),
                cell_pixel_width: self.environment.cell_pixel_width.max(1) as u32,
                cell_pixel_height: self.environment.cell_pixel_height.max(1) as u32,
            },
        )?;

        Ok(Action::ViewerProbed {
            generation,
            path: path.to_path_buf(),
            metadata,
            layout,
        })
    }

    pub fn prepare_viewer(
        &mut self,
        generation: u64,
        path: &Path,
        layout: ImageViewerLayout,
        writer: &mut TerminalWriter,
    ) -> Result<Action, String> {
        self.video = None;
        self.audio = None;

        let (prepared_kind, payload) = fox_media_term::prepare_preview(
            path,
            self.environment.protocol,
            self.environment.cell_pixel_width,
            self.environment.cell_pixel_height,
            rect_region(layout.media),
        )
        .map_err(|error| error.to_string())?;
        if prepared_kind != MediaKind::Image {
            return Err("Fox Media Term returned non-image data for the modal viewer".into());
        }

        writer.write_media(&payload)?;
        self.surface = Some(MediaSurface::Viewer {
            region: rect_region(layout.media),
        });

        Ok(Action::ViewerPrepared {
            generation,
            path: path.to_path_buf(),
        })
    }

    pub fn clear_current(
        &mut self,
        theme: &Theme,
        writer: &mut TerminalWriter,
    ) -> Result<(), String> {
        self.video = None;
        self.audio = None;
        self.playback_context = None;
        self.last_audio_snapshot = None;

        let Some(surface) = self.surface else {
            return Ok(());
        };

        let (region, color) = match surface {
            MediaSurface::Preview { region, .. } => (region, theme.preview_background),
            MediaSurface::Viewer { region } => (region, theme.float_background),
        };
        let payload =
            fox_media_term::prepare_cleanup(self.environment.protocol, region, rgb_channels(color))
                .map_err(|error| error.to_string())?;

        writer.clear_media(&payload, region, color)?;
        self.surface = None;

        Ok(())
    }
}

fn rect_region(rect: ratatui::layout::Rect) -> Region {
    Region {
        column: i32::from(rect.x),
        row: i32::from(rect.y),
        columns: i32::from(rect.width),
        rows: i32::from(rect.height),
    }
}

fn rgb_channels(color: ratatui::style::Color) -> [u8; 3] {
    // TerminalWriter reapplies palette/default backgrounds after native RGB cleanup.
    match color {
        ratatui::style::Color::Rgb(red, green, blue) => [red, green, blue],
        _ => [0, 0, 0],
    }
}
