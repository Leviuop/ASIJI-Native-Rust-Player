## ASIJI 0.8.0

Аудио без видео теперь получает нативную визуализацию: **Bars, Wave или Orbit**.
Три темы (Aurora, Ember, Ice), чувствительность, затухание и число полос
настраиваются через меню **S**, TOML и CLI на Windows и Linux.
Визуализация следует за звуком, паузой и перемоткой; дополнительный процесс
FFmpeg для неё не нужен.

### Скачать

- **Windows 10/11 x64:** распакуйте ZIP и запустите `start.bat`. Недостающий
  FFmpeg скачивается при первом запуске (~109 МБ, проверка SHA-256).
- **Linux x64, glibc 2.35+:** установите FFmpeg и ALSA, распакуйте tar.gz,
  запустите `./start.sh`.
- **Debian 12+ / Ubuntu 22.04+:** `sudo apt install ./asiji_0.8.0_amd64.deb`, затем `asiji`.

Rust и Python для готовых сборок не нужны. Громкость при запуске — 10%.
Положите свои файлы в `media/` или перетащите их в меню треков.
Для детального визуала: `--mode blocks --visualizer orbit --visual-theme ember`.

[Руководство](https://github.com/Leviuop/ASIJI-Native-Rust-Player/blob/main/docs/GUIDE.md) ·
[Настройки визуала](https://github.com/Leviuop/ASIJI-Native-Rust-Player/blob/main/docs/VISUALIZER.md)

Windows-воспроизведение проверено локально. Linux CI проверяет терминальные
сеансы с ALSA null; физический VAAPI на AMD/Intel пока не проверен.
Пользовательских аудио и видео в пакетах нет. Иллюстрация создана на синтетическом сигнале.

### Third-party components

Windows and Linux x64 builds; prebuilt packages need neither Rust nor Python.
Native audio visualizers, configurable through TOML, CLI and the settings menu.

Each distribution includes dependency notices in `licenses/` and complete
unmodified Symphonia 0.5.5 source archives under MPL-2.0 in `sources/`
(`/usr/share/doc/asiji/` for DEB). Keep these when redistributing.
FFmpeg is installed separately. See
[THIRD_PARTY.md](https://github.com/Leviuop/ASIJI-Native-Rust-Player/blob/main/THIRD_PARTY.md).
