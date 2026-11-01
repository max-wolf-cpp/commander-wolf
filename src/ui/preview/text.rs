use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::syntax::{SyntaxCategory, SyntaxTheme};
use crate::theme::Theme;

pub(super) fn draw(
    frame: &mut Frame<'_>,
    area: Rect,
    preview: &crate::preview::TextPreview,
    theme: &Theme,
    syntax_theme: &SyntaxTheme,
    syntax_enabled: bool,
    background: Style,
) {
    let mut lines = preview
        .lines
        .iter()
        .take(usize::from(area.height))
        .enumerate()
        .map(|(index, line)| {
            if !syntax_enabled {
                Line::styled(line.as_str(), Style::default().fg(theme.text))
            } else {
                syntax_line(
                    line,
                    preview
                        .highlighting
                        .as_ref()
                        .ok()
                        .and_then(|lines| lines.get(index))
                        .map(Vec::as_slice)
                        .unwrap_or(&[]),
                    syntax_theme,
                    theme.text,
                )
            }
        })
        .collect::<Vec<_>>();
    if preview.lines.is_empty() {
        lines.push(Line::styled(
            "Empty file",
            Style::default().fg(theme.preview_unavailable),
        ));
    } else if preview.truncated && lines.len() < usize::from(area.height) {
        lines.push(Line::styled(
            "… preview truncated …",
            Style::default().fg(theme.preview_unavailable),
        ));
    }

    if syntax_enabled
        && let Err(message) = &preview.highlighting
        && lines.len() < usize::from(area.height)
    {
        lines.push(Line::styled(
            format!("Syntax highlighting unavailable: {message}"),
            Style::default().fg(theme.preview_unavailable),
        ));
    }

    frame.render_widget(Paragraph::new(lines).style(background.fg(theme.text)), area);
}

fn syntax_line<'a>(
    text: &'a str,
    tokens: &[crate::syntax::SyntaxSpan],
    palette: &SyntaxTheme,
    fallback: Color,
) -> Line<'a> {
    let plain = Style::default().fg(palette.color(SyntaxCategory::Text, fallback));
    let mut spans = Vec::new();
    let mut cursor = 0;
    for token in tokens {
        if token.range.start < cursor || token.range.end < token.range.start {
            return Line::styled(text, plain);
        }

        let Some(gap) = text.get(cursor..token.range.start) else {
            return Line::styled(text, plain);
        };

        let Some(value) = text.get(token.range.clone()) else {
            return Line::styled(text, plain);
        };

        if !gap.is_empty() {
            spans.push(Span::styled(gap, plain));
        }
        spans.push(Span::styled(
            value,
            token_style(token.category, palette, fallback),
        ));
        cursor = token.range.end;
    }
    if cursor < text.len() {
        spans.push(Span::styled(&text[cursor..], plain));
    }

    Line::from(spans)
}

fn token_style(category: SyntaxCategory, palette: &SyntaxTheme, fallback: Color) -> Style {
    let mut style = Style::default().fg(palette.color(category, fallback));
    match category {
        SyntaxCategory::Comment | SyntaxCategory::Emphasis => {
            style = style.add_modifier(Modifier::ITALIC)
        }
        SyntaxCategory::Strong | SyntaxCategory::Heading => {
            style = style.add_modifier(Modifier::BOLD)
        }
        SyntaxCategory::Link => style = style.add_modifier(Modifier::UNDERLINED),
        _ => {}
    }

    style
}

#[cfg(test)]
mod tests {
    #[test]
    fn invalid_backend_ranges_fall_back_without_losing_unicode_text() {
        use crate::syntax::{SyntaxCategory, SyntaxSpan, SyntaxTheme};
        let palette = SyntaxTheme::default();
        let text = "狼 = 42";
        let line = super::syntax_line(
            text,
            &[SyntaxSpan {
                range: 1..3,
                category: SyntaxCategory::String,
            }],
            &palette,
            crate::theme::Theme::emergency_monochrome().text,
        );
        assert_eq!(
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>(),
            text
        );
    }
}
