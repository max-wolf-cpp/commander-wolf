use crate::theme::Rgb;

pub(crate) fn nearest_color(target: Rgb, colors: &[Rgb; 16], background: Rgb) -> usize {
    let target = oklab(target);
    let mut best_visible = None;
    let mut best_fallback = (0, 0.0, f64::INFINITY);

    for (index, color) in colors.iter().copied().enumerate() {
        let lab = oklab(color);
        let distance = lab
            .iter()
            .zip(target)
            .map(|(actual, target)| (actual - target).powi(2))
            .sum::<f64>();
        let contrast = contrast(color, background);

        if contrast >= 3.0 && best_visible.is_none_or(|(_, best_distance)| distance < best_distance)
        {
            best_visible = Some((index, distance));
        }

        if contrast > best_fallback.1 || (contrast == best_fallback.1 && distance < best_fallback.2)
        {
            best_fallback = (index, contrast, distance);
        }
    }

    best_visible.map_or(best_fallback.0, |(index, _)| index)
}

pub(crate) fn readable_palette_text(background: Rgb, colors: &[Rgb; 16]) -> usize {
    const NEUTRALS: [usize; 4] = [0, 7, 8, 15];
    let most_contrasting = |indices: &[usize]| {
        indices
            .iter()
            .copied()
            .max_by(|&left, &right| {
                contrast(colors[left], background).total_cmp(&contrast(colors[right], background))
            })
            .expect("candidate palette indices are nonempty")
    };

    let neutral = most_contrasting(&NEUTRALS);
    if contrast(colors[neutral], background) >= 4.5 {
        neutral
    } else {
        let all: [usize; 16] = std::array::from_fn(|index| index);
        most_contrasting(&all)
    }
}

pub(crate) fn is_light_background(color: Rgb) -> bool {
    luminance(color) >= 0.179
}

pub(crate) fn contrast(first: Rgb, second: Rgb) -> f64 {
    let first = luminance(first);
    let second = luminance(second);

    (first.max(second) + 0.05) / (first.min(second) + 0.05)
}

fn luminance(color: Rgb) -> f64 {
    0.2126 * linear(color.red) + 0.7152 * linear(color.green) + 0.0722 * linear(color.blue)
}

fn oklab(color: Rgb) -> [f64; 3] {
    // Coefficients from the Oklab reference conversion by Björn Ottosson.
    let red = linear(color.red);
    let green = linear(color.green);
    let blue = linear(color.blue);

    let l = (0.4122214708 * red + 0.5363325363 * green + 0.0514459929 * blue).cbrt();
    let m = (0.2119034982 * red + 0.6806995451 * green + 0.1073969566 * blue).cbrt();
    let s = (0.0883024619 * red + 0.2817188376 * green + 0.6299787005 * blue).cbrt();

    [
        0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s,
        1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s,
        0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s,
    ]
}

fn linear(channel: u8) -> f64 {
    let value = f64::from(channel) / 255.0;

    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest_color_prefers_similarity_with_readable_contrast() {
        let black = Rgb::parse("#000000").unwrap();
        let white = Rgb::parse("#FFFFFF").unwrap();
        let blue = Rgb::parse("#000080").unwrap();
        let cyan = Rgb::parse("#00FFFF").unwrap();
        let mut colors = [white; 16];
        colors[4] = blue;
        colors[14] = cyan;
        assert_eq!(nearest_color(cyan, &colors, black), 14);
        assert_eq!(nearest_color(blue, &colors, black), 14);
        assert_eq!(nearest_color(blue, &colors, white), 4);
    }

    #[test]
    fn selected_text_prefers_readable_neutrals_then_other_palette_colors() {
        let black = Rgb::parse("#000000").unwrap();
        let white = Rgb::parse("#FFFFFF").unwrap();
        let dark_blue = Rgb::parse("#000080").unwrap();
        let light_blue = Rgb::parse("#BBDDFF").unwrap();
        let mut colors = [dark_blue; 16];
        colors[0] = black;
        colors[15] = white;
        assert_eq!(readable_palette_text(dark_blue, &colors), 15);
        assert_eq!(readable_palette_text(light_blue, &colors), 0);

        colors[0] = dark_blue;
        colors[15] = dark_blue;
        colors[6] = white;
        assert_eq!(readable_palette_text(dark_blue, &colors), 6);
    }
}
