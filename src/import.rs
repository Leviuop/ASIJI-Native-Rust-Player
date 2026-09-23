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

pub fn copy(source: &Path, folder: &Path) -> Result<bool> {
    ensure!(
        crate::media::is_media(source),
        "Неподдерживаемый формат: {}",
        source.display()
    );
    ensure!(source.is_file(), "Файл не найден: {}", source.display());
    let target = folder.join(source.file_name().context("Нет имени файла")?);
    if target.exists() {
        if source.canonicalize()? == target.canonicalize()? {
            return Ok(false);
        }
        bail!(
            "Уже существует {}, исходник оставлен на месте",
            target.display()
        );
    }
    let mut input = fs::File::open(source)?;
    let mut temporary = tempfile::NamedTempFile::new_in(folder)?;
    io::copy(&mut input, &mut temporary)?;
    temporary.as_file().sync_all()?;
    temporary
        .persist_noclobber(&target)
        .map_err(|e| e.error)
        .with_context(|| format!("Не удалось добавить {}", target.display()))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
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
