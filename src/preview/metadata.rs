use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MetadataRow {
    Field { label: String, value: String },
    Separator,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileMetadata {
    pub path: String,
    pub file_type: String,
    pub size: String,
    pub created: String,
    pub modified: String,
    pub permissions: String,
    pub extension: Option<String>,
    pub hard_links: Option<u64>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MetadataViewportState {
    pub offset: usize,
    pub visible_rows: usize,
}

pub(crate) struct MetadataLayout {
    pub label_width: u16,
    pub rows: Vec<MetadataRow>,
}

impl MetadataRow {
    pub fn field(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self::Field {
            label: label.into(),
            value: value.into(),
        }
    }
}

impl FileMetadata {
    pub fn rows(&self) -> Vec<MetadataRow> {
        let mut rows = vec![
            MetadataRow::field("Path", &self.path),
            MetadataRow::field("Type", &self.file_type),
            MetadataRow::field("Size", &self.size),
            MetadataRow::field("Created", &self.created),
            MetadataRow::field("Modified", &self.modified),
            MetadataRow::field("Permissions", &self.permissions),
            MetadataRow::Separator,
        ];

        if let Some(extension) = &self.extension {
            rows.push(MetadataRow::field("Extension", extension));
        }
        if let Some(hard_links) = self.hard_links {
            rows.push(MetadataRow::field("Hard links", hard_links.to_string()));
        }

        rows
    }
}

impl MetadataLayout {
    pub(super) fn new(rows: Vec<MetadataRow>, width: u16) -> Self {
        // Leave room for a wide grapheme even in a narrow value column.
        let label_width = 13.min(width.saturating_sub(2));
        let value_width = usize::from(width.saturating_sub(label_width));
        let mut wrapped = Vec::new();
        for row in rows {
            match row {
                MetadataRow::Field { label, value } if label == "Path" && value_width > 0 => {
                    for (index, value) in wrap_path(&value, value_width).into_iter().enumerate() {
                        wrapped.push(MetadataRow::field(
                            if index == 0 { "Path" } else { "" },
                            value,
                        ));
                    }
                }
                row => wrapped.push(row),
            }
        }

        Self {
            label_width,
            rows: wrapped,
        }
    }
}

fn wrap_path(path: &str, width: usize) -> Vec<String> {
    if path.width() <= width {
        return vec![path.to_owned()];
    }

    let mut lines = Vec::new();
    let mut line = String::new();
    let mut line_width = 0;
    for grapheme in path.graphemes(true) {
        let grapheme_width = grapheme.width();
        if !line.is_empty() && line_width + grapheme_width > width {
            lines.push(std::mem::take(&mut line));
            line_width = 0;
        }
        line.push_str(grapheme);
        line_width += grapheme_width;
    }
    lines.push(line);

    lines
}

impl MetadataViewportState {
    pub fn set_visible_rows(&mut self, visible_rows: usize, content_rows: usize) {
        self.visible_rows = visible_rows;
        self.clamp(content_rows);
    }

    pub fn page_down(&mut self, content_rows: usize) {
        let step = (self.visible_rows / 2).max(1);
        self.offset = self.offset.saturating_add(step);
        self.clamp(content_rows);
    }

    pub fn page_up(&mut self) {
        let step = (self.visible_rows / 2).max(1);
        self.offset = self.offset.saturating_sub(step);
    }

    fn clamp(&mut self, content_rows: usize) {
        self.offset = self
            .offset
            .min(content_rows.saturating_sub(self.visible_rows));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrapped_paths_preserve_spaces_and_whole_graphemes() {
        let path = "C:\\two  spaces\\e\u{301}\\中文\\👩‍💻\\file name.txt";
        for width in [0, 1, 2, 4, 14, 15, 16, 30, 100] {
            let layout = MetadataLayout::new(vec![MetadataRow::field("Path", path)], width);
            let mut restored = String::new();
            for (index, row) in layout.rows.iter().enumerate() {
                let MetadataRow::Field { label, value } = row else {
                    panic!("expected a path field")
                };
                assert_eq!(label, if index == 0 { "Path" } else { "" });
                if width >= 2 {
                    assert!(value.width() <= usize::from(width - layout.label_width));
                }
                for grapheme in value.graphemes(true) {
                    assert!(path.graphemes(true).any(|original| original == grapheme));
                }
                restored.push_str(value);
            }
            assert_eq!(restored, path);
        }
    }

    #[test]
    fn paths_that_fit_keep_the_following_fields_on_their_original_rows() {
        for path in ["", "a.png", "1234567890"] {
            let rows = vec![
                MetadataRow::field("Path", path),
                MetadataRow::field("Type", "File"),
                MetadataRow::Separator,
            ];
            let layout = MetadataLayout::new(rows.clone(), 13 + path.width() as u16);
            assert_eq!(layout.rows, rows);
        }
    }

    #[test]
    fn metadata_pages_by_half_a_view_and_clamps() {
        let mut viewport = MetadataViewportState {
            offset: 0,
            visible_rows: 6,
        };

        viewport.page_down(10);
        assert_eq!(viewport.offset, 3);

        viewport.page_down(10);
        assert_eq!(viewport.offset, 4);

        viewport.page_up();
        assert_eq!(viewport.offset, 1);

        viewport.set_visible_rows(20, 10);
        assert_eq!(viewport.offset, 0);
    }
}
