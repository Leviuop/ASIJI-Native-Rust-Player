use super::*;

pub(super) fn draw(visual: &mut Visualizer) {
    let (width, height) = visual.size;
    let bands = visual.options.bands;
    let palette = match visual.options.theme {
        Theme::Aurora => ([37.0, 238.0, 191.0], [148.0, 96.0, 255.0]),
        Theme::Ember => ([255.0, 190.0, 62.0], [255.0, 57.0, 119.0]),
        Theme::Ice => ([100.0, 181.0, 255.0], [222.0, 250.0, 255.0]),
    };
    for y in 0..height {
        for x in 0..width {
            let u = x as f32 / width.max(2) as f32;
            let v = y as f32 / height.max(2) as f32;
            let band = (x * bands / width).min(bands - 1);
            let (strength, tint) = match visual.options.style {
                Style::Bars => {
                    let baseline = 0.76;
                    let distance = (v - baseline).abs();
                    let reflection = v > baseline;
                    let level = visual.levels[band] * if reflection { 0.22 } else { 0.68 };
                    let gap = width >= bands * 3 && (x * bands % width) < bands;
                    let fill = if !gap && distance < level {
                        if reflection {
                            0.26 * (1.0 - distance / 0.24).max(0.0)
                        } else {
                            0.65 + 0.35 * (1.0 - v)
                        }
                    } else {
                        0.0
                    };
                    let peak = !reflection
                        && !gap
                        && (distance - visual.peaks[band] * 0.68).abs() < 1.0 / height as f32;
                    (
                        if peak && visual.peaks[band] > 0.015 {
                            1.0
                        } else {
                            fill
                        },
                        u,
                    )
                }
                Style::Wave => {
                    let index = x * (FFT_SIZE - 1) / width.max(2);
                    let gain = visual.options.gain as f32 / 100.0;
                    let a = 0.32 + (visual.left[index] * gain).clamp(-1.0, 1.0) * 0.23;
                    let b = 0.70 + (visual.right[index] * gain).clamp(-1.0, 1.0) * 0.23;
                    let distance = (v - a).abs().min((v - b).abs()) * height as f32;
                    (
                        (1.3 - distance).clamp(0.0, 1.0) + (4.0 - distance).max(0.0) * 0.035,
                        if (v - a).abs() < (v - b).abs() {
                            u * 0.4
                        } else {
                            0.6 + u * 0.4
                        },
                    )
                }
                Style::Orbit => {
                    let dx = (u - 0.5) * 2.0 * width as f32 / height as f32
                        * visual.options.pixel_aspect;
                    let dy = (v - 0.5) * 2.0;
                    let angle = (dy.atan2(dx) / std::f32::consts::TAU + 1.0) % 1.0;
                    let index = (angle * bands as f32) as usize % bands;
                    let radius = (dx * dx + dy * dy).sqrt();
                    let edge = 0.40 + visual.levels[index] * 0.42;
                    let fill = if radius > 0.40 && radius < edge {
                        0.50 + (radius - 0.40)
                    } else {
                        0.0
                    };
                    let ring = (1.0 - (radius - 0.40).abs() * height as f32).max(0.0) * 0.45;
                    (fill.max(ring), angle)
                }
            };
            let pixel = (y * width + x) * 3;
            for c in 0..3 {
                visual.current.rgb[pixel + c] =
                    ((palette.0[c] * (1.0 - tint) + palette.1[c] * tint) * strength.min(1.0)) as u8;
            }
        }
    }
}
