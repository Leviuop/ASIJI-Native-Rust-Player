## ASIJI 0.9.3

Ускорена CPU-отрисовка визуализаторов без изменения картинки:
- Orbit повторно использует геометрию кадра.
- Wave рассчитывает положение волны один раз на столбец.
- Побайтное совпадение со старым рендерером проверяется на Windows и Linux.

На тестовом Windows-ПК Orbit рисуется примерно в 4,5 раза быстрее, Wave —
в 1,3 раза. Это время отрисовки RGB, а не общий FPS терминала.
[Методика и результаты](https://github.com/Leviuop/ASIJI-Native-Rust-Player/blob/main/docs/PERFORMANCE.md).

Настройки: **S → ↑↓ → ←→ / Enter → W**. **Esc** отменяет изменения.

### Скачать

- **Windows 10/11 x64:** распакуйте ZIP и запустите `start.bat`. Недостающий
  FFmpeg скачивается при первом запуске (~109 МБ, проверка SHA-256).
- **Linux x64, glibc 2.35+:** установите FFmpeg и ALSA, распакуйте tar.gz,
  запустите `./start.sh`.
- **Debian 12+ / Ubuntu 22.04+:** `sudo apt install ./asiji_0.9.3_amd64.deb`, затем `asiji`.

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
