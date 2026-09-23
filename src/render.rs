use clap::ValueEnum;
use std::{fmt::Write, sync::OnceLock};
use unicode_width::UnicodeWidthChar;

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Mode {
    Ascii,
    Blocks,
}

pub fn fit(width: usize, height: usize, columns: usize, rows: usize) -> (usize, usize) {
    let aspect = width as f64 / height.max(1) as f64 * 2.0;
    let w = columns.min((rows as f64 * aspect) as usize).max(1);
    let h = ((w as f64 / aspect).round() as usize).min(rows).max(1);
    (w, h)
}

pub fn lines(rgb: &[u8], width: usize, rows: usize, mode: Mode, color: bool) -> Vec<String> {
    static DECIMALS: OnceLock<[String; 256]> = OnceLock::new();
    static RAMP: OnceLock<[u8; 256]> = OnceLock::new();
    let decimals = DECIMALS.get_or_init(|| std::array::from_fn(|i| i.to_string()));
    let ramp = RAMP.get_or_init(|| {
        let chars = b" .,:;irsXA253hMHGS#9B&@";
        std::array::from_fn(|i| {
            chars[((i as f32 / 255.0).powf(0.85) * (chars.len() - 1) as f32) as usize]
        })
    });
    let gray =
        |p: &[u8]| ((77 * p[0] as u32 + 150 * p[1] as u32 + 29 * p[2] as u32 + 128) >> 8) as u8;
    let mut output = Vec::with_capacity(rows);
    for y in 0..rows {
        let mut line = String::with_capacity(width * if mode == Mode::Blocks { 40 } else { 20 });
        let mut previous = None;
        for x in 0..width {
            let offset = ((y * if mode == Mode::Blocks { 2 } else { 1 }) * width + x) * 3;
            let top = &rgb[offset..offset + 3];
            if mode == Mode::Ascii {
                let ch = ramp[gray(top) as usize] as char;
                let pair = [top[0], top[1], top[2], 0, 0, 0];
                if color && ch != ' ' && previous != Some(pair) {
                    line.push_str("\x1b[38;2;");
                    append_rgb(&mut line, top, decimals);
                    line.push('m');
                    previous = Some(pair);
                }
                line.push(ch);
            } else {
                let bottom = &rgb[offset + width * 3..offset + width * 3 + 3];
                let pair = if color {
                    [top[0], top[1], top[2], bottom[0], bottom[1], bottom[2]]
                } else {
                    let a = gray(top);
                    let b = gray(bottom);
                    [a, a, a, b, b, b]
                };
                if previous != Some(pair) {
                    line.push_str("\x1b[38;2;");
                    append_rgb(&mut line, &pair[..3], decimals);
                    line.push_str(";48;2;");
                    append_rgb(&mut line, &pair[3..], decimals);
                    line.push('m');
                    previous = Some(pair);
                }
                line.push('▀');
            }
        }
        if color || mode == Mode::Blocks {
            line.push_str("\x1b[0m");
        }
        output.push(line);
    }
    output
}

fn append_rgb(out: &mut String, pixel: &[u8], decimals: &[String; 256]) {
    out.push_str(&decimals[pixel[0] as usize]);
    out.push(';');
    out.push_str(&decimals[pixel[1] as usize]);
    out.push(';');
    out.push_str(&decimals[pixel[2] as usize]);
}

pub fn clip(text: &str, columns: usize) -> String {
    let mut remaining = columns;
    text.chars()
        .take_while(|c| {
            let width = c.width().unwrap_or(0);
            if width > remaining {
                return false;
            }
            remaining -= width;
            true
        })
        .collect()
}

#[derive(Default)]
pub struct Screen {
    previous: Vec<String>,
    size: (u16, u16),
}

impl Screen {
    pub fn update(&mut self, rows: Vec<String>, size: (u16, u16)) -> String {
        let reset = size != self.size;
        let mut out = String::new();
        if reset {
            self.previous.clear();
            out.push_str("\x1b[2J");
        }
        for (y, row) in rows.iter().enumerate() {
            if self.previous.get(y) != Some(row) {
                let _ = write!(out, "\x1b[{};1H{}\x1b[0m\x1b[K", y + 1, row);
            }
        }
        for y in rows.len()..self.previous.len() {
            let _ = write!(out, "\x1b[{};1H\x1b[0m\x1b[K", y + 1);
        }
        self.previous = rows;
        self.size = size;
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn black_and_white_ascii() {
        assert_eq!(lines(&[0; 6], 2, 1, Mode::Ascii, false), ["  "]);
        assert_eq!(lines(&[255; 6], 2, 1, Mode::Ascii, false), ["@@"]);
    }

    #[test]
    fn blocks_keep_both_rgb_pixels() {
        let result = lines(&[255, 0, 0, 0, 0, 255], 1, 1, Mode::Blocks, true);
        assert_eq!(result, ["\x1b[38;2;255;0;0;48;2;0;0;255m▀\x1b[0m"]);
    }

    #[test]
    fn full_width_keeps_aspect_ratio() {
        assert_eq!(fit(1920, 1080, 599, 173), (599, 168));
    }

    #[test]
    fn unchanged_screen_emits_nothing() {
        let mut screen = Screen::default();
        assert!(!screen.update(vec!["hello".into()], (80, 24)).is_empty());
        assert!(screen.update(vec!["hello".into()], (80, 24)).is_empty());
        assert!(screen.update(vec!["bye".into()], (80, 24)).contains("bye"));
    }

    #[test]
    fn resize_forces_redraw_and_wide_titles_fit() {
        let mut screen = Screen::default();
        screen.update(vec!["hello".into()], (80, 24));
        assert!(
            screen
                .update(vec!["hello".into()], (81, 24))
                .contains("\x1b[2J")
        );
        assert_eq!(clip("a界b", 3), "a界");
    }
}
