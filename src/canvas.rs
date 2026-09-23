//! Exact-color cell updates. The previous screen is compared before ANSI encoding.
use crate::render::Mode;
use std::{fmt::Write, sync::OnceLock};

const DEFAULT: u32 = u32::MAX;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
struct Cell {
    foreground: u32,
    background: u32,
    glyph: char,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Layout {
    size: (usize, usize),
    origin: (usize, usize),
    mode: Mode,
    color: bool,
}

#[derive(Default)]
pub struct Canvas {
    previous: Vec<Cell>,
    current: Vec<Cell>,
    layout: Option<Layout>,
    output: String,
}

impl Canvas {
    pub fn invalidate(&mut self) {
        self.layout = None;
    }

    pub fn update(
        &mut self,
        rgb: &[u8],
        size: (usize, usize),
        origin: (usize, usize),
        mode: Mode,
        color: bool,
    ) -> &str {
        static RAMP: OnceLock<[char; 256]> = OnceLock::new();
        let ramp = RAMP.get_or_init(|| {
            let chars = b" .,:;irsXA253hMHGS#9B&@";
            std::array::from_fn(|i| {
                chars[((i as f32 / 255.0).powf(0.85) * (chars.len() - 1) as f32) as usize] as char
            })
        });
        let layout = Layout {
            size,
            origin,
            mode,
            color,
        };
        let full = self.layout != Some(layout);
        let (width, rows) = size;
        self.current.resize(width * rows, Cell::default());
        let gray =
            |p: &[u8]| ((77 * p[0] as u32 + 150 * p[1] as u32 + 29 * p[2] as u32 + 128) >> 8) as u8;
        let pack = |p: &[u8]| {
            if color {
                (p[0] as u32) << 16 | (p[1] as u32) << 8 | p[2] as u32
            } else {
                gray(p) as u32 * 0x010101
            }
        };
        for y in 0..rows {
            for x in 0..width {
                let offset = (y * if mode == Mode::Blocks { 2 } else { 1 } * width + x) * 3;
                let top = &rgb[offset..offset + 3];
                let cell = if mode == Mode::Ascii {
                    let glyph = ramp[gray(top) as usize];
                    Cell {
                        glyph,
                        foreground: if color && glyph != ' ' {
                            pack(top)
                        } else {
                            DEFAULT
                        },
                        background: DEFAULT,
                    }
                } else {
                    let upper = pack(top);
                    let lower = pack(&rgb[offset + width * 3..offset + width * 3 + 3]);
                    if upper == lower {
                        Cell {
                            glyph: ' ',
                            foreground: DEFAULT,
                            background: lower,
                        }
                    } else {
                        Cell {
                            glyph: '▀',
                            foreground: upper,
                            background: lower,
                        }
                    }
                };
                self.current[y * width + x] = cell;
            }
        }
        self.output.clear();
        let mut foreground = DEFAULT;
        let mut background = DEFAULT;
        for y in 0..rows {
            let offset = y * width;
            let dirty = |x: usize| full || self.previous[offset + x] != self.current[offset + x];
            let mut x = 0;
            while x < width {
                while x < width && !dirty(x) {
                    x += 1;
                }
                if x == width {
                    break;
                }
                let start = x;
                let mut end = x + 1;
                x += 1;
                // Short unchanged gaps cost less to repaint than another cursor command.
                while x < width {
                    if dirty(x) {
                        end = x + 1;
                    } else if x - end >= 3 {
                        break;
                    }
                    x += 1;
                }
                if self.output.is_empty() {
                    self.output.push_str("\x1b[0m");
                }
                let _ = write!(
                    self.output,
                    "\x1b[{};{}H",
                    origin.1 + y + 1,
                    origin.0 + start + 1
                );
                for index in start..end {
                    emit(
                        &mut self.output,
                        self.current[offset + index],
                        &mut foreground,
                        &mut background,
                    );
                }
            }
        }
        if !self.output.is_empty() {
            self.output.push_str("\x1b[0m");
        }
        std::mem::swap(&mut self.previous, &mut self.current);
        self.layout = Some(layout);
        &self.output
    }
}

fn emit(out: &mut String, mut cell: Cell, foreground: &mut u32, background: &mut u32) {
    if cell.glyph == '▀' {
        let normal = usize::from(cell.foreground != *foreground)
            + usize::from(cell.background != *background);
        let inverted = usize::from(cell.background != *foreground)
            + usize::from(cell.foreground != *background);
        if inverted < normal {
            std::mem::swap(&mut cell.foreground, &mut cell.background);
            cell.glyph = '▄';
        }
    }
    let fg = cell.foreground != DEFAULT && cell.foreground != *foreground;
    let bg = cell.background != DEFAULT && cell.background != *background;
    if fg || bg {
        out.push_str("\x1b[");
        if fg {
            out.push_str("38;2;");
            rgb_text(out, cell.foreground);
            *foreground = cell.foreground;
        }
        if bg {
            if fg {
                out.push(';');
            }
            out.push_str("48;2;");
            rgb_text(out, cell.background);
            *background = cell.background;
        }
        out.push('m');
    }
    out.push(cell.glyph);
}

fn rgb_text(out: &mut String, rgb: u32) {
    static DECIMALS: OnceLock<[String; 256]> = OnceLock::new();
    let decimals = DECIMALS.get_or_init(|| std::array::from_fn(|i| i.to_string()));
    out.push_str(&decimals[(rgb >> 16) as usize]);
    out.push(';');
    out.push_str(&decimals[((rgb >> 8) & 255) as usize]);
    out.push(';');
    out.push_str(&decimals[(rgb & 255) as usize]);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apply_ansi(grid: &mut [Cell], stride: usize, text: &str) {
        let mut remaining = text;
        let (mut x, mut y) = (0, 0);
        let (mut foreground, mut background) = (DEFAULT, DEFAULT);
        while !remaining.is_empty() {
            if let Some(csi) = remaining.strip_prefix("\x1b[") {
                let end = csi.find(|c: char| c.is_ascii_alphabetic()).unwrap();
                let args: Vec<usize> = csi[..end].split(';').map(|n| n.parse().unwrap()).collect();
                match csi.as_bytes()[end] {
                    b'H' => {
                        y = args[0] - 1;
                        x = args[1] - 1;
                    }
                    b'm' => {
                        let mut i = 0;
                        while i < args.len() {
                            match args[i] {
                                0 => {
                                    foreground = DEFAULT;
                                    background = DEFAULT;
                                    i += 1;
                                }
                                38 | 48 => {
                                    assert_eq!(args[i + 1], 2);
                                    let rgb =
                                        ((args[i + 2] << 16) | (args[i + 3] << 8) | args[i + 4])
                                            as u32;
                                    if args[i] == 38 {
                                        foreground = rgb;
                                    } else {
                                        background = rgb;
                                    }
                                    i += 5;
                                }
                                code => panic!("unexpected SGR {code}"),
                            }
                        }
                    }
                    _ => panic!("unexpected CSI"),
                }
                remaining = &csi[end + 1..];
            } else {
                let glyph = remaining.chars().next().unwrap();
                grid[y * stride + x] = Cell {
                    glyph,
                    foreground,
                    background,
                };
                x += 1;
                remaining = &remaining[glyph.len_utf8()..];
            }
        }
    }

    fn visual(cell: Cell) -> (char, u32, u32) {
        match cell.glyph {
            ' ' => (' ', cell.background, cell.background),
            '▄' => ('▀', cell.background, cell.foreground),
            '▀' if cell.foreground == cell.background => (' ', cell.background, cell.background),
            _ => (cell.glyph, cell.foreground, cell.background),
        }
    }

    #[test]
    fn incremental_output_matches_reference_pixels_across_modes() {
        let mut canvas = Canvas::default();
        let mut actual = vec![Cell::default(); 15 * 10];
        let mut seed = 1234567_u32;
        let mut pixels = vec![0; 8 * 6 * 3];
        for mode in [Mode::Blocks, Mode::Ascii, Mode::Blocks] {
            for color in [true, false] {
                for step in 0..12 {
                    for byte in &mut pixels {
                        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                        if step == 0 || seed % 17 == 0 {
                            *byte = (seed >> 24) as u8;
                        }
                    }
                    let output = canvas.update(&pixels, (8, 3), (2, 1), mode, color);
                    apply_ansi(&mut actual, 15, output);
                    let reference = crate::render::lines(&pixels, 8, 3, mode, color);
                    let mut expected = vec![Cell::default(); 15 * 10];
                    for (y, line) in reference.iter().enumerate() {
                        apply_ansi(&mut expected, 15, &format!("\x1b[{};3H{line}", y + 2));
                    }
                    for y in 1..4 {
                        for x in 2..10 {
                            assert_eq!(
                                visual(actual[y * 15 + x]),
                                visual(expected[y * 15 + x]),
                                "{mode:?} color={color} step={step} at {x},{y}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn unchanged_cells_generate_no_output() {
        let mut canvas = Canvas::default();
        assert!(
            canvas
                .update(&[255, 0, 0, 0, 0, 255], (1, 1), (4, 3), Mode::Blocks, true)
                .contains('▀')
        );
        assert!(
            canvas
                .update(&[255, 0, 0, 0, 0, 255], (1, 1), (4, 3), Mode::Blocks, true)
                .is_empty()
        );
    }

    #[test]
    fn one_changed_cell_does_not_redraw_the_row() {
        let mut canvas = Canvas::default();
        let mut rgb = vec![100; 60];
        canvas.update(&rgb, (10, 1), (0, 2), Mode::Blocks, true);
        rgb[12] = 200;
        let delta = canvas.update(&rgb, (10, 1), (0, 2), Mode::Blocks, true);
        assert!(delta.contains("\x1b[3;5H"));
        assert!(delta.len() < 90, "{delta:?}");
    }

    #[test]
    fn uniform_blocks_use_background_only() {
        let mut canvas = Canvas::default();
        let out = canvas.update(&[100; 60], (10, 1), (0, 0), Mode::Blocks, true);
        assert!(!out.contains("38;2;"));
        assert!(out.contains("48;2;100;100;100m"));
    }
}
