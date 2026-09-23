use crate::media::Frame;
use anyhow::{Result, ensure};
use clap::ValueEnum;
use rustfft::{Fft, FftPlanner, num_complex::Complex32};
use std::{fs::File, io::BufReader, path::Path, sync::Arc};

#[cfg(test)]
#[path = "../tests/support/visualizer_reference.rs"]
mod reference;

const FFT_SIZE: usize = 2048;

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Style {
    Bars,
    Wave,
    Orbit,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tone(path: &Path, opposite: bool, silent: bool) -> Result<()> {
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 48000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut wav = hound::WavWriter::create(path, spec)?;
        for i in 0..48000 {
            let value = if silent {
                0
            } else {
                (12000.0 * (i as f32 * std::f32::consts::TAU * 1500.0 / 48000.0).sin()) as i16
            };
            wav.write_sample(value)?;
            wav.write_sample(if opposite { -value } else { value })?;
        }
        wav.finalize()?;
        Ok(())
    }
    #[test]
    fn real_signal_silence_stereo_pause_and_palette() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let a = dir.path().join("a.wav");
        let b = dir.path().join("b.wav");
        let silence = dir.path().join("silence.wav");
        tone(&a, false, false)?;
        tone(&b, true, false)?;
        tone(&silence, false, true)?;
        for style in [Style::Bars, Style::Wave, Style::Orbit] {
            let options = Options {
                style,
                ..Options::default()
            };
            let mut visual = Visualizer::open(&a, (120, 60), 30, options)?;
            assert!(visual.advance(0.2)?);
            let first = visual.current.rgb.clone();
            assert!(first.iter().any(|v| *v > 100));
            assert!(!visual.advance(0.2)?);
            assert_eq!(first, visual.current.rgb);
            let mut muted = Visualizer::open(&silence, (120, 60), 30, options)?;
            muted.advance(0.2)?;
            assert_ne!(first, muted.current.rgb);
            let mut other = Visualizer::open(
                &a,
                (120, 60),
                30,
                Options {
                    theme: Theme::Ember,
                    ..options
                },
            )?;
            other.advance(0.2)?;
            assert_ne!(first, other.current.rgb);
            let mut reversed = Visualizer::open(&b, (120, 60), 30, options)?;
            reversed.advance(0.2)?;
            if style != Style::Wave {
                assert_eq!(first, reversed.current.rgb);
            }
            visual.advance(1.0)?;
        }
        Ok(())
    }
    #[test]
    #[ignore = "CPU benchmark; run with --release --nocapture"]
    fn benchmark_visualizer() -> Result<()> {
        use std::{hint::black_box, time::Instant};
        let dir = tempfile::tempdir()?;
        let file = dir.path().join("tone.wav");
        tone(&file, false, false)?;
        for size in [(160, 90), (500, 280)] {
            for style in [Style::Bars, Style::Wave, Style::Orbit] {
                let mut visual = Visualizer::open(
                    &file,
                    size,
                    60,
                    Options {
                        style,
                        ..Options::default()
                    },
                )?;
                visual.advance(0.2)?;
                for _ in 0..10 {
                    visual.draw();
                }
                let mut samples = Vec::new();
                let mut reference_samples = Vec::new();
                for _ in 0..5 {
                    let started = Instant::now();
                    for _ in 0..200 {
                        black_box(&mut visual).draw();
                        black_box(&visual.current.rgb);
                    }
                    samples.push(started.elapsed().as_secs_f64() * 1000.0 / 200.0);
                    let started = Instant::now();
                    for _ in 0..200 {
                        reference::draw(black_box(&mut visual));
                        black_box(&visual.current.rgb);
                    }
                    reference_samples.push(started.elapsed().as_secs_f64() * 1000.0 / 200.0);
                }
                println!(
                    "VISUAL_BENCH {}",
                    serde_json::json!({"style":format!("{style:?}"), "size":size, "draw_ms":samples, "reference_ms":reference_samples})
                );
            }
        }
        Ok(())
    }

    #[test]
    fn pixels_match_reference() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let file = dir.path().join("tone.wav");
        tone(&file, false, false)?;
        for size in [(1, 1), (2, 3), (37, 19), (160, 90)] {
            for style in [Style::Bars, Style::Wave, Style::Orbit] {
                for (theme, bands, gain, pixel_aspect) in [
                    (Theme::Aurora, 8, 10, 0.25),
                    (Theme::Ember, 48, 100, 1.0),
                    (Theme::Ice, 128, 400, 2.0),
                ] {
                    let mut visual = Visualizer::open(
                        &file,
                        size,
                        60,
                        Options {
                            style,
                            theme,
                            bands,
                            gain,
                            pixel_aspect,
                            ..Options::default()
                        },
                    )?;
                    for frame in 0..4 {
                        visual.advance(frame as f64 * 0.1)?;
                        for (i, level) in visual.levels.iter_mut().enumerate() {
                            *level = ((i * 17 + frame * 11) % 101) as f32 / 100.0;
                            visual.peaks[i] = (*level + 0.15).min(1.0);
                        }
                        visual.draw();
                        let actual = visual.current.rgb.clone();
                        reference::draw(&mut visual);
                        assert_eq!(
                            actual, visual.current.rgb,
                            "{style:?} {theme:?} {size:?} frame {frame}"
                        );
                    }
                }
            }
        }
        Ok(())
    }

    #[test]
    fn spectrum_detects_known_frequency() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let file = dir.path().join("tone.wav");
        tone(&file, false, false)?;
        let mut v = Visualizer::open(&file, (160, 80), 30, Options::default())?;
        v.advance(0.1)?;
        let peak = v
            .energy
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .unwrap()
            .0;
        assert_eq!(peak, 64); // 1500 Hz / (48000 / 2048).
        assert!(v.levels.iter().any(|l| *l > 0.7));
        Ok(())
    }
    #[test]
    #[ignore = "writes optional demonstration frames; set ASIJI_DEMO_DIR"]
    fn export_visualizer_demo() -> Result<()> {
        use std::io::Write;
        let destination = std::env::var_os("ASIJI_DEMO_DIR").expect("ASIJI_DEMO_DIR");
        let destination = Path::new(&destination);
        std::fs::create_dir_all(destination)?;
        let dir = tempfile::tempdir()?;
        let file = dir.path().join("synthetic.wav");
        let mut wav = hound::WavWriter::create(
            &file,
            hound::WavSpec {
                channels: 2,
                sample_rate: 48000,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            },
        )?;
        for i in 0..48000 * 6 {
            let t = i as f32 / 48000.0;
            let channel = |shift: f32| -> i16 {
                let mut value = 0.0;
                for n in 0..32 {
                    let freq = 60.0 * 1.16_f32.powi(n);
                    let amp = (0.5 + 0.5 * (t * (1.4 + n as f32 * 0.13) + n as f32 + shift).sin())
                        .powi(3);
                    value += (t * std::f32::consts::TAU * freq + shift).sin() * amp / 32.0;
                }
                (value * 27000.0) as i16
            };
            wav.write_sample(channel(0.0))?;
            wav.write_sample(channel(0.5))?;
        }
        wav.finalize()?;
        for (name, style, theme) in [
            ("bars", Style::Bars, Theme::Aurora),
            ("wave", Style::Wave, Theme::Ice),
            ("orbit", Style::Orbit, Theme::Ember),
        ] {
            let mut v = Visualizer::open(
                &file,
                (160, 80),
                12,
                Options {
                    style,
                    theme,
                    gain: 160,
                    ..Options::default()
                },
            )?;
            for i in 0..60 {
                v.advance(i as f64 / 12.0)?;
                let mut out = File::create(destination.join(format!("{name}-{i:03}.ppm")))?;
                write!(out, "P6\n160 80\n255\n")?;
                out.write_all(&v.current.rgb)?;
            }
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Theme {
    Aurora,
    Ember,
    Ice,
}

#[derive(Clone, Copy)]
pub struct Options {
    pub style: Style,
    pub theme: Theme,
    pub gain: u16,
    pub smoothing: u8,
    pub bands: usize,
    pub pixel_aspect: f32,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            style: Style::Bars,
            theme: Theme::Aurora,
            gain: 100,
            smoothing: 80,
            bands: 48,
            pixel_aspect: 1.0,
        }
    }
}

pub struct Visualizer {
    reader: hound::WavReader<BufReader<File>>,
    options: Options,
    size: (usize, usize),
    fps: u32,
    fft: Arc<dyn Fft<f32>>,
    buffer: Vec<Complex32>,
    scratch: Vec<Complex32>,
    left: Vec<f32>,
    right: Vec<f32>,
    energy: Vec<f32>,
    levels: Vec<f32>,
    peaks: Vec<f32>,
    ranges: Vec<(usize, usize)>,
    window: Vec<f32>,
    orbit_geometry: Vec<(f32, f32)>,
    wave_columns: Vec<(f32, f32)>,
    pub current: Frame,
}

impl Visualizer {
    pub fn open(path: &Path, size: (usize, usize), fps: u32, options: Options) -> Result<Self> {
        let reader = hound::WavReader::open(path)?;
        let spec = reader.spec();
        ensure!(
            spec.channels == 2
                && spec.bits_per_sample == 16
                && spec.sample_format == hound::SampleFormat::Int,
            "Визуализатору нужен кэш PCM16 stereo"
        );
        ensure!(
            size.0 > 0 && size.1 > 0 && fps > 0 && (8..=128).contains(&options.bands),
            "Некорректный размер визуализатора"
        );
        let fft = FftPlanner::new().plan_fft_forward(FFT_SIZE);
        let scratch = vec![Complex32::default(); fft.get_inplace_scratch_len()];
        let max_hz = 16000.0_f32.min(spec.sample_rate as f32 / 2.0);
        let ranges = (0..options.bands)
            .map(|i| {
                let bin = |i: usize| {
                    ((45.0 * (max_hz / 45.0).powf(i as f32 / options.bands as f32))
                        * FFT_SIZE as f32
                        / spec.sample_rate as f32) as usize
                };
                let low = bin(i).clamp(1, FFT_SIZE / 2 - 1);
                (low, bin(i + 1).clamp(low + 1, FFT_SIZE / 2))
            })
            .collect();
        // Geometry stays fixed until resize recreates the visualizer.
        let orbit_geometry = if options.style == Style::Orbit {
            let (width, height) = size;
            (0..width * height)
                .map(|i| {
                    let u = (i % width) as f32 / width.max(2) as f32;
                    let v = (i / width) as f32 / height.max(2) as f32;
                    let dx = (u - 0.5) * 2.0 * width as f32 / height as f32 * options.pixel_aspect;
                    let dy = (v - 0.5) * 2.0;
                    let angle = (dy.atan2(dx) / std::f32::consts::TAU + 1.0) % 1.0;
                    ((dx * dx + dy * dy).sqrt(), angle)
                })
                .collect()
        } else {
            Vec::new()
        };
        Ok(Self {
            reader,
            options,
            orbit_geometry,
            wave_columns: vec![
                (0.0, 0.0);
                if options.style == Style::Wave {
                    size.0
                } else {
                    0
                }
            ],
            size,
            fps,
            fft,
            scratch,
            buffer: vec![Complex32::default(); FFT_SIZE],
            left: vec![0.0; FFT_SIZE],
            right: vec![0.0; FFT_SIZE],
            energy: vec![0.0; FFT_SIZE / 2],
            levels: vec![0.0; options.bands],
            peaks: vec![0.0; options.bands],
            ranges,
            window: (0..FFT_SIZE)
                .map(|i| {
                    0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / (FFT_SIZE - 1) as f32).cos()
                })
                .collect(),
            current: Frame {
                time: -1.0,
                rgb: vec![0; size.0 * size.1 * 3],
            },
        })
    }

    pub fn advance(&mut self, position: f64) -> Result<bool> {
        let time = (position * self.fps as f64).floor() / self.fps as f64;
        if (time - self.current.time).abs() < 0.5 / self.fps as f64 {
            return Ok(false);
        }
        let spec = self.reader.spec();
        let offset = ((time * spec.sample_rate as f64) as u32).min(self.reader.duration());
        self.reader.seek(offset)?;
        let mut samples = self.reader.samples::<i16>();
        for i in 0..FFT_SIZE {
            self.left[i] = samples.next().transpose()?.unwrap_or(0) as f32 / 32768.0;
            self.right[i] = samples.next().transpose()?.unwrap_or(0) as f32 / 32768.0;
        }
        self.energy.fill(0.0);
        // Power is combined after the FFT so opposite-phase stereo does not cancel.
        for channel in [&self.left, &self.right] {
            for (i, sample) in channel.iter().enumerate() {
                self.buffer[i] = Complex32::new(sample * self.window[i], 0.0);
            }
            self.fft
                .process_with_scratch(&mut self.buffer, &mut self.scratch);
            for (power, bin) in self.energy.iter_mut().zip(&self.buffer) {
                *power += bin.norm_sqr();
            }
        }
        let elapsed = if self.current.time < 0.0 {
            1.0 / self.fps as f64
        } else {
            (time - self.current.time).max(0.0)
        } as f32;
        let retention = (self.options.smoothing as f32 / 100.0).powf(elapsed * 60.0);
        for (index, (low, high)) in self.ranges.iter().enumerate() {
            let amplitude = self.energy[*low..*high]
                .iter()
                .copied()
                .fold(0.0_f32, f32::max)
                .sqrt()
                / FFT_SIZE as f32;
            let db = 20.0
                * (amplitude * self.options.gain as f32 / 100.0)
                    .max(1e-6)
                    .log10();
            let target = ((db + 65.0) / 60.0).clamp(0.0, 1.0);
            self.levels[index] = if target > self.levels[index] {
                target
            } else {
                self.levels[index] * retention + target * (1.0 - retention)
            };
            self.peaks[index] = (self.peaks[index] - elapsed * 0.28).max(self.levels[index]);
        }
        self.current.time = time;
        self.draw();
        Ok(true)
    }

    fn draw(&mut self) {
        let (width, height) = self.size;
        let bands = self.options.bands;
        let palette = match self.options.theme {
            Theme::Aurora => ([37.0, 238.0, 191.0], [148.0, 96.0, 255.0]),
            Theme::Ember => ([255.0, 190.0, 62.0], [255.0, 57.0, 119.0]),
            Theme::Ice => ([100.0, 181.0, 255.0], [222.0, 250.0, 255.0]),
        };
        for (x, column) in self.wave_columns.iter_mut().enumerate() {
            let index = x * (FFT_SIZE - 1) / width.max(2);
            let gain = self.options.gain as f32 / 100.0;
            *column = (
                0.32 + (self.left[index] * gain).clamp(-1.0, 1.0) * 0.23,
                0.70 + (self.right[index] * gain).clamp(-1.0, 1.0) * 0.23,
            );
        }
        for y in 0..height {
            for x in 0..width {
                let u = x as f32 / width.max(2) as f32;
                let v = y as f32 / height.max(2) as f32;
                let band = (x * bands / width).min(bands - 1);
                let (strength, tint) = match self.options.style {
                    Style::Bars => {
                        let baseline = 0.76;
                        let distance = (v - baseline).abs();
                        let reflection = v > baseline;
                        let level = self.levels[band] * if reflection { 0.22 } else { 0.68 };
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
                            && (distance - self.peaks[band] * 0.68).abs() < 1.0 / height as f32;
                        (
                            if peak && self.peaks[band] > 0.015 {
                                1.0
                            } else {
                                fill
                            },
                            u,
                        )
                    }
                    Style::Wave => {
                        let (a, b) = self.wave_columns[x];
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
                        let (radius, angle) = self.orbit_geometry[y * width + x];
                        let index = (angle * bands as f32) as usize % bands;
                        let edge = 0.40 + self.levels[index] * 0.42;
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
                    self.current.rgb[pixel + c] =
                        ((palette.0[c] * (1.0 - tint) + palette.1[c] * tint) * strength.min(1.0))
                            as u8;
                }
            }
        }
    }
}
