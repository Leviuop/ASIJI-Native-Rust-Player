## ASIJI 0.9.1

Настройки теперь выбираются стрелками: **S → ↑↓ → ←→ / Enter → W**.
Визуализатор, палитра, чувствительность и громкость доступны без ввода команд.
**Esc** отменяет изменения, **E** открывает расширенные текстовые настройки.

Добавлены экспериментальные **живые обои Windows Explorer**: в настройках
включите «Живые обои (Windows)», сохраните и выберите трек. Видео или
визуализатор появится под значками основного монитора. Терминал можно свернуть;
**Q** возвращает в меню и убирает обои. Поддерживаются ASCII и цветной HD.
[Ограничения режима](https://github.com/Leviuop/ASIJI-Native-Rust-Player/blob/main/docs/WALLPAPER.md).
В Linux живые обои пока недоступны; терминальный плеер и новое меню работают.

При mute появляется заметная надпись с подсказкой клавиши включения звука.
Аудиодвижок не изменялся.

### Скачать

- **Windows 10/11 x64:** распакуйте ZIP и запустите `start.bat`. Недостающий
  FFmpeg скачивается при первом запуске (~109 МБ, проверка SHA-256).
- **Linux x64, glibc 2.35+:** установите FFmpeg и ALSA, распакуйте tar.gz,
  запустите `./start.sh`.
- **Debian 12+ / Ubuntu 22.04+:** `sudo apt install ./asiji_0.9.1_amd64.deb`, затем `asiji`.

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
