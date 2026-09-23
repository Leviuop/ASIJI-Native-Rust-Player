mod audio;
mod canvas;
#[cfg(test)]
mod integration_tests;
mod media;
mod profile;
mod render;
mod timing;

use anyhow::{Context, Result, bail};
use audio::Audio;
use clap::Parser;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
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

#[derive(Parser)]
#[command(version, about = "ASIJI — нативный ASCII/HD музыкальный плеер")]
struct Args {
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
    #[arg(long)]
    mono: bool,
    #[arg(long)]
    check: bool,
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
    tools: &'a Tools,
    path: &'a Path,
    info: VideoInfo,
    spectrum: bool,
    fps: u32,
}

impl Clip<'_> {
    fn open(&self, size: (usize, usize), mode: Mode, position: f64) -> Result<Video> {
        let pixels_high = size.1 * if mode == Mode::Blocks { 2 } else { 1 };
        let mut video = Video::open(
            self.tools,
            self.path,
            self.info,
            (size.0, pixels_high),
            self.fps,
            position,
            self.spectrum,
        )?;
        video.first()?;
        Ok(video)
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
        " Space: pause | Left/Right or A/D: seek 5s | Up/Down or +/-: volume",
        width,
    ));
    rows.push(render::clip(
        " H: HD/ASCII | N/P: track | M: mute | C: color | R: repeat | Q: menu | X: exit",
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
    let wav = tools.prepare_audio(&track.audio, &root.join(".cache"))?;
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
    let clip = Clip {
        tools: &selected_tools,
        path: track.video.as_deref().unwrap_or(&wav),
        info,
        spectrum: track.video.is_none(),
        fps: effective_fps,
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
                    let code = match key.code {
                        KeyCode::Char(c) => KeyCode::Char(c.to_ascii_lowercase()),
                        other => other,
                    };
                    if code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                        return Ok(Action::Exit);
                    }
                    match code {
                        KeyCode::Char('x') => return Ok(Action::Exit),
                        KeyCode::Char('q') | KeyCode::Esc => return Ok(Action::Menu),
                        KeyCode::Char('n') => return Ok(Action::Next),
                        KeyCode::Char('p') => return Ok(Action::Previous),
                        KeyCode::Char(' ') => {
                            if audio.sink.is_paused() {
                                audio.sink.play();
                            } else {
                                audio.sink.pause();
                            }
                        }
                        KeyCode::Left | KeyCode::Char('a') => {
                            restart = Some((audio.position() - 5.0).max(0.0))
                        }
                        KeyCode::Right | KeyCode::Char('d') => {
                            restart = Some((audio.position() + 5.0).min(audio.duration))
                        }
                        KeyCode::Up | KeyCode::Char('+' | '=') => {
                            settings.volume = (settings.volume + 0.05).min(1.0)
                        }
                        KeyCode::Down | KeyCode::Char('-') => {
                            settings.volume = (settings.volume - 0.05).max(0.0)
                        }
                        KeyCode::Char('m') => settings.muted = !settings.muted,
                        KeyCode::Char('c') => settings.color = !settings.color,
                        KeyCode::Char('r') => settings.repeat = !settings.repeat,
                        KeyCode::Char('h') => {
                            settings.mode = if settings.mode == Mode::Ascii {
                                Mode::Blocks
                            } else {
                                Mode::Ascii
                            };
                            restart = Some(audio.position());
                        }
                        _ => {}
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
        let changed = video.advance(audio.position())?;
        if let Some(frame) = &video.current {
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
            let rows = screen_rows(
                track,
                &audio,
                settings,
                size,
                dimensions,
                fps,
                selected_tools.decoder,
            );
            let output = screen.update(rows, size);
            let frame = video.current.as_ref().context("Нет видеокадра")?;
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
            let next = video
                .current
                .as_ref()
                .map_or(audio.position(), |frame| frame.time)
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
    for path in exe.ancestors().skip(1) {
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
    let tools = Tools::find(&project_root()?);
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
    let args = Args::parse();
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
    let folder = args.media.clone().unwrap_or_else(|| root.join("media"));
    fs::create_dir_all(&folder)?;
    let tools = Tools::find(&root);
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
        volume: 0.10,
        muted: false,
        color: !args.mono,
        repeat: false,
        mode: args.mode,
    };
    let mut index = args.play.map(|n| n.saturating_sub(1));
    let mut menu_error = None;
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
            print!("\nНомер трека / R — обновить / Q — выход: ");
            io::stdout().flush()?;
            let mut input = String::new();
            if io::stdin().read_line(&mut input)? == 0 {
                break;
            }
            if matches!(input.trim().to_lowercase().as_str(), "q" | "й") {
                break;
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
