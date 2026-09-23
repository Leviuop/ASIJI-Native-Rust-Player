use crate::{Args, Decoder, Mode};
use anyhow::{Context, Result, bail};
use clap::{ArgMatches, ValueEnum, parser::ValueSource};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

const EXAMPLE: &str = include_str!("../config.example.toml");

#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Config {
    bindings: Option<std::collections::BTreeMap<String, Vec<String>>>,
    volume: Option<u8>,
    muted: Option<bool>,
    repeat: Option<bool>,
    fps: Option<u32>,
    width: Option<usize>,
    mode: Option<String>,
    color: Option<bool>,
    hwaccel: Option<String>,
    media: Option<PathBuf>,
}

fn location(
    windows: bool,
    appdata: Option<PathBuf>,
    xdg: Option<PathBuf>,
    home: Option<PathBuf>,
) -> Option<PathBuf> {
    let base = if windows {
        appdata.filter(|p| p.is_absolute())
    } else {
        xdg.filter(|p| p.is_absolute())
            .or_else(|| home.filter(|p| p.is_absolute()).map(|p| p.join(".config")))
    };
    base.map(|p| p.join("asiji").join("config.toml"))
}

fn default_path() -> Option<PathBuf> {
    location(
        cfg!(windows),
        std::env::var_os("APPDATA").map(PathBuf::from),
        std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from),
        std::env::var_os("HOME").map(PathBuf::from),
    )
}

pub fn initialize(explicit: Option<&Path>) -> Result<PathBuf> {
    let path = explicit
        .map(Path::to_owned)
        .or_else(default_path)
        .context("Не найден каталог настроек; укажите --config ПУТЬ --init-config")?;
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .with_context(|| {
            format!(
                "Не удалось создать {}; существующий конфиг не перезаписывается",
                path.display()
            )
        })?;
    file.write_all(EXAMPLE.as_bytes())?;
    Ok(path)
}

pub fn load(args: &mut Args, matches: &ArgMatches) -> Result<()> {
    if args.no_config {
        return Ok(());
    }
    let Some(path) = args.config.clone().or_else(default_path) else {
        return Ok(());
    };
    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound && args.config.is_none() => {
            return Ok(());
        }
        Err(error) => {
            return Err(error)
                .with_context(|| format!("Не удалось прочитать конфиг {}", path.display()));
        }
    };
    apply(args, matches, &path, &contents)
        .with_context(|| format!("Ошибка в конфиге {}", path.display()))
}

fn apply(args: &mut Args, matches: &ArgMatches, path: &Path, contents: &str) -> Result<()> {
    let config: Config = toml::from_str(contents.trim_start_matches('\u{feff}'))?;
    if let Some(bindings) = config.bindings {
        args.bindings = crate::bindings::Bindings::new(bindings)?;
    }
    anyhow::ensure!(
        config.volume.is_none_or(|v| v <= 100),
        "volume должен быть 0..100"
    );
    anyhow::ensure!(
        config.fps.is_none_or(|v| (10..=60).contains(&v)),
        "fps должен быть 10..60"
    );
    anyhow::ensure!(
        config
            .width
            .is_none_or(|v| v == 0 || (40..=1000).contains(&v)),
        "width должен быть 0 или 40..1000"
    );
    let mode = config
        .mode
        .map(|s| Mode::from_str(&s, false).map_err(anyhow::Error::msg))
        .transpose()?;
    let decoder = config
        .hwaccel
        .map(|s| Decoder::from_str(&s, false).map_err(anyhow::Error::msg))
        .transpose()?;
    if config
        .media
        .as_ref()
        .is_some_and(|p| p.as_os_str().is_empty())
    {
        bail!("media не может быть пустым");
    }
    let from_cli = |name| matches.value_source(name) == Some(ValueSource::CommandLine);
    macro_rules! merge {
        ($field:ident, $value:expr) => {
            if !from_cli(stringify!($field)) {
                if let Some(value) = $value {
                    args.$field = value;
                }
            }
        };
    }
    merge!(volume, config.volume);
    merge!(muted, config.muted);
    merge!(repeat, config.repeat);
    merge!(fps, config.fps);
    merge!(width, config.width);
    merge!(mode, mode);
    merge!(hwaccel, decoder);
    if !from_cli("mono") && !from_cli("color") {
        if let Some(color) = config.color {
            args.mono = !color;
        }
    }
    if !from_cli("media") {
        if let Some(media) = config.media {
            args.media = Some(if media.is_absolute() {
                media
            } else {
                std::path::absolute(path)?
                    .parent()
                    .context("Каталог конфига")?
                    .join(media)
            });
        }
    }
    Ok(())
}

pub fn effective(args: &Args) -> Result<String> {
    let config = Config {
        bindings: Some(args.bindings.names.clone()),
        volume: Some(args.volume),
        muted: Some(args.muted),
        repeat: Some(args.repeat),
        fps: Some(args.fps),
        width: Some(args.width),
        mode: Some(args.mode.to_possible_value().unwrap().get_name().into()),
        color: Some(!args.mono),
        hwaccel: Some(args.hwaccel.to_possible_value().unwrap().get_name().into()),
        media: Some(
            args.media
                .clone()
                .unwrap_or(crate::project_root()?.join("media")),
        ),
    };
    Ok(toml::to_string_pretty(&config)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{CommandFactory, FromArgMatches};
    fn parsed(cli: &[&str]) -> (Args, ArgMatches) {
        let matches = Args::command().try_get_matches_from(cli).unwrap();
        (Args::from_arg_matches(&matches).unwrap(), matches)
    }
    #[test]
    fn cli_overrides_config_even_with_default_values() {
        let (mut args, matches) = parsed(&[
            "asiji", "--fps", "30", "--volume", "10", "--color", "--repeat", "false",
        ]);
        apply(
            &mut args,
            &matches,
            Path::new("config.toml"),
            "fps=60\nvolume=80\ncolor=false\nrepeat=true\nmode='blocks'",
        )
        .unwrap();
        assert_eq!(
            (args.fps, args.volume, args.mono, args.repeat),
            (30, 10, false, false)
        );
        assert_eq!(args.mode, Mode::Blocks);
    }
    #[test]
    fn validates_typos_ranges_and_enum_values() {
        for content in [
            "volum=10",
            "volume=101",
            "fps=0",
            "width=1",
            "mode='pixel'",
            "hwaccel='gpu'",
            "media=''",
            "color='yes'",
            "volume=1\nvolume=2",
        ] {
            let (mut args, matches) = parsed(&["asiji"]);
            assert!(
                apply(&mut args, &matches, Path::new("config.toml"), content).is_err(),
                "{content}"
            );
        }
    }
    #[test]
    fn relative_media_uses_config_directory() {
        let dir = tempfile::tempdir().unwrap();
        let (mut args, matches) = parsed(&["asiji"]);
        apply(
            &mut args,
            &matches,
            &dir.path().join("config.toml"),
            "media='music'",
        )
        .unwrap();
        assert_eq!(args.media.unwrap(), dir.path().join("music"));
    }
    #[test]
    fn creates_example_without_overwriting() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings/config.toml");
        initialize(Some(&path)).unwrap();
        let (mut args, matches) = parsed(&["asiji"]);
        apply(
            &mut args,
            &matches,
            &path,
            &fs::read_to_string(&path).unwrap(),
        )
        .unwrap();
        assert_eq!(args.volume, 10);
        assert!(initialize(Some(&path)).is_err());
        assert_eq!(fs::read_to_string(path).unwrap(), EXAMPLE);
    }
    #[test]
    fn explicit_missing_file_errors_and_no_config_skips_it() {
        let dir = tempfile::tempdir().unwrap();
        let (mut args, matches) = parsed(&["asiji"]);
        args.config = Some(dir.path().join("missing.toml"));
        assert!(load(&mut args, &matches).is_err());
        args.no_config = true;
        load(&mut args, &matches).unwrap();
    }
    #[test]
    fn xdg_and_platform_paths() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().to_path_buf();
        assert_eq!(
            location(false, None, Some(base.clone()), None),
            Some(base.join("asiji/config.toml"))
        );
        assert_eq!(
            location(false, None, Some("relative".into()), Some(base.clone())),
            Some(base.join(".config/asiji/config.toml"))
        );
        assert_eq!(
            location(true, Some(base.clone()), None, None),
            Some(base.join("asiji/config.toml"))
        );
    }
}
