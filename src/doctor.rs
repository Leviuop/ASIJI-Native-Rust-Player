use crate::media::{Decoder, Tools, Video, command, safe_text};
use anyhow::{Context, Result};
use std::path::Path;

pub fn run(tools: &Tools, video: Option<&Path>) -> Result<()> {
    println!(
        "ASIJI {} / {}",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS
    );
    let mut healthy = true;
    for path in [&tools.ffmpeg, &tools.ffprobe] {
        match command(path).arg("-version").output() {
            Ok(output) if output.status.success() => println!(
                "{}: {}",
                path.display(),
                safe_text(
                    String::from_utf8_lossy(&output.stdout)
                        .lines()
                        .next()
                        .unwrap_or("OK")
                )
            ),
            Ok(output) => {
                healthy = false;
                println!(
                    "{}: ошибка {} — {}",
                    path.display(),
                    output.status,
                    safe_text(&String::from_utf8_lossy(&output.stderr))
                );
            }
            Err(error) => {
                healthy = false;
                println!("{}: {error}", path.display());
            }
        }
    }
    println!("GPU device: {}", tools.device.as_deref().unwrap_or("auto"));
    if cfg!(target_os = "linux") {
        if let Ok(entries) = std::fs::read_dir("/dev/dri") {
            for entry in entries
                .flatten()
                .filter(|e| e.file_name().to_string_lossy().starts_with("renderD"))
            {
                println!("VAAPI device: {}", entry.path().display());
            }
        }
        println!(
            "AMD/Intel: VAAPI + системный видеодрайвер; NVIDIA: драйвер NVIDIA.\nFFmpeg и VAAPI-драйверы устанавливаются пакетным менеджером дистрибутива."
        );
    } else if cfg!(windows) {
        println!(
            "AMD/Intel/NVIDIA: D3D11VA; NVIDIA также CUDA/NVDEC.\nFFmpeg: scripts/setup-ffmpeg.ps1. Драйверы: сайт производителя GPU или Windows Update."
        );
    }
    println!("CUDA Toolkit и ROCm для декодирования плееру не нужны.");
    anyhow::ensure!(
        healthy,
        "Установите FFmpeg и FFprobe, затем повторите --doctor"
    );
    let output = command(&tools.ffmpeg)
        .args(["-hide_banner", "-hwaccels"])
        .output()?;
    anyhow::ensure!(
        output.status.success(),
        "FFmpeg -hwaccels завершился с ошибкой"
    );
    println!(
        "Поддержка в сборке FFmpeg (не проверка драйвера/GPU):\n{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let Some(path) = video else {
        println!("Для реальной проверки: --doctor --doctor-video ПУТЬ_К_ВИДЕО");
        return Ok(());
    };
    let info = tools.probe(path)?;
    let candidates = if cfg!(windows) {
        [Decoder::Cpu, Decoder::Cuda, Decoder::D3d11va]
    } else {
        [Decoder::Cpu, Decoder::Cuda, Decoder::Vaapi]
    };
    let mut cpu_ok = false;
    for decoder in candidates {
        let mut trial = tools.clone();
        trial.decoder = decoder;
        let result = (|| -> Result<()> {
            let mut video = Video::open(&trial, path, info, (160, 90), 30, 0.0, false)?;
            for _ in 0..8 {
                video.first()?;
            }
            Ok(())
        })();
        match result {
            Ok(()) => {
                println!("{}: OK (8 кадров декодированы)", decoder.label());
                cpu_ok |= decoder == Decoder::Cpu;
            }
            Err(error) => println!("{}: FAIL — {error:#}", decoder.label()),
        }
    }
    cpu_ok
        .then_some(())
        .context("CPU-декодирование недоступно; проверьте файл и сборку FFmpeg")
}
