use anyhow::{Context, Result};
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink, Source};
use std::{
    fs::File,
    path::{Path, PathBuf},
    time::Duration,
};

pub struct Audio {
    pub sink: Sink,
    pub duration: f64,
    path: PathBuf,
    _stream: OutputStream,
}

impl Audio {
    pub fn open(path: &Path, volume: f32) -> Result<Self> {
        let source = Decoder::try_from(File::open(path)?)?;
        let duration = source
            .total_duration()
            .context("Неизвестная длительность WAV")?
            .as_secs_f64();
        anyhow::ensure!(duration > 0.0, "Пустая аудиодорожка");
        let mut stream = OutputStreamBuilder::open_default_stream()
            .context("Не удалось открыть аудиоустройство")?;
        stream.log_on_drop(false);
        let sink = Sink::connect_new(stream.mixer());
        sink.pause();
        sink.set_volume(volume);
        sink.append(source);
        Ok(Self {
            sink,
            duration,
            path: path.to_owned(),
            _stream: stream,
        })
    }

    pub fn position(&self) -> f64 {
        self.sink.get_pos().as_secs_f64().min(self.duration)
    }

    pub fn seek(&self, seconds: f64) -> Result<()> {
        if self.sink.empty() {
            self.sink
                .append(Decoder::try_from(File::open(&self.path)?)?);
        }
        self.sink
            .try_seek(Duration::from_secs_f64(seconds.clamp(0.0, self.duration)))
            .map_err(|error| anyhow::anyhow!("Перемотка: {error}"))?;
        Ok(())
    }
}
