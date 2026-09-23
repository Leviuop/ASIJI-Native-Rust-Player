use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

pub struct Profile {
    path: Option<PathBuf>,
    started: Instant,
    pub playback_started: Option<Instant>,
    pub decode_wait: Duration,
    pub render: Duration,
    pub write: Duration,
    pub bytes: u64,
    pub frames: u64,
    pub decoder: String,
    pub columns: usize,
    pub rows: usize,
    pub requested_fps: u32,
    pub effective_fps: u32,
    pub iterations: u64,
    pub wait: Duration,
    pub max_video_lag: f64,
}

impl Profile {
    pub fn new(path: Option<&Path>) -> Self {
        Self {
            path: path.map(Path::to_owned),
            started: Instant::now(),
            playback_started: None,
            decode_wait: Duration::ZERO,
            render: Duration::ZERO,
            write: Duration::ZERO,
            bytes: 0,
            frames: 0,
            decoder: String::new(),
            columns: 0,
            rows: 0,
            requested_fps: 0,
            effective_fps: 0,
            iterations: 0,
            wait: Duration::ZERO,
            max_video_lag: 0.0,
        }
    }
}

impl Drop for Profile {
    fn drop(&mut self) {
        if let Some(path) = &self.path {
            let report = serde_json::json!({
                "elapsed_seconds": self.started.elapsed().as_secs_f64(),
                "playback_seconds":self.playback_started.map(|start|start.elapsed().as_secs_f64()),
                "frames":self.frames, "bytes":self.bytes,
                "decoder":self.decoder,
                "columns":self.columns,"rows":self.rows,"requested_fps":self.requested_fps,"effective_fps":self.effective_fps,
                "iterations":self.iterations,"wait_ms":self.wait.as_secs_f64()*1000.0,"max_video_lag_seconds":self.max_video_lag,
                "decode_queue_ms":self.decode_wait.as_secs_f64()*1000.0,
                "render_ms":self.render.as_secs_f64()*1000.0,
                "terminal_write_ms":self.write.as_secs_f64()*1000.0,
                "note":"write measures time submitting ANSI to the terminal, not GPU scanout; decode queue time excludes FFmpeg worker execution"
            });
            if let Err(error) = std::fs::write(path, serde_json::to_string_pretty(&report).unwrap())
            {
                eprintln!("Не удалось сохранить профиль {}: {error}", path.display());
            }
        }
    }
}
