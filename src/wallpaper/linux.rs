use super::{Backend, frame, select_backend, stream::FrameServer};
use crate::{Args, render::Mode};
use anyhow::{Context, Result, bail};
use fs2::FileExt;
use std::{
    env, fs,
    io::{Read, Seek, Write},
    os::unix::{
        fs::{DirBuilderExt, MetadataExt, PermissionsExt},
        process::CommandExt,
    },
    path::PathBuf,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

const PLUGIN: &str = "io.github.Leviuop.asiji";
const GNOME_ID: &str = "asiji@leviuop.github.io";

fn backend(args: &Args) -> Result<Backend> {
    select_backend(
        args.wallpaper_backend,
        &env::var("XDG_CURRENT_DESKTOP").unwrap_or_default(),
        env::var_os("WAYLAND_DISPLAY").is_some(),
        env::var_os("DISPLAY").is_some(),
    )
}

fn program(names: &[&str]) -> Result<PathBuf> {
    for name in names {
        for directory in env::split_paths(&env::var_os("PATH").unwrap_or_default()) {
            let path = directory.join(name);
            if path.is_file() {
                return Ok(path);
            }
        }
    }
    bail!(
        "Не найдена программа {}. Установите её пакет для своего дистрибутива; см. docs/WALLPAPER.md",
        names.join(" / ")
    )
}

fn output(command: &mut Command) -> Result<String> {
    let mut file = tempfile::tempfile()?;
    let mut child = command
        .stdin(Stdio::null())
        .stdout(file.try_clone()?)
        .stderr(file.try_clone()?)
        .spawn()?;
    let deadline = Instant::now() + Duration::from_secs(5);
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            bail!("Команда рабочего стола не ответила за 5 секунд");
        }
        thread::sleep(Duration::from_millis(20));
    };
    file.rewind()?;
    let mut text = String::new();
    file.take(65536).read_to_string(&mut text)?;
    anyhow::ensure!(
        status.success(),
        "Команда рабочего стола завершилась с {status}: {}",
        crate::media::safe_text(&text)
    );
    Ok(text.trim().to_owned())
}

fn dbus(script: &str) -> Result<String> {
    let path = program(&[
        "qdbus6",
        "qdbus-qt6",
        "/usr/lib/qt6/bin/qdbus",
        "/usr/lib/qt5/bin/qdbus",
        "qdbus",
    ])?;
    output(Command::new(path).args([
        "org.kde.plasmashell",
        "/PlasmaShell",
        "org.kde.PlasmaShell.evaluateScript",
        script,
    ]))
}

fn data_home() -> Result<PathBuf> {
    env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/share")))
        .filter(|p| p.is_absolute())
        .context("Не найден XDG_DATA_HOME / HOME")
}

fn runtime() -> Result<PathBuf> {
    let base = env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .context("Для Linux-обоев требуется XDG_RUNTIME_DIR графического сеанса")?;
    let path = base.join("asiji-wallpaper");
    match fs::DirBuilder::new().mode(0o700).create(&path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e.into()),
    }
    let meta = fs::symlink_metadata(&path)?;
    anyhow::ensure!(
        meta.is_dir()
            && meta.uid() == unsafe { libc::geteuid() }
            && meta.permissions().mode() & 0o077 == 0,
        "Небезопасный каталог {}",
        path.display()
    );
    Ok(path)
}

fn install_plasma() -> Result<()> {
    let version = output(Command::new(program(&["plasmashell"])?).arg("--version"))?;
    let major = version
        .split_whitespace()
        .find_map(|s| s.split('.').next()?.parse::<u32>().ok())
        .context("Не удалось определить версию Plasma")?;
    anyhow::ensure!(
        matches!(major, 5 | 6),
        "Поддерживаются Plasma 5/6; обнаружено {version}"
    );
    let root = data_home()?.join("plasma/wallpapers").join(PLUGIN);
    fs::create_dir_all(root.join("contents/ui"))?;
    fs::create_dir_all(root.join("contents/config"))?;
    fs::write(root.join("LICENSE"), include_str!("../../LICENSE"))?;
    let mut metadata: serde_json::Value =
        serde_json::from_str(include_str!("../../linux/plasma/metadata.json"))?;
    if major == 6 {
        metadata["X-Plasma-API-Minimum-Version"] = "6.0".into();
    }
    fs::write(
        root.join("metadata.json"),
        serde_json::to_vec_pretty(&metadata)?,
    )?;
    fs::write(
        root.join("contents/config/main.xml"),
        include_str!("../../linux/plasma/main.xml"),
    )?;
    fs::write(
        root.join("contents/ui/FrameView.qml"),
        include_str!("../../linux/plasma/FrameView.qml"),
    )?;
    fs::write(
        root.join("contents/ui/main.qml"),
        if major == 6 {
            include_str!("../../linux/plasma/main6.qml")
        } else {
            include_str!("../../linux/plasma/main5.qml")
        },
    )?;
    Ok(())
}

fn restore_plasma(url: Option<&str>) -> Result<()> {
    let expected = serde_json::to_string(&url)?;
    dbus(&format!(
        r#"var expected = {expected};
        desktops().forEach(function(d) {{
            if (d.wallpaperPlugin !== '{PLUGIN}') return;
            d.currentConfigGroup = ['Wallpaper', '{PLUGIN}', 'General'];
            if (expected !== null && d.readConfig('SourceUrl', '') !== expected) return;
            var previous = d.readConfig('PreviousPlugin', 'org.kde.image');
            d.writeConfig('SourceUrl', '');
            d.wallpaperPlugin = previous === '{PLUGIN}' ? 'org.kde.image' : previous;
        }});"#
    ))?;
    Ok(())
}

enum Desktop {
    Plasma(String),
    Process(Child, fs::File),
    Gnome(PathBuf),
}
pub struct Wallpaper {
    desktop: Desktop,
    server: FrameServer,
    _lock: fs::File,
}

impl Wallpaper {
    pub fn open(args: &Args) -> Result<Self> {
        let selected = backend(args)?;
        let runtime = runtime()?;
        let lock = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(runtime.join("session.lock"))?;
        lock.try_lock_exclusive()
            .context("Уже запущен другой сеанс живых обоев ASIJI")?;
        let server = FrameServer::start(args.fps)?;
        let url = server.url();
        let desktop = match selected {
            Backend::Plasma => {
                install_plasma()?;
                let source = serde_json::to_string(&url)?;
                let script = format!(
                    r#"var ds = desktopsForActivity(currentActivity());
                    if (ds.length === 0) throw new Error('No desktop');
                    ds.forEach(function(d) {{
                        var old = d.wallpaperPlugin;
                        d.currentConfigGroup = ['Wallpaper', '{PLUGIN}', 'General'];
                        if (old !== '{PLUGIN}') d.writeConfig('PreviousPlugin', old);
                        d.writeConfig('SourceUrl', {source});
                        d.writeConfig('Fps', {});
                        d.wallpaperPlugin = '{PLUGIN}';
                    }}); print(ds.length);"#,
                    args.fps
                );
                if let Err(error) = dbus(&script) {
                    let _ = restore_plasma(Some(&url));
                    return Err(error);
                }
                Desktop::Plasma(url)
            }
            Backend::Mpvpaper | Backend::X11 => {
                let (child, log) = start_player(selected, &url, args.fps)?;
                Desktop::Process(child, log)
            }
            Backend::Gnome => {
                install_gnome()?;
                let ready = runtime.join("gnome-ready");
                anyhow::ensure!(
                    fs::metadata(&ready)
                        .and_then(|m| m.modified())
                        .ok()
                        .and_then(|t| t.elapsed().ok())
                        .is_some_and(|age| age < Duration::from_secs(5)),
                    "Расширение GNOME установлено. Включите: gnome-extensions enable {GNOME_ID}. Если оно ещё не найдено, выйдите и войдите в сеанс, затем повторите команду."
                );
                let path = runtime.join("gnome.json");
                let mut file = tempfile::NamedTempFile::new_in(&runtime)?;
                write!(file, "{}", serde_json::json!({"url":url, "fps":args.fps}))?;
                file.persist(&path)?;
                Desktop::Gnome(path)
            }
            Backend::Auto => unreachable!(),
        };
        Ok(Self {
            desktop,
            server,
            _lock: lock,
        })
    }
    pub fn draw(
        &mut self,
        rgb: &[u8],
        dimensions: (usize, usize),
        mode: Mode,
        color: bool,
    ) -> Result<()> {
        if let Desktop::Process(child, log) = &mut self.desktop {
            if let Some(status) = child.try_wait()? {
                log.rewind()?;
                let mut message = String::new();
                log.take(4096).read_to_string(&mut message)?;
                bail!(
                    "Программа обоев завершилась ({status}): {}. См. docs/WALLPAPER.md",
                    crate::media::safe_text(&message)
                );
            }
        }
        self.server
            .publish(frame::bmp(rgb, dimensions, mode, color)?);
        Ok(())
    }
}

impl Drop for Wallpaper {
    fn drop(&mut self) {
        match &mut self.desktop {
            Desktop::Plasma(url) => {
                if let Err(e) = restore_plasma(Some(url)) {
                    eprintln!(
                        "Обои Plasma не восстановлены: {e}. Выполните asiji --wallpaper-restore"
                    );
                }
            }
            Desktop::Gnome(path) => {
                let _ = fs::remove_file(path);
            }
            Desktop::Process(child, _) => {
                // Each helper owns a fresh process group, including xwinwrap's mpv child.
                unsafe {
                    libc::kill(-(child.id() as i32), libc::SIGTERM);
                }
                thread::sleep(Duration::from_millis(100));
                unsafe {
                    libc::kill(-(child.id() as i32), libc::SIGKILL);
                }
                let _ = child.wait();
            }
        }
    }
}

fn start_player(backend: Backend, url: &str, fps: u32) -> Result<(Child, fs::File)> {
    let options = format!(
        "no-audio no-config no-terminal osc=no cache=no untimed=yes demuxer-lavf-format=bmp_pipe demuxer-lavf-o=framerate={fps} demuxer-lavf-probesize=32 demuxer-lavf-analyzeduration=0.1 demuxer-readahead-secs=0"
    );
    let stream = format!("{url}/stream.bmp");
    let mut command = if backend == Backend::Mpvpaper {
        let mut c = Command::new(program(&["mpvpaper"])?);
        c.args(["-o", &options, "ALL", &stream]);
        c
    } else {
        let mpv = program(&["mpv"])?;
        let mut c = Command::new(program(&["xwinwrap"])?);
        c.args([
            "-b",
            "-s",
            "-fs",
            "-st",
            "-sp",
            "-nf",
            "-ni",
            "-fdt",
            "--",
            "sh",
            "-c",
            "wid=$1; shift; exec \"$@\" --wid=\"$wid\"",
            "asiji-x11",
            "WID",
        ]);
        c.arg(mpv)
            .args(options.split_whitespace().map(|s| format!("--{s}")))
            .arg(stream);
        c
    };
    let log = tempfile::tempfile()?;
    command
        .process_group(0)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(log.try_clone()?);
    let parent = std::process::id() as libc::pid_t;
    unsafe {
        command.pre_exec(move || {
            if libc::prctl(
                libc::PR_SET_PDEATHSIG,
                libc::SIGTERM as libc::c_ulong,
                0 as libc::c_ulong,
                0 as libc::c_ulong,
                0 as libc::c_ulong,
            ) != 0
            {
                return Err(std::io::Error::last_os_error());
            }
            if libc::getppid() != parent {
                return Err(std::io::Error::other("ASIJI parent exited"));
            }
            Ok(())
        });
    }
    let child = command
        .spawn()
        .context("Не удалось запустить программу обоев")?;
    Ok((child, log))
}

fn install_gnome() -> Result<()> {
    let root = data_home()?.join("gnome-shell/extensions").join(GNOME_ID);
    fs::create_dir_all(&root)?;
    fs::write(root.join("LICENSE"), include_str!("../../LICENSE"))?;
    fs::write(
        root.join("metadata.json"),
        include_str!("../../linux/gnome/metadata.json"),
    )?;
    fs::write(
        root.join("extension.js"),
        include_str!("../../linux/gnome/extension.js"),
    )?;
    Ok(())
}

pub fn configure(args: &Args) -> Result<()> {
    if args.wallpaper_restore {
        restore_plasma(None)?;
        println!("Фон Plasma восстановлен");
        return Ok(());
    }
    match backend(args)? {
        Backend::Plasma => {
            install_plasma()?;
            println!("Плагин Plasma установлен. Включите живые обои в настройках ASIJI.");
        }
        Backend::Gnome => {
            install_gnome()?;
            println!(
                "Расширение установлено. Включите: gnome-extensions enable {GNOME_ID}. При первой установке может понадобиться выход и вход в сеанс."
            );
        }
        Backend::Mpvpaper => {
            program(&["mpvpaper"])?;
            println!("mpvpaper найден; включите живые обои в ASIJI.");
        }
        Backend::X11 => {
            program(&["xwinwrap"])?;
            program(&["mpv"])?;
            println!("xwinwrap и mpv найдены; включите живые обои в ASIJI.");
        }
        Backend::Auto => unreachable!(),
    }
    Ok(())
}
