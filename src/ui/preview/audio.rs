use ratatui::Frame;
use ratatui::layout::{Margin, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::Block;
use ratatui::widgets::canvas::{Canvas, Context, Line as CanvasLine};

use crate::theme::Theme;

pub(super) fn draw(
    frame: &mut Frame<'_>,
    area: Rect,
    levels: &[u16],
    theme: &Theme,
    background: Style,
) {
    if area.is_empty() {
        return;
    }

    frame.render_widget(Block::default().style(background), area);

    let padded = area.inner(Margin::new((area.width / 10).min(4), area.height / 6));
    let width = padded.width.min(96);
    let height = padded.height.min(15);
    let waveform_area = Rect::new(
        padded.x + (padded.width - width) / 2,
        padded.y + (padded.height - height) / 2,
        width,
        height,
    );
    let envelope = audio_envelope(levels, usize::from(width) * 2);
    let canvas_background = background.bg.unwrap_or(theme.preview_background.into());

    frame.render_widget(
        Canvas::default()
            .background_color(canvas_background)
            .x_bounds([0.0, (envelope.len() - 1) as f64])
            .y_bounds([-1.0, 1.0])
            .paint(|context| paint_waveform(context, &envelope, canvas_background, theme)),
        waveform_area,
    );
}

fn paint_waveform(context: &mut Context<'_>, envelope: &[f64], base: Color, theme: &Theme) {
    // Draw the quiet inner texture first so the outer contour stays crisp.
    for (index, &amplitude) in envelope.iter().enumerate().step_by(4) {
        context.draw(&CanvasLine {
            x1: index as f64,
            y1: -amplitude,
            x2: index as f64,
            y2: amplitude,
            color: if matches!(base, Color::Rgb(..)) {
                blend_color(base, theme.preview_visualizer_low, 0.22)
            } else {
                theme.preview_visualizer_low
            },
        });
    }

    for (index, pair) in envelope.windows(2).enumerate() {
        let position = index as f64 / (envelope.len() - 1) as f64;
        let accent = blend_color(
            theme.preview_visualizer_low,
            theme.preview_visualizer_high,
            position,
        );
        let strength = 0.45 + 0.55 * pair[0].sqrt();
        for (scale, opacity) in [(0.45, 0.3), (1.0, strength)] {
            let color = if matches!(base, Color::Rgb(..)) {
                blend_color(base, accent, opacity)
            } else {
                accent
            };
            for direction in [-1.0, 1.0] {
                context.draw(&CanvasLine {
                    x1: index as f64,
                    y1: pair[0] * scale * direction,
                    x2: (index + 1) as f64,
                    y2: pair[1] * scale * direction,
                    color,
                });
            }
        }
    }
}

fn audio_envelope(levels: &[u16], samples: usize) -> Vec<f64> {
    if levels.is_empty() || samples < 2 {
        return vec![0.0; samples];
    }

    // PCM buckets describe an amplitude envelope, not frequency bands. Smooth adjacent
    // buckets before interpolation to avoid a jagged contour at wide terminal sizes.
    let smoothed: Vec<f64> = (0..levels.len())
        .map(|index| {
            [1.0, 2.0, 3.0, 2.0, 1.0]
                .iter()
                .enumerate()
                .map(|(offset, weight)| {
                    let sample = (index + offset).saturating_sub(2).min(levels.len() - 1);
                    f64::from(levels[sample].min(1_000)) * weight / 9_000.0
                })
                .sum()
        })
        .collect();

    (0..samples)
        .map(|index| {
            let position = index as f64 / (samples - 1) as f64;
            let source = position * (smoothed.len() - 1) as f64;
            let left = source as usize;
            let right = (left + 1).min(smoothed.len() - 1);
            let fraction = source.fract();
            let weight = fraction * fraction * (3.0 - 2.0 * fraction);
            let amplitude = smoothed[left] * (1.0 - weight) + smoothed[right] * weight;
            let edge = (position.min(1.0 - position) / 0.15).min(1.0);
            let taper = edge * edge * (3.0 - 2.0 * edge);
            amplitude.sqrt() * taper * 0.85
        })
        .collect()
}

fn blend_color(from: Color, to: Color, weight: f64) -> Color {
    let (Color::Rgb(from_red, from_green, from_blue), Color::Rgb(to_red, to_green, to_blue)) =
        (from, to)
    else {
        return if weight < 0.5 { from } else { to };
    };

    let channel = |from: u8, to: u8| {
        (f64::from(from) * (1.0 - weight) + f64::from(to) * weight).round() as u8
    };

    Color::Rgb(
        channel(from_red, to_red),
        channel(from_green, to_green),
        channel(from_blue, to_blue),
    )
}

#[cfg(test)]
mod tests {
    use super::audio_envelope;

    #[test]
    fn audio_envelope_is_silent_without_audio_and_bounded_for_loud_input() {
        assert!(audio_envelope(&[1_000], 0).is_empty());
        assert_eq!(audio_envelope(&[1_000], 1), vec![0.0]);
        assert_eq!(audio_envelope(&[], 16), vec![0.0; 16]);
        assert_eq!(audio_envelope(&[0; 64], 16), vec![0.0; 16]);
        let loud = audio_envelope(&[u16::MAX; 64], 128);
        assert_eq!(loud.first(), Some(&0.0));
        assert_eq!(loud.last(), Some(&0.0));
        assert!(loud.iter().all(|value| (0.0..=0.85).contains(value)));
        assert!(loud[64] > 0.8);
    }

    #[test]
    fn audio_envelope_preserves_dynamics_and_softens_isolated_peaks() {
        let quiet = audio_envelope(&[100; 64], 128);
        let loud = audio_envelope(&[900; 64], 128);
        assert!(quiet[64] > 0.0 && quiet[64] < loud[64]);
        let mut impulse = [0; 9];
        impulse[4] = 1_000;
        let envelope = audio_envelope(&impulse, 65);
        assert!(envelope[24] > 0.0);
        assert!(envelope[32] > envelope[24]);
        assert!(envelope[32] < 0.85);
        for (left, right) in envelope.iter().zip(envelope.iter().rev()) {
            assert!((left - right).abs() < 1e-10);
        }
    }

    #[test]
    fn audio_visualizer_stays_inside_its_viewport_and_preserves_background() {
        use ratatui::{Terminal, backend::TestBackend, layout::Rect, style::Style};
        let theme = crate::theme::Theme::emergency_monochrome();
        let background = ratatui::style::Color::Rgb(30, 30, 46);
        for (width, height) in [(0, 0), (1, 1), (1, 2), (2, 1), (8, 4), (60, 20), (120, 40)] {
            let mut terminal = Terminal::new(TestBackend::new(width + 4, height + 4)).unwrap();
            let area = Rect::new(2, 2, width, height);
            terminal
                .draw(|frame| {
                    super::draw(
                        frame,
                        area,
                        &[0, 250, 500, 1_000],
                        &theme,
                        Style::default().bg(background),
                    );
                })
                .unwrap();
            let buffer = terminal.backend().buffer();
            for y in 0..height + 4 {
                for x in 0..width + 4 {
                    let cell = &buffer[(x, y)];
                    if area.contains((x, y).into()) {
                        assert_eq!(cell.bg, background);
                    } else {
                        assert_eq!(cell.symbol(), " ");
                        assert_eq!(cell.bg, ratatui::style::Color::Reset);
                    }
                }
            }
            if width > 2 && height > 2 {
                assert!(
                    buffer
                        .content()
                        .iter()
                        .any(|cell| cell.symbol() != " " && cell.symbol() != "⠀")
                );
            }
        }
    }
}
