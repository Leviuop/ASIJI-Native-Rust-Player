use crate::{
    audio::Audio,
    media::{Tools, Video, discover},
    render::{self, Mode},
};
use anyhow::Result;
use std::{
    fs,
    path::Path,
    thread,
    time::{Duration, Instant},
};

fn fixture() -> Result<(tempfile::TempDir, Tools, std::path::PathBuf)> {
    let folder = tempfile::tempdir()?;
    let tools = Tools::find(Path::new(env!("CARGO_MANIFEST_DIR")));
    tools.check()?;
    let path = folder.path().join("Клип с пробелами.mp4");
    let output = crate::media::command(&tools.ffmpeg)
        .args([
            "-nostdin",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=320x180:rate=24",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=48000",
            "-t",
            "2",
            "-c:v",
            "libx264",
            "-c:a",
            "aac",
            "-y",
        ])
        .arg(&path)
        .output()?;
    anyhow::ensure!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok((folder, tools, path))
}

#[test]
#[ignore = "requires FFmpeg/FFprobe binaries"]
fn real_decode_seek_loop_and_cache() -> Result<()> {
    let (folder, tools, path) = fixture()?;
    assert_eq!(discover(folder.path())?.len(), 1);
    let info = tools.probe(&path)?;
    assert_eq!((info.width, info.height), (320, 180));
    let wav = tools.prepare_audio(&path, &folder.path().join("cache"))?;
    let modified = wav.metadata()?.modified()?;
    assert_eq!(
        tools.prepare_audio(&path, &folder.path().join("cache"))?,
        wav
    );
    assert_eq!(modified, wav.metadata()?.modified()?);
    let frame_at = |position| -> Result<Vec<u8>> {
        let mut video = Video::open(&tools, &path, info, (80, 44), 30, position, false)?;
        video.first()?;
        Ok(video.current.as_ref().unwrap().rgb.clone())
    };
    let first = frame_at(0.0)?;
    let later = frame_at(1.0)?;
    assert_ne!(first, later);
    assert_eq!(first, frame_at(info.duration)?);
    assert_eq!(render::lines(&later, 80, 22, Mode::Blocks, true).len(), 22);
    // The source loops past EOF and the bounded queue can be dropped while full.
    let mut video = Video::open(&tools, &path, info, (40, 20), 30, 1.9, false)?;
    video.first()?;
    let deadline = Instant::now() + Duration::from_secs(3);
    while video.current.as_ref().unwrap().time < 2.2 && Instant::now() < deadline {
        video.advance(2.2)?;
        thread::sleep(Duration::from_millis(5));
    }
    assert!(video.current.as_ref().unwrap().time >= 2.19);
    thread::sleep(Duration::from_millis(100));
    drop(video);
    let mut spectrum = Video::open(&tools, &wav, info, (80, 44), 30, 0.0, true)?;
    spectrum.first()?;
    assert_eq!(spectrum.current.as_ref().unwrap().rgb.len(), 80 * 44 * 3);
    drop(spectrum);
    let broken = folder.path().join("broken.mp3");
    fs::write(&broken, b"not audio")?;
    let invalid_cache = folder.path().join("invalid");
    assert!(tools.prepare_audio(&broken, &invalid_cache).is_err());
    assert_eq!(fs::read_dir(invalid_cache)?.count(), 0);
    Ok(())
}

fn wait_until(condition: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(4);
    while !condition() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(10));
    }
    assert!(condition(), "Audio device did not reach the expected state");
}

#[test]
#[ignore = "requires an output audio device and FFmpeg"]
fn real_audio_pause_seek_end_and_restart() -> Result<()> {
    let (folder, tools, path) = fixture()?;
    let wav = tools.prepare_audio(&path, &folder.path().join("cache"))?;
    let audio = Audio::open(&wav, 0.0)?;
    assert!(audio.sink.is_paused());
    assert_eq!(audio.sink.volume(), 0.0);
    audio.sink.play();
    wait_until(|| audio.position() > 0.15);
    audio.sink.pause();
    thread::sleep(Duration::from_millis(80));
    let paused = audio.position();
    thread::sleep(Duration::from_millis(150));
    assert!((audio.position() - paused).abs() < 0.01);
    audio.seek(1.0)?;
    assert!((audio.position() - 1.0).abs() < 0.03);
    audio.sink.play();
    wait_until(|| audio.position() > 1.1);
    audio.seek(audio.duration - 0.05)?;
    wait_until(|| audio.sink.empty());
    audio.sink.pause();
    audio.seek(0.0)?;
    assert!(!audio.sink.empty());
    audio.sink.play();
    wait_until(|| audio.position() > 0.1);
    Ok(())
}
