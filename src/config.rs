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
    hwaccel_device: Option<String>,
    hwaccel_fallback: Option<bool>,
    media: Option<PathBuf>,
    cache_dir: Option<PathBuf>,
    cache_max_mb: Option<u64>,
    cache_max_days: Option<u64>,
    import_mode: Option<String>,
    import_conflict: Option<String>,
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
    anyhow::ensure!(
        config
            .cache_dir
            .as_ref()
            .is_none_or(|p| !p.as_os_str().is_empty()),
        "cache_dir не может быть пустым"
    );
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
    merge!(
        import_mode,
        config
            .import_mode
            .map(|s| crate::import::ImportMode::from_str(&s, false).map_err(anyhow::Error::msg))
            .transpose()?
    );
    merge!(
        import_conflict,
        config
            .import_conflict
            .map(|s| crate::import::Conflict::from_str(&s, false).map_err(anyhow::Error::msg))
            .transpose()?
    );
    merge!(cache_max_mb, config.cache_max_mb);
    merge!(cache_max_days, config.cache_max_days);
    if !from_cli("cache_dir") {
        args.cache_dir = config.cache_dir.map(|p| {
            if p.is_absolute() {
                p
            } else {
                path.parent().unwrap_or(Path::new(".")).join(p)
            }
        });
    }
    merge!(volume, config.volume);
    merge!(muted, config.muted);
    merge!(repeat, config.repeat);
    merge!(fps, config.fps);
    merge!(width, config.width);
    merge!(mode, mode);
    merge!(hwaccel, decoder);
    merge!(hwaccel_fallback, config.hwaccel_fallback);
    if let Some(device) = &config.hwaccel_device {
        anyhow::ensure!(
            !device.trim().is_empty(),
            "hwaccel_device не может быть пустым"
        );
    }
    if !from_cli("hwaccel_device") {
        args.hwaccel_device = config.hwaccel_device;
    }
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
        import_mode: Some(
            args.import_mode
                .to_possible_value()
                .unwrap()
                .get_name()
                .into(),
        ),
        import_conflict: Some(
            args.import_conflict
                .to_possible_value()
                .unwrap()
                .get_name()
                .into(),
        ),
        cache_dir: args
            .cache_dir
            .as_ref()
            .map(std::path::absolute)
            .transpose()?,
        cache_max_mb: Some(args.cache_max_mb),
        cache_max_days: Some(args.cache_max_days),
        volume: Some(args.volume),
        muted: Some(args.muted),
        repeat: Some(args.repeat),
        fps: Some(args.fps),
        width: Some(args.width),
        mode: Some(args.mode.to_possible_value().unwrap().get_name().into()),
        color: Some(!args.mono),
        hwaccel: Some(args.hwaccel.to_possible_value().unwrap().get_name().into()),
        hwaccel_device: args.hwaccel_device.clone(),
        hwaccel_fallback: Some(args.hwaccel_fallback),
        media: Some(std::path::absolute(
            args.media
                .clone()
                .unwrap_or(crate::project_root()?.join("media")),
        )?),
    };
    Ok(toml::to_string_pretty(&config)?)
}

pub fn edit(args: &mut Args, key: &str, value: &str) -> Result<()> {
    use clap::CommandFactory;
    let mut config: toml::Table = toml::from_str(&effective(args)?)?;
    if let Some(action) = key.strip_prefix("bind ") {
        let bindings = config
            .get_mut("bindings")
            .and_then(toml::Value::as_table_mut)
            .context("Бинды")?;
        bindings.insert(
            action.into(),
            toml::Value::Array(
                value
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(|s| toml::Value::String(s.into()))
                    .collect(),
            ),
        );
    } else {
        let parsed = match key {
            "volume" | "fps" | "width" | "cache_max_mb" | "cache_max_days" => {
                toml::Value::Integer(value.parse()?)
            }
            "color" | "muted" | "repeat" | "hwaccel_fallback" => {
                toml::Value::Boolean(value.parse()?)
            }
            "mode" | "hwaccel" | "hwaccel_device" | "media" | "cache_dir" | "import_mode"
            | "import_conflict" => toml::Value::String(value.into()),
            _ => bail!("Неизвестная настройка: {key}"),
        };
        if key == "hwaccel_device" && value.is_empty() {
            config.remove(key);
        } else {
            config.insert(key.into(), parsed);
        }
    }
    let matches = Args::command().try_get_matches_from(["asiji"])?;
    let mut candidate = args.clone();
    apply(
        &mut candidate,
        &matches,
        &std::env::current_dir()?.join("config.toml"),
        &toml::to_string(&config)?,
    )?;
    *args = candidate;
    Ok(())
}

pub fn save(args: &Args) -> Result<PathBuf> {
    let path = std::path::absolute(
        args.config
            .clone()
            .or_else(default_path)
            .context("Укажите --config ПУТЬ")?,
    )?;
    let parent = path.parent().context("Каталог конфига")?;
    fs::create_dir_all(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(effective(args)?.as_bytes())?;
    temporary.as_file().sync_all()?;
    if path.exists() {
        fs::copy(&path, path.with_extension("toml.bak"))?;
    }
    temporary.persist(&path).map_err(|e| e.error)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{CommandFactory, FromArgMatches};
    #[test]
    fn settings_validate_and_save_with_backup() {
        let dir = tempfile::tempdir().unwrap();
        let (mut args, _) = parsed(&["asiji"]);
        args.config = Some(dir.path().join("config.toml"));
        edit(&mut args, "volume", "37").unwrap();
        assert!(edit(&mut args, "volume", "101").is_err());
        assert_eq!(args.volume, 37);
        edit(&mut args, "bind pause", "k").unwrap();
        assert!(edit(&mut args, "bind pause", "q").is_err());
        edit(&mut args, "import_mode", "move").unwrap();
        save(&args).unwrap();
        let original = fs::read(args.config.as_ref().unwrap()).unwrap();
        edit(&mut args, "volume", "10").unwrap();
        save(&args).unwrap();
        assert_eq!(
            fs::read(dir.path().join("config.toml.bak")).unwrap(),
            original
        );
        let (mut loaded, matches) = parsed(&["asiji"]);
        loaded.config = args.config.clone();
        load(&mut loaded, &matches).unwrap();
        assert_eq!(loaded.volume, 10);
        assert_eq!(loaded.import_mode, crate::import::ImportMode::Move);
        assert_eq!(loaded.bindings.hint("pause"), "k");
    }
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
    fn gpu_config_and_cli_precedence() {
        let (mut args, matches) = parsed(&[
            "asiji",
            "--hwaccel-device",
            "1",
            "--hwaccel-fallback",
            "true",
        ]);
        apply(
            &mut args,
            &matches,
            Path::new("config.toml"),
            "hwaccel='vaapi'\nhwaccel_device='/dev/dri/renderD129'\nhwaccel_fallback=false",
        )
        .unwrap();
        assert_eq!(args.hwaccel, Decoder::Vaapi);
        assert_eq!(args.hwaccel_device.as_deref(), Some("1"));
        assert!(args.hwaccel_fallback);
        assert!(effective(&args).unwrap().contains("hwaccel_device = \"1\""));
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
