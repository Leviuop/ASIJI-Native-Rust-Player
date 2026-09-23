use anyhow::{Context, Result, bail};
use clap::ValueEnum;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread::{self, JoinHandle},
    time::{Duration, UNIX_EPOCH},
};
use unicode_normalization::UnicodeNormalization;

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Decoder {
    Auto,
    Cpu,
    Cuda,
    D3d11va,
}

impl Decoder {
    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "AUTO",
            Self::Cpu => "CPU",
            Self::Cuda => "CUDA",
            Self::D3d11va => "D3D11VA",
        }
    }
}

const AUDIO: &[&str] = &[
    "flac", "wav", "mp3", "m4a", "ogg", "opus", "aac", "wma", "aiff",
];
const VIDEO: &[&str] = &["mp4", "mkv", "webm", "mov", "avi", "m4v", "mpeg", "mpg"];

#[derive(Debug)]
pub struct Track {
    pub title: String,
    pub audio: PathBuf,
    pub video: Option<PathBuf>,
}

pub fn safe_text(s: &str) -> String {
    s.chars()
        .filter(|c| {
            !c.is_control() && !matches!(*c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        })
        .collect()
}

fn extension(path: &Path) -> String {
    path.extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase()
}

pub fn discover(folder: &Path) -> Result<Vec<Track>> {
    let mut groups: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
    for item in fs::read_dir(folder).with_context(|| format!("Медиатека: {}", folder.display()))?
    {
        let path = item?.path();
        let ext = extension(&path);
        if path.is_file() && (AUDIO.contains(&ext.as_str()) || VIDEO.contains(&ext.as_str())) {
            let key: String = path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .nfc()
                .collect::<String>()
                .to_lowercase();
            groups.entry(key).or_default().push(path);
        }
    }
    let mut tracks = Vec::new();
    for mut paths in groups.into_values() {
        paths.sort();
        let choose = |extensions: &[&str]| -> Option<PathBuf> {
            extensions
                .iter()
                .find_map(|ext| paths.iter().find(|p| extension(p) == *ext).cloned())
        };
        let video = choose(VIDEO);
        if let Some(audio) = choose(AUDIO).or_else(|| video.clone()) {
            tracks.push(Track {
                title: audio
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
                audio,
                video,
            });
        }
    }
    Ok(tracks)
}

pub fn command(program: &Path) -> Command {
    let mut command = Command::new(program);
    command.stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
}

#[derive(Clone)]
pub struct Tools {
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
    pub decoder: Decoder,
}

impl Tools {
    pub fn find(root: &Path) -> Self {
        let find = |name: &str| {
            if let Some(path) = std::env::var_os(format!("ASIJI_{}", name.to_uppercase())) {
                return PathBuf::from(path);
            }
            let filename = if cfg!(windows) {
                format!("{name}.exe")
            } else {
                name.to_owned()
            };
            let local = root.join("bin").join(&filename);
            if local.is_file() {
                local
            } else {
                PathBuf::from(filename)
            }
        };
        Self {
            ffmpeg: find("ffmpeg"),
            ffprobe: find("ffprobe"),
            decoder: Decoder::Cpu,
        }
    }

    pub fn check(&self) -> Result<()> {
        for path in [&self.ffmpeg, &self.ffprobe] {
            let output = command(path).arg("-version").output().with_context(|| {
                format!(
                    "Не найден {}. Установите FFmpeg/FFprobe или положите их в bin.",
                    path.display()
                )
            })?;
            if !output.status.success() {
                bail!("Не удалось запустить {}", path.display());
            }
        }
        Ok(())
    }

    pub fn prepare_audio(&self, source: &Path, cache: &Path) -> Result<PathBuf> {
        fs::create_dir_all(cache)?;
        let meta = source.metadata()?;
        let mut hasher = Sha256::new();
        hasher.update(source.canonicalize()?.to_string_lossy().as_bytes());
        hasher.update(meta.len().to_le_bytes());
        hasher.update(
            meta.modified()?
                .duration_since(UNIX_EPOCH)?
                .as_nanos()
                .to_le_bytes(),
        );
        let target = cache.join(format!("rust-pcm-{:x}.wav", hasher.finalize()));
        if target.is_file() {
            return Ok(target);
        }
        println!("Подготовка звука при первом открытии…");
        let temporary = tempfile::Builder::new().suffix(".wav").tempfile_in(cache)?;
        let output = command(&self.ffmpeg)
            .args(["-nostdin", "-hide_banner", "-loglevel", "error", "-y", "-i"])
            .arg(source)
            .args([
                "-map",
                "0:a:0",
                "-vn",
                "-ac",
                "2",
                "-ar",
                "48000",
                "-c:a",
                "pcm_s16le",
            ])
            .arg(temporary.path())
            .output()?;
        if !output.status.success() {
            bail!(
                "Не удалось прочитать звук {}: {}",
                source.display(),
                safe_text(&String::from_utf8_lossy(&output.stderr))
            );
        }
        temporary.persist(&target).map_err(|e| e.error)?;
        Ok(target)
    }

    pub fn probe(&self, source: &Path) -> Result<VideoInfo> {
        let output = command(&self.ffprobe)
            .args([
                "-v",
                "error",
                "-select_streams",
                "v:0",
                "-show_streams",
                "-show_format",
                "-of",
                "json",
            ])
            .arg(source)
            .output()?;
        if !output.status.success() {
            bail!(
                "Не удалось прочитать видео {}: {}",
                source.display(),
                safe_text(&String::from_utf8_lossy(&output.stderr))
            );
        }
        let json: Value = serde_json::from_slice(&output.stdout)?;
        let stream = &json["streams"][0];
        let mut width = stream["width"].as_u64().unwrap_or(0) as usize;
        let mut height = stream["height"].as_u64().unwrap_or(0) as usize;
        if let Some((a, b)) = stream["sample_aspect_ratio"]
            .as_str()
            .and_then(|s| s.split_once(':'))
        {
            if let (Ok(a), Ok(b)) = (a.parse::<f64>(), b.parse::<f64>()) {
                if a > 0.0 && b > 0.0 {
                    width = (width as f64 * a / b).round() as usize;
                }
            }
        }
        if let Some(data) = stream["side_data_list"].as_array() {
            if data
                .iter()
                .any(|d| d["rotation"].as_i64().unwrap_or(0).rem_euclid(180) == 90)
            {
                std::mem::swap(&mut width, &mut height);
            }
        }
        let duration = stream["duration"]
            .as_str()
            .or_else(|| json["format"]["duration"].as_str())
            .unwrap_or("0")
            .parse::<f64>()?;
        if width == 0 || height == 0 || !duration.is_finite() || duration <= 0.0 {
            bail!(
                "Некорректные размеры или длительность видео: {}",
                source.display()
            );
        }
        Ok(VideoInfo {
            width,
            height,
            duration,
            fps: stream["avg_frame_rate"]
                .as_str()
                .and_then(|s| s.split_once('/'))
                .and_then(|(n, d)| Some(n.parse::<f64>().ok()? / d.parse::<f64>().ok()?))
                .filter(|f| f.is_finite() && *f > 0.0)
                .unwrap_or(30.0),
        })
    }

    pub fn select_decoder(
        &self,
        path: &Path,
        info: VideoInfo,
        size: (usize, usize),
        fps: u32,
        requested: Decoder,
    ) -> Result<Self> {
        let mut selected = self.clone();
        if requested != Decoder::Auto {
            selected.decoder = requested;
            return Ok(selected);
        }
        println!("Подбор декодера CPU/GPU для этого видео…");
        let mut best = f64::INFINITY;
        let candidates = if cfg!(windows) {
            vec![Decoder::Cpu, Decoder::Cuda, Decoder::D3d11va]
        } else {
            vec![Decoder::Cpu, Decoder::Cuda]
        };
        let mut errors = Vec::new();
        for decoder in candidates {
            let mut tools = self.clone();
            tools.decoder = decoder;
            let trial = (|| -> Result<f64> {
                let mut video = Video::open(&tools, path, info, size, fps, 0.0, false)?;
                for _ in 0..8 {
                    video.first()?;
                }
                let start = std::time::Instant::now();
                for _ in 0..64 {
                    video.first()?;
                }
                Ok(start.elapsed().as_secs_f64() / 64.0)
            })();
            match trial {
                Ok(seconds) => {
                    println!("  {}: {:.3} мс/кадр", decoder.label(), seconds * 1000.0);
                    // Require a meaningful win over the simpler CPU path.
                    if seconds < best * 0.9 {
                        best = seconds;
                        selected.decoder = decoder;
                    }
                }
                Err(error) => {
                    println!("  {}: недоступен", decoder.label());
                    errors.push(format!("{}: {error}", decoder.label()));
                }
            }
        }
        if !best.is_finite() {
            bail!("Не удалось подобрать декодер: {}", errors.join("; "));
        }
        println!("Выбран {}", selected.decoder.label());
        Ok(selected)
    }
}

#[derive(Clone, Copy)]
pub struct VideoInfo {
    pub width: usize,
    pub height: usize,
    pub duration: f64,
    pub fps: f64,
}

pub struct Frame {
    pub time: f64,
    pub rgb: Vec<u8>,
}

pub struct Video {
    child: Child,
    receiver: Option<Receiver<std::io::Result<Frame>>>,
    worker: Option<JoinHandle<()>>,
    error_file: tempfile::NamedTempFile,
    pending: Option<Frame>,
    pub current: Option<Frame>,
}

impl Video {
    pub fn open(
        tools: &Tools,
        path: &Path,
        info: VideoInfo,
        size: (usize, usize),
        fps: u32,
        start: f64,
        spectrum: bool,
    ) -> Result<Self> {
        let (width, pixels_high) = size;
        let error_file = tempfile::NamedTempFile::new()?;
        let mut cmd = command(&tools.ffmpeg);
        if !spectrum {
            match tools.decoder {
                Decoder::Cuda => {
                    cmd.args(["-hwaccel", "cuda"]);
                }
                Decoder::D3d11va => {
                    cmd.args(["-hwaccel", "d3d11va"]);
                }
                _ => {}
            }
        }
        cmd.args([
            "-nostdin",
            "-hide_banner",
            "-loglevel",
            "error",
            "-threads",
            "2",
            "-filter_threads",
            "1",
            "-filter_complex_threads",
            "1",
            "-stream_loop",
            "-1",
            "-ss",
        ])
        .arg(format!("{:.6}", start.rem_euclid(info.duration)))
        .arg("-i")
        .arg(path);
        if spectrum {
            cmd.arg("-filter_complex").arg(format!("[0:a:0]showspectrum=s={width}x{pixels_high}:mode=combined:color=intensity:slide=replace:scale=log:fps={fps},format=rgb24[v]"))
                .args(["-map", "[v]"]);
        } else {
            cmd.args(["-map", "0:v:0", "-vf"]).arg(format!(
                "fps={fps},scale={width}:{pixels_high}:flags=area,setsar=1"
            ));
        }
        let mut child = cmd
            .args(["-an", "-sn", "-dn"])
            .args(["-pix_fmt", "rgb24", "-f", "rawvideo", "pipe:1"])
            .stdout(Stdio::piped())
            .stderr(error_file.reopen()?)
            .spawn()?;
        let mut stdout = child.stdout.take().context("FFmpeg stdout")?;
        // Backpressure keeps decoded frames bounded even while playback is paused.
        let (sender, receiver) = mpsc::sync_channel(2);
        let worker = thread::spawn(move || {
            for index in 0_u64.. {
                let mut rgb = vec![0; width * pixels_high * 3];
                match stdout.read_exact(&mut rgb) {
                    Ok(()) => {
                        if sender
                            .send(Ok(Frame {
                                time: start + index as f64 / fps as f64,
                                rgb,
                            }))
                            .is_err()
                        {
                            break;
                        }
                    }
                    Err(error) => {
                        let _ = sender.send(Err(error));
                        break;
                    }
                }
            }
        });
        Ok(Self {
            child,
            receiver: Some(receiver),
            worker: Some(worker),
            error_file,
            pending: None,
            current: None,
        })
    }

    fn decode_error(&self, error: impl std::fmt::Display) -> anyhow::Error {
        let detail = fs::read_to_string(self.error_file.path()).unwrap_or_default();
        anyhow::anyhow!("Декодирование видео: {error}. {}", safe_text(&detail))
    }

    pub fn first(&mut self) -> Result<()> {
        let frame = self
            .receiver
            .as_ref()
            .unwrap()
            .recv_timeout(Duration::from_secs(15))
            .map_err(|e| self.decode_error(e))?;
        self.current = Some(frame.map_err(|e| self.decode_error(e))?);
        Ok(())
    }

    pub fn advance(&mut self, position: f64) -> Result<bool> {
        let mut changed = false;
        loop {
            if self.pending.is_none() {
                match self.receiver.as_ref().unwrap().try_recv() {
                    Ok(Ok(frame)) => self.pending = Some(frame),
                    Ok(Err(error)) => return Err(self.decode_error(error)),
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => bail!("Декодер видео остановился"),
                }
            }
            if self.pending.as_ref().unwrap().time > position {
                break;
            }
            self.current = self.pending.take();
            changed = true;
        }
        Ok(changed)
    }
}

impl Drop for Video {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        self.receiver.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pairs_and_standalone_files() -> Result<()> {
        let folder = tempfile::tempdir()?;
        for name in [
            "Песня.MP3",
            "Песня.mp4",
            "Solo.flac",
            "Clip.webm",
            "notes.txt",
        ] {
            fs::write(folder.path().join(name), [])?;
        }
        let tracks = discover(folder.path())?;
        assert_eq!(tracks.len(), 3);
        let song = tracks.iter().find(|t| t.title == "Песня").unwrap();
        assert_eq!(song.audio.extension().unwrap(), "MP3");
        assert!(song.video.is_some());
        assert!(
            tracks
                .iter()
                .find(|t| t.title == "Solo")
                .unwrap()
                .video
                .is_none()
        );
        Ok(())
    }
    #[test]
    fn strips_terminal_and_bidi_controls() {
        assert_eq!(safe_text("a\x1b\n\tb\u{202e}"), "ab");
    }
}
