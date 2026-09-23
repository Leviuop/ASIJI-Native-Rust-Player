mod audio;
mod bindings;
mod cache;
mod canvas;
mod config;
mod doctor;
mod import;
#[cfg(test)]
mod integration_tests;
mod media;
mod profile;
mod render;
mod settings_menu;
mod timing;
mod visualizer;

use anyhow::{Context, Result, bail};
use audio::Audio;
use bindings::{Bindings, Control};
use clap::{CommandFactory, FromArgMatches, Parser};
use crossterm::{
    cursor,
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use media::{Decoder, Tools, Track, Video, VideoInfo, safe_text};
use render::{Mode, Screen};
use std::{
    fs,
    io::{self, IsTerminal, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

#[derive(Parser, Clone)]
#[command(version, about = "ASIJI — нативный ASCII/HD музыкальный плеер")]
struct Args {
    #[arg(skip)]
    bindings: Bindings,
    #[arg(long, conflicts_with = "no_config", help = "Путь к config.toml")]
    config: Option<PathBuf>,
    #[arg(long, help = "Не читать файл настроек")]
    no_config: bool,
    #[arg(long, conflicts_with_all = ["no_config", "print_config"], help = "Создать пример конфига и выйти")]
    init_config: bool,
    #[arg(long, help = "Показать итоговые настройки и выйти")]
    print_config: bool,
    #[arg(long, default_value_t = 10, value_parser = clap::value_parser!(u8).range(0..=100), help = "Громкость 0..100")]
    volume: u8,
    #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
    muted: bool,
    #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
    repeat: bool,
    #[arg(
        long,
        conflicts_with = "mono",
        help = "Цветной вывод, включая отмену color=false из конфига"
    )]
    color: bool,
    #[arg(long)]
    media: Option<PathBuf>,
    #[arg(long, default_value_t = 30, value_parser = clap::value_parser!(u32).range(10..=60))]
    fps: u32,
    #[arg(
        long,
        default_value_t = 0,
        help = "0 — всё окно; 40..1000 — лимит колонок"
    )]
    width: usize,
    #[arg(long, value_enum, default_value_t = Mode::Ascii)]
    mode: Mode,
    #[arg(long, value_enum, default_value_t = visualizer::Style::Bars)]
    visualizer: visualizer::Style,
    #[arg(long, value_enum, default_value_t = visualizer::Theme::Aurora)]
    visual_theme: visualizer::Theme,
    #[arg(long, default_value_t = 100, value_parser = clap::value_parser!(u16).range(10..=400))]
    visual_gain: u16,
    #[arg(long, default_value_t = 80, value_parser = clap::value_parser!(u8).range(0..=99))]
    visual_smoothing: u8,
    #[arg(long, default_value_t = 48, value_parser = clap::value_parser!(u16).range(8..=128))]
    visual_bands: u16,
    #[arg(long)]
    mono: bool,
    #[arg(long)]
    check: bool,
    #[arg(long, help = "Проверить FFmpeg и GPU без запуска плеера")]
    doctor: bool,
    #[arg(long, help = "Очистить звуковой кэш и выйти")]
    clear_cache: bool,
    #[arg(long)]
    cache_dir: Option<PathBuf>,
    #[arg(long, default_value_t = 1024)]
    cache_max_mb: u64,
    #[arg(long, default_value_t = 30)]
    cache_max_days: u64,
    #[arg(long, value_enum, default_value_t = import::ImportMode::Copy)]
    import_mode: import::ImportMode,
    #[arg(long, value_enum, default_value_t = import::Conflict::Skip)]
    import_conflict: import::Conflict,
    #[arg(
        long,
        requires = "doctor",
        help = "Проверить декодирование конкретного видео"
    )]
    doctor_video: Option<PathBuf>,
    #[arg(
        long,
        help = "GPU: индекс для CUDA/D3D11VA или /dev/dri/renderD128 для VAAPI"
    )]
    hwaccel_device: Option<String>,
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set, help = "Переход на CPU при ошибке GPU")]
    hwaccel_fallback: bool,
    #[arg(long, help = "Замер ANSI-рендера из файла RGB24 (без терминала)")]
    benchmark: Option<PathBuf>,
    #[arg(long, default_value_t = 140)]
    bench_rows: usize,
    #[arg(long, help = "Записать измерения воспроизведения в JSON")]
    profile: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = 0,
        help = "Завершить воспроизведение через N секунд (для замеров)"
    )]
    profile_seconds: u32,
    #[arg(long, help = "Сразу включить трек с указанным номером")]
    play: Option<usize>,
    #[arg(long, value_enum, default_value_t = Decoder::Auto, help = "Автовыбор самого быстрого декодера или явный CPU/GPU")]
    hwaccel: Decoder,
    #[arg(
        long,
        help = "Сравнить старый и новый вывод на последовательности кадров клипа"
    )]
    benchmark_video: Option<PathBuf>,
    #[arg(long, default_value_t = 120, value_parser = clap::value_parser!(u32).range(2..=1000))]
    bench_frames: u32,
}

struct Settings {
    bindings: Bindings,
    volume: f32,
    muted: bool,
    color: bool,
    repeat: bool,
    mode: Mode,
}

struct Terminal;
impl Terminal {
    fn open() -> Result<Self> {
        anyhow::ensure!(
            io::stdin().is_terminal() && io::stdout().is_terminal(),
            "Запустите плеер в терминале"
        );
        terminal::enable_raw_mode()?;
        let guard = Self;
        execute!(io::stdout(), EnterAlternateScreen, cursor::Hide)?;
        Ok(guard)
    }
}
impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = write!(io::stdout(), "\x1b[?2026l\x1b[0m");
        let _ = execute!(io::stdout(), cursor::Show, LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
    }
}

#[derive(PartialEq, Debug)]
enum Action {
    Menu,
    Exit,
    Next,
    Previous,
    Ended,
}

fn clock(seconds: f64) -> String {
    let s = seconds as u64;
    format!("{:02}:{:02}", s / 60, s % 60)
}

fn geometry(info: VideoInfo, size: (u16, u16), limit: usize) -> (usize, usize) {
    let columns = (size.0.saturating_sub(1) as usize).min(if limit == 0 { 1000 } else { limit });
    render::fit(
        info.width,
        info.height,
        columns,
        size.1.saturating_sub(7) as usize,
    )
}

struct Clip<'a> {
    tools: Tools,
    fallback_notice: bool,
    path: &'a Path,
    info: VideoInfo,
    spectrum: bool,
    fps: u32,
    visual: visualizer::Options,
}

enum Playback {
    Video(Video),
    Visual(Box<visualizer::Visualizer>),
}
impl Playback {
    fn current(&self) -> Option<&media::Frame> {
        match self {
            Self::Video(v) => v.current.as_ref(),
            Self::Visual(v) => Some(&v.current),
        }
    }
    fn advance(&mut self, position: f64) -> Result<bool> {
        match self {
            Self::Video(v) => v.advance(position),
            Self::Visual(v) => v.advance(position),
        }
    }
}

impl Clip<'_> {
    fn open(&mut self, size: (usize, usize), mode: Mode, position: f64) -> Result<Playback> {
        let pixels_high = size.1 * if mode == Mode::Blocks { 2 } else { 1 };
        if self.spectrum {
            let mut options = self.visual;
            options.pixel_aspect = if mode == Mode::Blocks { 1.0 } else { 0.5 };
            let mut visual =
                visualizer::Visualizer::open(self.path, (size.0, pixels_high), self.fps, options)?;
            visual.advance(position)?;
            return Ok(Playback::Visual(Box::new(visual)));
        }
        let mut video = Video::open(
            &self.tools,
            self.path,
            self.info,
            (size.0, pixels_high),
            self.fps,
            position,
            self.spectrum,
        )?;
        if let Err(error) = video.first() {
            if self.spectrum || self.tools.decoder == Decoder::Cpu || !self.tools.fallback {
                return Err(error);
            }
            drop(video);
            self.tools.decoder = Decoder::Cpu;
            self.fallback_notice = true;
            video = Video::open(
                &self.tools,
                self.path,
                self.info,
                (size.0, pixels_high),
                self.fps,
                position,
                self.spectrum,
            )?;
            video
                .first()
                .with_context(|| format!("GPU: {error:#}; CPU"))?;
        }
        Ok(Playback::Video(video))
    }
}

fn screen_rows(
    track: &Track,
    audio: &Audio,
    settings: &Settings,
    size: (u16, u16),
    frame_size: (usize, usize),
    fps: f64,
    decoder: Decoder,
) -> Vec<String> {
    let width = size.0.saturating_sub(1) as usize;
    let area = size.1.saturating_sub(7) as usize;
    let mut rows = vec![
        render::clip(
            &format!(
                " ASIJI / RUST / {} / {}",
                decoder.label(),
                safe_text(&track.title)
            ),
            width,
        ),
        String::new(),
    ];
    rows.resize(2 + area, String::new());
    let volume = if settings.muted {
        "MUTE".into()
    } else {
        format!("{:.0}%", settings.volume * 100.0)
    };
    let state = if audio.sink.is_paused() {
        "PAUSE"
    } else {
        "PLAY"
    };
    let mode = if settings.mode == Mode::Blocks {
        "HD"
    } else {
        "ASCII"
    };
    let color = if settings.color { "COLOR" } else { "MONO" };
    let repeat = if settings.repeat { "ON" } else { "OFF" };
    rows.push(render::clip(
        &format!(
            " {state} {} / {}  VOL {volume}  {mode} {}x{} {color} {fps:.0} FPS REPEAT {repeat}",
            clock(audio.position()),
            clock(audio.duration),
            frame_size.0,
            frame_size.1
        ),
        width,
    ));
    let bar = width.saturating_sub(4).min(60);
    let filled = ((audio.position() / audio.duration * bar as f64).round() as usize).min(bar);
    rows.push(format!(
        " [{}{}]",
        "=".repeat(filled),
        "-".repeat(bar - filled)
    ));
    rows.push(render::clip(
        &format!(
            " {}: pause | {}/{}: seek 5s | {}/{}: volume",
            settings.bindings.hint("pause"),
            settings.bindings.hint("seek_backward"),
            settings.bindings.hint("seek_forward"),
            settings.bindings.hint("volume_up"),
            settings.bindings.hint("volume_down")
        ),
        width,
    ));
    rows.push(render::clip(
        &format!(" {}: HD/ASCII | {}/{}: track | {}: mute | {}: color | {}: repeat | {}: menu | {}: exit", settings.bindings.hint("mode"), settings.bindings.hint("next"), settings.bindings.hint("previous"), settings.bindings.hint("mute"), settings.bindings.hint("color"), settings.bindings.hint("repeat"), settings.bindings.hint("menu"), settings.bindings.hint("quit")),
        width,
    ));
    rows.truncate(size.1.saturating_sub(1) as usize);
    rows
}

fn play(
    track: &Track,
    tools: &Tools,
    root: &Path,
    args: &Args,
    settings: &mut Settings,
) -> Result<Action> {
    let mut metrics = profile::Profile::new(args.profile.as_deref());
    let cache_path = args
        .cache_dir
        .clone()
        .unwrap_or_else(|| cache::default_path(root));
    let cache = cache::Cache::open(&cache_path, args.cache_max_mb, args.cache_max_days)?;
    cache.prune(None, false)?;
    let wav = tools.prepare_audio(&track.audio, &cache_path)?;
    cache.prune(Some(&wav), false)?;
    let audio = Audio::open(&wav, if settings.muted { 0.0 } else { settings.volume })?;
    let info = match &track.video {
        Some(path) => tools.probe(path)?,
        None => VideoInfo {
            width: 640,
            height: 240,
            duration: audio.duration,
            fps: args.fps as f64,
        },
    };
    let effective_fps = args.fps.min(info.fps.ceil() as u32).max(1);
    let initial_size = terminal::size()?;
    let initial_dimensions = geometry(info, initial_size, args.width);
    let selected_tools = if let Some(path) = &track.video {
        tools.select_decoder(
            path,
            info,
            (
                initial_dimensions.0,
                initial_dimensions.1 * if settings.mode == Mode::Blocks { 2 } else { 1 },
            ),
            effective_fps,
            args.hwaccel,
        )?
    } else {
        tools.clone()
    };
    metrics.decoder = selected_tools.decoder.label().to_owned();
    metrics.requested_fps = args.fps;
    metrics.effective_fps = effective_fps;
    let mut clip = Clip {
        tools: selected_tools.clone(),
        fallback_notice: track.video.is_some()
            && args.hwaccel != Decoder::Cpu
            && args.hwaccel != Decoder::Auto
            && selected_tools.decoder == Decoder::Cpu,
        path: track.video.as_deref().unwrap_or(&wav),
        info,
        spectrum: track.video.is_none(),
        fps: effective_fps,
        visual: visualizer::Options {
            style: args.visualizer,
            theme: args.visual_theme,
            gain: args.visual_gain,
            smoothing: args.visual_smoothing,
            bands: args.visual_bands as usize,
            pixel_aspect: 1.0,
        },
    };
    let _terminal = Terminal::open()?;
    let _timer = timing::PlaybackTimer::start()?;
    let mut size = terminal::size()?;
    anyhow::ensure!(
        size.0 >= 40 && size.1 >= 12,
        "Увеличьте окно терминала хотя бы до 40×12"
    );
    let mut dimensions = geometry(info, size, args.width);
    metrics.columns = dimensions.0;
    metrics.rows = dimensions.1;
    let mut video = clip.open(dimensions, settings.mode, 0.0)?;
    let mut screen = Screen::default();
    let mut canvas = canvas::Canvas::default();
    let mut dirty = true;
    let mut hud_time = Instant::now();
    let mut count = 0;
    let mut fps = effective_fps as f64;
    audio.sink.play();
    let play_started = Instant::now();
    metrics.playback_started = Some(play_started);
    loop {
        if args.profile_seconds > 0
            && play_started.elapsed().as_secs() >= args.profile_seconds as u64
        {
            return Ok(Action::Exit);
        }
        metrics.iterations += 1;
        let mut restart = None;
        while event::poll(Duration::ZERO)? {
            match event::read()? {
                Event::Key(key) if key.kind != KeyEventKind::Release => {
                    let Some(control) = settings.bindings.action(key) else {
                        continue;
                    };
                    match control {
                        Control::Quit => return Ok(Action::Exit),
                        Control::Menu => return Ok(Action::Menu),
                        Control::Next => return Ok(Action::Next),
                        Control::Previous => return Ok(Action::Previous),
                        Control::Pause => {
                            if audio.sink.is_paused() {
                                audio.sink.play();
                            } else {
                                audio.sink.pause();
                            }
                        }
                        Control::Backward => restart = Some((audio.position() - 5.0).max(0.0)),
                        Control::Forward => {
                            restart = Some((audio.position() + 5.0).min(audio.duration))
                        }
                        Control::VolumeUp => settings.volume = (settings.volume + 0.05).min(1.0),
                        Control::VolumeDown => settings.volume = (settings.volume - 0.05).max(0.0),
                        Control::Mute => settings.muted = !settings.muted,
                        Control::Color => settings.color = !settings.color,
                        Control::Repeat => settings.repeat = !settings.repeat,
                        Control::Mode => {
                            settings.mode = if settings.mode == Mode::Ascii {
                                Mode::Blocks
                            } else {
                                Mode::Ascii
                            };
                            restart = Some(audio.position());
                        }
                    }
                    audio
                        .sink
                        .set_volume(if settings.muted { 0.0 } else { settings.volume });
                    dirty = true;
                }
                Event::Resize(w, h) => {
                    anyhow::ensure!(
                        w >= 40 && h >= 12,
                        "Окно слишком маленькое: увеличьте его и выберите трек снова"
                    );
                    size = (w, h);
                    canvas.invalidate();
                    dimensions = geometry(info, size, args.width);
                    metrics.columns = dimensions.0;
                    metrics.rows = dimensions.1;
                    restart = Some(audio.position());
                    dirty = true;
                }
                _ => {}
            }
        }
        if restart.is_some_and(|p| p >= audio.duration) || (restart.is_none() && audio.sink.empty())
        {
            if settings.repeat {
                restart = Some(0.0);
            } else {
                return Ok(Action::Ended);
            }
        }
        if let Some(position) = restart {
            let paused = audio.sink.is_paused();
            audio.sink.pause();
            audio.seek(position)?;
            // Drop the old decoder first so repeated resize/seek never accumulates workers.
            drop(video);
            video = clip.open(dimensions, settings.mode, position)?;
            if !paused {
                audio.sink.play();
            }
            dirty = true;
        }
        let decode_start = Instant::now();
        let changed = match video.advance(audio.position()) {
            Ok(changed) => changed,
            Err(error) if clip.tools.decoder != Decoder::Cpu && clip.tools.fallback => {
                let paused = audio.sink.is_paused();
                audio.sink.pause();
                let position = audio.position();
                drop(video);
                clip.tools.decoder = Decoder::Cpu;
                clip.fallback_notice = true;
                video = clip
                    .open(dimensions, settings.mode, position)
                    .with_context(|| format!("GPU: {error:#}; CPU"))?;
                if !paused {
                    audio.sink.play();
                }
                dirty = true;
                true
            }
            Err(error) => return Err(error),
        };
        if metrics.decoder != clip.tools.decoder.label() {
            metrics.decoder = clip.tools.decoder.label().to_owned();
        }
        if let Some(frame) = video.current() {
            metrics.max_video_lag = metrics.max_video_lag.max(audio.position() - frame.time);
        }
        metrics.decode_wait += decode_start.elapsed();
        let render_start = Instant::now();
        let hud_due = hud_time.elapsed() >= Duration::from_millis(500);
        if hud_due {
            fps = count as f64 / hud_time.elapsed().as_secs_f64();
            count = 0;
            hud_time = Instant::now();
        }
        if changed || dirty || hud_due {
            let mut rows = screen_rows(
                track,
                &audio,
                settings,
                size,
                dimensions,
                fps,
                clip.tools.decoder,
            );
            if clip.spectrum {
                rows[0] = render::clip(
                    &format!(
                        " ASIJI / {:?} / {:?} / {}",
                        args.visualizer,
                        args.visual_theme,
                        safe_text(&track.title)
                    ),
                    size.0.saturating_sub(1) as usize,
                );
            }
            if clip.fallback_notice && rows.len() > 1 {
                rows[1] = render::clip(
                    " GPU unavailable; continued on CPU. See --doctor.",
                    size.0.saturating_sub(1) as usize,
                );
            }
            let output = screen.update(rows, size);
            let frame = video.current().context("Нет видеокадра")?;
            let origin = (
                (size.0.saturating_sub(1) as usize).saturating_sub(dimensions.0) / 2,
                2 + (size.1.saturating_sub(7) as usize).saturating_sub(dimensions.1) / 2,
            );
            let image = if changed || dirty {
                canvas.update(
                    &frame.rgb,
                    dimensions,
                    origin,
                    settings.mode,
                    settings.color,
                )
            } else {
                ""
            };
            metrics.render += render_start.elapsed();
            metrics.bytes += (output.len() + image.len()) as u64;
            metrics.frames += u64::from(changed || dirty);
            let write_start = Instant::now();
            if !output.is_empty() || !image.is_empty() {
                let mut stdout = io::stdout().lock();
                stdout.write_all(b"\x1b[?2026h")?;
                stdout.write_all(output.as_bytes())?;
                stdout.write_all(image.as_bytes())?;
                stdout.write_all(b"\x1b[?2026l")?;
                stdout.flush()?;
            }
            metrics.write += write_start.elapsed();
            dirty = false;
        }
        count += u32::from(changed);
        // Follow the audio timeline instead of accumulating late wake-ups.
        let remaining = if audio.sink.is_paused() {
            Duration::from_millis(200)
        } else {
            let next = video.current().map_or(audio.position(), |frame| frame.time)
                + 1.0 / effective_fps as f64;
            Duration::from_secs_f64(
                (next - audio.position()).clamp(0.001, 1.0 / effective_fps as f64),
            )
        };
        if !remaining.is_zero() {
            let wait_start = Instant::now();
            let _ = event::poll(remaining)?;
            metrics.wait += wait_start.elapsed();
        }
    }
}

fn project_root() -> Result<PathBuf> {
    let exe = std::env::current_exe()?;
    if cfg!(target_os = "linux") && exe.starts_with("/usr/") {
        let base = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/share")))
            .context("Не найден пользовательский каталог данных")?;
        return Ok(base.join("asiji"));
    }
    for path in exe.ancestors().skip(1).filter(|p| p.parent().is_some()) {
        if path.join("Cargo.toml").is_file() || path.join("media").is_dir() {
            return Ok(path.to_owned());
        }
    }
    Ok(exe.parent().context("Папка плеера")?.to_owned())
}

fn benchmark(args: &Args, path: &Path) -> Result<()> {
    anyhow::ensure!(
        (1..=1000).contains(&args.width) && (1..=1000).contains(&args.bench_rows),
        "Для замера нужны --width и --bench-rows в диапазоне 1..1000"
    );
    let rgb = fs::read(path)?;
    let factor = if args.mode == Mode::Blocks { 2 } else { 1 };
    anyhow::ensure!(
        rgb.len() == args.width * args.bench_rows * factor * 3,
        "Размер файла не соответствует RGB24-сетке"
    );
    let _ = render::lines(&rgb, args.width, args.bench_rows, args.mode, !args.mono);
    let start = Instant::now();
    let mut bytes = 0;
    for _ in 0..100 {
        let result = std::hint::black_box(render::lines(
            std::hint::black_box(&rgb),
            args.width,
            args.bench_rows,
            args.mode,
            !args.mono,
        ));
        bytes = result.iter().map(String::len).sum::<usize>();
    }
    println!(
        "{}",
        serde_json::json!({"ms_per_frame": start.elapsed().as_secs_f64() * 10.0, "bytes": bytes, "iterations": 100})
    );
    Ok(())
}

fn benchmark_video(args: &Args, path: &Path) -> Result<()> {
    let width = if args.width == 0 { 500 } else { args.width };
    anyhow::ensure!(
        (1..=1000).contains(&width) && (1..=1000).contains(&args.bench_rows),
        "Некорректный размер бенчмарка"
    );
    let mut tools = Tools::find(&project_root()?);
    tools.device = args.hwaccel_device.clone();
    tools.fallback = args.hwaccel_fallback;
    let info = tools.probe(path)?;
    let fps = args.fps.min(info.fps.ceil() as u32).max(1);
    let pixels_high = args.bench_rows * if args.mode == Mode::Blocks { 2 } else { 1 };
    let tools = tools.select_decoder(path, info, (width, pixels_high), fps, args.hwaccel)?;
    let mut video = Video::open(
        &tools,
        path,
        info,
        (width, pixels_high),
        fps,
        20.0_f64.rem_euclid(info.duration),
        false,
    )?;
    let mut screen = Screen::default();
    let mut canvas = canvas::Canvas::default();
    let (mut old_ns, mut new_ns, mut old_bytes, mut new_bytes) = (0_u128, 0_u128, 0_u64, 0_u64);
    for _ in 0..args.bench_frames {
        video.first()?;
        let frame = &video.current.as_ref().unwrap().rgb;
        let start = Instant::now();
        let rows = render::lines(frame, width, args.bench_rows, args.mode, !args.mono);
        let old = screen.update(rows, (width as u16 + 1, args.bench_rows as u16 + 1));
        old_ns += start.elapsed().as_nanos();
        old_bytes += old.len() as u64;
        let start = Instant::now();
        let new = canvas.update(
            frame,
            (width, args.bench_rows),
            (0, 0),
            args.mode,
            !args.mono,
        );
        new_ns += start.elapsed().as_nanos();
        new_bytes += new.len() as u64;
    }
    let report = serde_json::json!({"frames":args.bench_frames,"columns":width,"rows":args.bench_rows,
        "decoder":tools.decoder.label(),"mode":format!("{:?}",args.mode),
        "old_ms_per_frame":old_ns as f64/args.bench_frames as f64/1e6,
        "new_ms_per_frame":new_ns as f64/args.bench_frames as f64/1e6,
        "old_bytes":old_bytes,"new_bytes":new_bytes,
        "scope":"Same decoded RGB sequence: previous row-diff encoder vs exact cell-diff encoder; excludes terminal I/O"});
    let json = serde_json::to_string_pretty(&report)?;
    if let Some(path) = &args.profile {
        fs::write(path, &json)?;
    }
    println!("{json}");
    Ok(())
}

fn run() -> Result<()> {
    let matches = Args::command().get_matches();
    let mut args = Args::from_arg_matches(&matches)?;
    if args.init_config {
        let path = config::initialize(args.config.as_deref())?;
        println!("Создан конфиг: {}", path.display());
        return Ok(());
    }
    config::load(&mut args, &matches)?;
    if args.print_config {
        print!("{}", config::effective(&args)?);
        return Ok(());
    }
    if args.clear_cache {
        let path = args
            .cache_dir
            .clone()
            .unwrap_or_else(|| cache::default_path(&project_root().unwrap_or_default()));
        let cache = cache::Cache::open(&path, args.cache_max_mb, args.cache_max_days)?;
        println!("Удалено из кэша: {} байт", cache.prune(None, true)?);
        return Ok(());
    }
    if args.doctor {
        let mut tools = Tools::find(&project_root()?);
        tools.device = args.hwaccel_device.clone();
        return doctor::run(&tools, args.doctor_video.as_deref());
    }
    if let Some(path) = &args.benchmark_video {
        return benchmark_video(&args, path);
    }
    if let Some(path) = &args.benchmark {
        return benchmark(&args, path);
    }
    if args.width != 0 && !(40..=1000).contains(&args.width) {
        bail!("--width должен быть 0 или 40..1000");
    }
    let root = project_root()?;
    let mut folder = args.media.clone().unwrap_or_else(|| root.join("media"));
    fs::create_dir_all(&folder)?;
    let mut tools = Tools::find(&root);
    tools.device = args.hwaccel_device.clone();
    tools.fallback = args.hwaccel_fallback;
    tools.check()?;
    if args.check {
        use rodio::DeviceTrait;
        use rodio::cpal::traits::HostTrait;
        let device = rodio::cpal::default_host()
            .default_output_device()
            .context("Нет устройства вывода звука")?;
        println!(
            "ASIJI {} / Rust\nFFmpeg: {}\nFFprobe: {}\nАудиоустройство: {}\nМедиатека: {}\nТреков: {}",
            env!("CARGO_PKG_VERSION"),
            tools.ffmpeg.display(),
            tools.ffprobe.display(),
            device.name()?,
            folder.display(),
            media::discover(&folder)?.len()
        );
        return Ok(());
    }
    let mut settings = Settings {
        bindings: args.bindings.clone(),
        volume: args.volume as f32 / 100.0,
        muted: args.muted,
        color: !args.mono,
        repeat: args.repeat,
        mode: args.mode,
    };
    let mut index = args.play.map(|n| n.saturating_sub(1));
    let mut menu_error = None;
    let mut import_report = None;
    if let Some(number) = args.play {
        anyhow::ensure!(
            number > 0 && number <= media::discover(&folder)?.len(),
            "Некорректный номер трека"
        );
    }
    loop {
        let tracks = media::discover(&folder)?;
        if index.is_none() {
            if io::stdout().is_terminal() {
                execute!(
                    io::stdout(),
                    terminal::Clear(terminal::ClearType::All),
                    terminal::Clear(terminal::ClearType::Purge),
                    cursor::MoveTo(0, 0)
                )?;
            }
            println!("\n  ASIJI / RUST / MUSIC IN CHARACTERS\n");
            if let Some(error) = menu_error.take() {
                println!("Ошибка: {error}\n");
            }
            if let Some(report) = import_report.take() {
                println!("{report}\n");
            }
            if tracks.is_empty() {
                println!(
                    "Добавьте Название.mp3 + Название.mp4 или клип со звуком в {}",
                    folder.display()
                );
            }
            for (i, track) in tracks.iter().enumerate() {
                println!(
                    "  {:>3}. {}  [{}]",
                    i + 1,
                    safe_text(&track.title),
                    if track.video.is_some() {
                        "VIDEO"
                    } else {
                        "AUDIO"
                    }
                );
            }
            println!(
                "\nПеретащите аудио/видео сюда и нажмите Enter. Режим: {:?}; конфликт: {:?}",
                args.import_mode, args.import_conflict
            );
            println!("S — настройки / C — очистить кэш");
            print!("Номер трека / R — обновить / Q — выход: ");
            io::stdout().flush()?;
            let mut input = String::new();
            if io::stdin().read_line(&mut input)? == 0 {
                break;
            }
            if matches!(input.trim().to_lowercase().as_str(), "q" | "й") {
                break;
            }
            if input.trim().eq_ignore_ascii_case("s") {
                args.volume = (settings.volume * 100.0).round() as u8;
                args.muted = settings.muted;
                args.repeat = settings.repeat;
                args.mode = settings.mode;
                args.mono = !settings.color;
                match settings_menu::open(&mut args) {
                    Ok(true) => {
                        folder = args.media.clone().unwrap_or_else(|| root.join("media"));
                        fs::create_dir_all(&folder)?;
                        tools.device = args.hwaccel_device.clone();
                        tools.fallback = args.hwaccel_fallback;
                        settings = Settings {
                            bindings: args.bindings.clone(),
                            volume: args.volume as f32 / 100.0,
                            muted: args.muted,
                            color: !args.mono,
                            repeat: args.repeat,
                            mode: args.mode,
                        };
                        import_report = Some("Настройки сохранены".into());
                    }
                    Ok(false) => (),
                    Err(error) => menu_error = Some(format!("{error:#}")),
                }
                continue;
            }
            if input.trim().eq_ignore_ascii_case("c") {
                let path = args
                    .cache_dir
                    .clone()
                    .unwrap_or_else(|| cache::default_path(&root));
                match cache::Cache::open(&path, args.cache_max_mb, args.cache_max_days)
                    .and_then(|c| c.prune(None, true))
                {
                    Ok(bytes) => import_report = Some(format!("Кэш очищен: {bytes} байт")),
                    Err(error) => menu_error = Some(format!("{error:#}")),
                }
                continue;
            }
            if input.trim().is_empty() || matches!(input.trim().to_lowercase().as_str(), "r" | "к")
            {
                continue;
            }
            if input.trim().parse::<usize>().is_err() {
                match import::paths(&input, cfg!(windows)) {
                    Ok(paths) => {
                        import_report = Some(import::batch(
                            paths,
                            &folder,
                            args.import_mode,
                            args.import_conflict,
                        ));
                    }
                    Err(error) => menu_error = Some(safe_text(&format!("{error:#}"))),
                }
                continue;
            }
            index = input
                .trim()
                .parse::<usize>()
                .ok()
                .filter(|&n| n > 0 && n <= tracks.len())
                .map(|n| n - 1);
            if index.is_none() {
                continue;
            }
        }
        if tracks.is_empty() {
            index = None;
            continue;
        }
        let current = index.unwrap() % tracks.len();
        match play(&tracks[current], &tools, &root, &args, &mut settings) {
            Ok(Action::Exit) => break,
            Ok(Action::Menu) => index = None,
            Ok(Action::Previous) => index = Some((current + tracks.len() - 1) % tracks.len()),
            Ok(Action::Ended) if current + 1 == tracks.len() => index = None,
            Ok(_) => index = Some((current + 1) % tracks.len()),
            Err(error) => {
                menu_error = Some(format!("{error:#}"));
                index = None;
            }
        }
    }
    Ok(())
}

fn main() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = execute!(io::stdout(), cursor::Show, LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
        previous(info);
    }));
    if let Err(error) = run() {
        eprintln!("Ошибка: {error:#}");
        std::process::exit(1);
    }
}
