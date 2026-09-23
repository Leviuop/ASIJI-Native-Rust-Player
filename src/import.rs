use anyhow::{Context, Result, bail, ensure};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

// Terminal drops paste paths, not shell commands. Never execute or expand them.
pub fn paths(input: &str, windows: bool) -> Result<Vec<PathBuf>> {
    let input = input.trim();
    if Path::new(input).is_file() {
        return Ok(vec![PathBuf::from(input)]);
    }
    let mut chars = input.chars().peekable();
    let mut quote = None;
    let mut token = String::new();
    let mut tokens = Vec::new();
    while let Some(c) = chars.next() {
        match c {
            '\'' | '"' if quote == Some(c) => quote = None,
            '\'' | '"' if quote.is_none() => quote = Some(c),
            '\\' if !windows && quote != Some('\'') => {
                let next = chars
                    .peek()
                    .copied()
                    .context("Незавершённый путь после обратного слеша")?;
                if quote.is_none() || matches!(next, '"' | '\\' | '$' | '`') {
                    token.push(chars.next().unwrap());
                } else {
                    token.push(c);
                }
            }
            c if c.is_whitespace() && quote.is_none() => {
                if !token.is_empty() {
                    tokens.push(std::mem::take(&mut token));
                }
            }
            c => token.push(c),
        }
    }
    ensure!(quote.is_none(), "Незакрытая кавычка в пути");
    if !token.is_empty() {
        tokens.push(token);
    }
    ensure!(
        !tokens.is_empty(),
        "Перетащите файл или вставьте его полный путь"
    );
    tokens
        .into_iter()
        .map(|token| local_path(&token, windows))
        .collect()
}

fn local_path(token: &str, windows: bool) -> Result<PathBuf> {
    let Some(uri) = token.strip_prefix("file://") else {
        return Ok(PathBuf::from(token));
    };
    let path = if uri.starts_with('/') {
        uri
    } else {
        uri.strip_prefix("localhost")
            .filter(|p| p.starts_with('/'))
            .context("Поддерживаются только локальные file:// пути")?
    };
    let mut bytes = Vec::new();
    let mut source = path.bytes();
    while let Some(byte) = source.next() {
        if byte == b'%' {
            let a = source.next().and_then(|b| (b as char).to_digit(16));
            let b = source.next().and_then(|b| (b as char).to_digit(16));
            bytes.push(
                (a.context("Некорректный file:// путь")? * 16
                    + b.context("Некорректный file:// путь")?) as u8,
            );
        } else {
            bytes.push(byte);
        }
    }
    ensure!(!bytes.contains(&0), "Нулевой байт в пути");
    let decoded = String::from_utf8(bytes).context("Путь file:// должен содержать UTF-8")?;
    let decoded = if windows && decoded.as_bytes().get(2) == Some(&b':') {
        &decoded[1..]
    } else {
        &decoded
    };
    Ok(PathBuf::from(decoded))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum ImportMode {
    Copy,
    Move,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum Conflict {
    Skip,
    Rename,
}

#[cfg(test)]
pub fn copy(source: &Path, folder: &Path) -> Result<bool> {
    transfer(
        source,
        &folder.join(source.file_name().context("Нет имени файла")?),
        ImportMode::Copy,
        |_, _| Ok(true),
    )
}

fn transfer(
    source: &Path,
    target: &Path,
    mode: ImportMode,
    mut progress: impl FnMut(u64, u64) -> Result<bool>,
) -> Result<bool> {
    use std::io::{Read, Write};
    ensure!(
        crate::media::is_media(source),
        "Неподдерживаемый формат: {}",
        source.display()
    );
    ensure!(source.is_file(), "Файл не найден: {}", source.display());
    if target.exists() {
        if source.canonicalize()? == target.canonicalize()? {
            return Ok(false);
        }
        bail!(
            "Уже существует {}, исходник оставлен на месте",
            target.display()
        );
    }
    if mode == ImportMode::Move {
        ensure!(
            !fs::symlink_metadata(source)?.file_type().is_symlink(),
            "Симлинк можно скопировать, но нельзя переместить"
        );
    }
    let mut input = fs::File::open(source)?;
    let before = input.metadata()?;
    let mut temporary =
        tempfile::NamedTempFile::new_in(target.parent().context("Каталог назначения")?)?;
    let mut buffer = vec![0; 1024 * 1024];
    let mut copied = 0;
    loop {
        ensure!(
            progress(copied, before.len())?,
            "Импорт отменён; исходник сохранён"
        );
        let count = input.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        temporary.write_all(&buffer[..count])?;
        copied += count as u64;
    }
    temporary.as_file().sync_all()?;
    let after = fs::metadata(source)?;
    ensure!(
        after.len() == before.len() && after.modified()? == before.modified()?,
        "Исходник изменился во время копирования; повторите импорт после завершения записи"
    );
    temporary
        .persist_noclobber(target)
        .map_err(|e| e.error)
        .with_context(|| format!("Не удалось добавить {}", target.display()))?;
    drop(input);
    if mode == ImportMode::Move {
        fs::remove_file(source).with_context(|| {
            format!(
                "Копия готова в {}, но исходник удалить не удалось",
                target.display()
            )
        })?;
    }
    Ok(true)
}

pub fn batch(paths: Vec<PathBuf>, folder: &Path, mode: ImportMode, conflict: Conflict) -> String {
    use crossterm::{
        event::{self, Event, KeyCode, KeyModifiers},
        terminal,
    };
    use std::{
        collections::{HashMap, HashSet},
        io::{IsTerminal, Write},
        time::{Duration, Instant},
    };
    let mut stems: HashSet<String> = fs::read_dir(folder)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| {
            e.path()
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase()
        })
        .collect();
    let mut renamed: HashMap<PathBuf, String> = HashMap::new();
    let mut report = Vec::new();
    for source in paths {
        let mut cancelled = false;
        let result = (|| -> Result<bool> {
            let name = source.file_name().context("Нет имени файла")?;
            let mut target = folder.join(name);
            let already = source
                .canonicalize()
                .ok()
                .zip(target.canonicalize().ok())
                .is_some_and(|(a, b)| a == b);
            if conflict == Conflict::Rename && !already {
                let stem = source
                    .file_stem()
                    .context("Нет имени")?
                    .to_string_lossy()
                    .into_owned();
                let key = source.with_extension("");
                let selected = renamed.entry(key).or_insert_with(|| {
                    let mut candidate = stem.clone();
                    let mut index = 2;
                    while stems.contains(&candidate.to_lowercase()) {
                        candidate = format!("{stem} ({index})");
                        index += 1;
                    }
                    stems.insert(candidate.to_lowercase());
                    candidate
                });
                target = folder.join(format!(
                    "{}.{}",
                    selected,
                    source.extension().unwrap_or_default().to_string_lossy()
                ));
            }
            println!(
                "Добавление: {}",
                crate::media::safe_text(&source.to_string_lossy())
            );
            let interactive = io::stdin().is_terminal() && io::stdout().is_terminal();
            struct Raw(bool);
            impl Drop for Raw {
                fn drop(&mut self) {
                    if self.0 {
                        let _ = terminal::disable_raw_mode();
                        println!();
                    }
                }
            }
            if interactive {
                terminal::enable_raw_mode()?;
            }
            let _raw = Raw(interactive);
            let mut last = Instant::now() - Duration::from_secs(1);
            let result = transfer(&source, &target, mode, |done, total| {
                if interactive {
                    if last.elapsed() >= Duration::from_millis(100) || done == total {
                        print!(
                            "\r{} / {} МиБ ({:.0}%) — Esc отменить    ",
                            done / 1048576,
                            total / 1048576,
                            done as f64 * 100.0 / total.max(1) as f64
                        );
                        io::stdout().flush()?;
                        last = Instant::now();
                    }
                    while event::poll(Duration::ZERO)? {
                        if let Event::Key(key) = event::read()? {
                            if key.code == KeyCode::Esc
                                || (key.code == KeyCode::Char('c')
                                    && key.modifiers.contains(KeyModifiers::CONTROL))
                            {
                                cancelled = true;
                                return Ok(false);
                            }
                        }
                    }
                }
                Ok(true)
            });
            if matches!(result, Ok(true)) {
                report.push(format!(
                    "Добавлено: {}",
                    crate::media::safe_text(&target.to_string_lossy())
                ));
            }
            result
        })();
        match result {
            Ok(true) => (),
            Ok(false) => report.push(format!(
                "Уже в медиатеке: {}",
                crate::media::safe_text(&source.to_string_lossy())
            )),
            Err(error) => report.push(format!(
                "Не добавлено: {}",
                crate::media::safe_text(&format!("{error:#}"))
            )),
        }
        if cancelled {
            break;
        }
    }
    report.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancel_move_and_paired_rename() -> Result<()> {
        let source = tempfile::tempdir()?;
        let library = tempfile::tempdir()?;
        let audio = source.path().join("Song.mp3");
        let video = source.path().join("Song.mp4");
        fs::write(&audio, vec![7_u8; 2 * 1024 * 1024])?;
        fs::write(&video, b"video")?;
        assert!(
            transfer(
                &audio,
                &library.path().join("Song.mp3"),
                ImportMode::Move,
                |done, _| Ok(done == 0)
            )
            .is_err()
        );
        assert!(audio.exists());
        assert_eq!(fs::read_dir(library.path())?.count(), 0);
        fs::write(library.path().join("Song.mp3"), b"existing")?;
        let report = batch(
            vec![audio.clone(), video.clone()],
            library.path(),
            ImportMode::Move,
            Conflict::Rename,
        );
        assert!(!report.contains("Не добавлено"), "{report}");
        assert!(!audio.exists() && !video.exists());
        assert!(library.path().join("Song (2).mp3").exists());
        assert!(library.path().join("Song (2).mp4").exists());
        assert_eq!(fs::read(library.path().join("Song.mp3"))?, b"existing");
        Ok(())
    }
    #[test]
    fn terminal_path_formats() {
        assert_eq!(
            paths(r#""C:\My Music\Песня.mp4" 'D:\Audio\Song.mp3'"#, true).unwrap(),
            vec![
                PathBuf::from(r"C:\My Music\Песня.mp4"),
                PathBuf::from(r"D:\Audio\Song.mp3")
            ]
        );
        assert_eq!(
            paths(r#"/home/me/My\ Song.mp4 '/home/me/it'\''s.mp3'"#, false).unwrap(),
            vec![
                PathBuf::from("/home/me/My Song.mp4"),
                PathBuf::from("/home/me/it's.mp3")
            ]
        );
        assert_eq!(
            paths(
                "file:///home/me/My%20Song.mp4 file://localhost/home/a.mp3",
                false
            )
            .unwrap(),
            vec![
                PathBuf::from("/home/me/My Song.mp4"),
                PathBuf::from("/home/a.mp3")
            ]
        );
        assert_eq!(
            paths("file:///C:/Music/Track.mp4", true).unwrap(),
            vec![PathBuf::from("C:/Music/Track.mp4")]
        );
        for input in [
            "'unfinished",
            "file://server/music.mp3",
            "file:///bad%00.mp3",
            "file:///bad%XX.mp3",
        ] {
            assert!(paths(input, false).is_err(), "{input}");
        }
    }
    #[test]
    fn copies_pairs_without_overwriting_or_removing_sources() -> Result<()> {
        let source = tempfile::tempdir()?;
        let library = tempfile::tempdir()?;
        for name in ["Песня.mp3", "Песня.mp4"] {
            let file = source.path().join(name);
            fs::write(&file, name)?;
            assert!(copy(&file, library.path())?);
            assert_eq!(fs::read(&file)?, fs::read(library.path().join(name))?);
            assert!(copy(&file, library.path()).is_err());
            assert!(!copy(&library.path().join(name), library.path())?);
        }
        assert_eq!(crate::media::discover(library.path())?.len(), 1);
        assert_eq!(fs::read_dir(library.path())?.count(), 2);
        assert!(copy(source.path(), library.path()).is_err());
        assert!(copy(&source.path().join("missing.mp4"), library.path()).is_err());
        let script = source.path().join("script.sh");
        fs::write(&script, "exit 1")?;
        assert!(copy(&script, library.path()).is_err());
        Ok(())
    }
}
