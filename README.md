# ASIJI

**Музыка и видео символами прямо в терминале.**

[![Build](https://github.com/Leviuop/ASIJI-Native-Rust-Player/actions/workflows/build.yml/badge.svg)](https://github.com/Leviuop/ASIJI-Native-Rust-Player/actions/workflows/build.yml)
[![Release](https://img.shields.io/github/v/release/Leviuop/ASIJI-Native-Rust-Player)](https://github.com/Leviuop/ASIJI-Native-Rust-Player/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Нативный плеер на Rust для Windows и Linux. Цветной ASCII, более детальный
режим полублоков, обычное управление с клавиатуры и синхронизация со звуком.

**[Скачать готовую сборку](https://github.com/Leviuop/ASIJI-Native-Rust-Player/releases/latest)** ·
[Управление](#управление) · [Настройки](#настройки) · [Сборка](#сборка-из-исходников)

## Быстрый старт

### Windows 10/11, x64

1. Скачайте `asiji-…-windows-x86_64.zip` из Releases и распакуйте целиком.
2. Положите свои файлы в `media/`.
3. Запустите `start.bat` и выберите номер трека.

При первом запуске недостающие FFmpeg и FFprobe скачиваются с gyan.dev
(~109 МБ, с проверкой SHA-256). Установка прав администратора не требует;
последующие запуски работают без интернета. Если FFmpeg уже есть в PATH или
папке `bin/`, скачивание не нужно. Rust и Python не требуются.

Для лучшей картинки используйте Windows Terminal, разверните окно и уменьшите
размер моноширинного шрифта. Запускайте из распакованной папки, не из ZIP.

### Linux, x64

Готовая сборка требует glibc 2.35+ и ALSA: например, Ubuntu 22.04+ или Debian 12+.

```bash
sudo apt update
sudo apt install ffmpeg libasound2
# Распакуйте архив из Releases, затем перейдите в его папку:
./start.sh
```

На Ubuntu 24.04+ пакет ALSA может называться `libasound2t64`.
Для других дистрибутивов установите FFmpeg и ALSA через свой пакетный менеджер.
Для старой glibc или другой архитектуры используйте сборку из исходников.
Нужны работающий звуковой выход и терминал с UTF-8 и 24-битным цветом.

### English quick start

Download and extract a Windows or Linux x64 archive from **Releases**.
Put your media in `media/`, then run `start.bat` or `./start.sh`.
Windows downloads missing FFmpeg on first launch. Linux needs system FFmpeg,
ALSA and glibc 2.35+. Prebuilt releases need neither Rust nor Python.
Press **Space** to pause, **H** for half-block video, **Q** for the menu.

## Медиатека

```text
media/
  My track.mp3
  My track.mp4
  Another clip.webm
```

Одинаковые имена без расширения объединяются в пару; регистр не важен.
Отдельный аудиофайл имеет приоритет над звуком видео. Один клип со звуком тоже
работает. Для аудио без видео отображается спектр. Вложенные папки не сканируются.

- **Аудио:** FLAC, WAV, MP3, M4A, OGG, OPUS, AAC, WMA, AIFF.
- **Видео:** MP4, MKV, WEBM, MOV, AVI, M4V, MPEG, MPG.

Короткое видео повторяется до конца аудио. По завершении включается следующий
трек; после последнего открывается меню. `R` обновляет список. При обновлении
и возврате из плеера экран и история прокрутки очищаются.

Медиафайлы в комплект не входят. Плеер не изменяет и не отправляет ваши треки.

## Управление

Во время воспроизведения используйте английскую раскладку.

| Клавиша | Действие |
|---|---|
| Space | Пауза / продолжить |
| ← / → или A / D | Перемотка на 5 секунд |
| ↑ / ↓ или + / − | Громкость |
| M | Выключить / включить звук |
| H | ASCII / HD-полублоки |
| C | Цвет / монохром |
| N / P | Следующий / предыдущий трек |
| R | Повтор трека |
| Q / Esc | Вернуться в меню |
| X / Ctrl+C | Закрыть плеер |

## Настройки

### Файл настроек и бинды

Создать TOML-конфиг с комментариями:

```powershell
# Windows, из папки плеера
.\bin\asiji.exe --init-config
notepad "$env:APPDATA\asiji\config.toml"
```

```bash
# Linux, из папки плеера
./bin/asiji --init-config
${EDITOR:-nano} "${XDG_CONFIG_HOME:-$HOME/.config}/asiji/config.toml"
```

Windows читает `%APPDATA%\asiji\config.toml`. Linux читает
`$XDG_CONFIG_HOME/asiji/config.toml`, а если XDG_CONFIG_HOME не задан или не является
абсолютным путём — `~/.config/asiji/config.toml`. Создание не перезаписывает существующий файл.
Без конфига действуют обычные настройки. Пример включён в релиз: [config.example.toml](config.example.toml).

```toml
volume = 10
muted = false
repeat = false
fps = 60
width = 0
mode = "blocks"
color = true
hwaccel = "auto"
# media = "D:/Music"       # Windows
# media = "/home/me/Music" # Linux

[bindings]
pause = ["space", "k"]
seek_backward = ["left", "j"]
seek_forward = ["right", "l"]
mode = ["f2"]
```

Приоритет: **параметры CLI → конфиг → встроенные значения**. Например,
`--volume 10 --fps 30 --color --repeat false` перекрывает соответствующие значения файла.
`--muted true` запускает без звука, `--mono` отключает цвет.

Каждый список в `[bindings]` заменяет все клавиши одного действия; остальные действия
сохраняют назначения по умолчанию. `[]` отключает действие. Конфликт клавиш или
неизвестное действие вызывает ошибку с путём к конфигу. **Ctrl+C всегда закрывает плеер.**
Подсказки во время воспроизведения показывают первую назначенную клавишу каждого действия.
В списке треков по-прежнему работают номера, `R` и `Q` с Enter.

Можно назначать одиночные символы, `space`, `enter`, `esc`, `tab`, `backspace`,
`left`, `right`, `up`, `down`, `home`, `end`, `pageup`, `pagedown`, `insert`, `delete`,
`f1`–`f12`, а также сочетания `ctrl+`, `alt+`, `shift+`. Некоторые сочетания перехватывает
терминал или окружение рабочего стола; они не доходят до плеера. Большие и маленькие
латинские буквы по умолчанию равнозначны; явный бинд `shift+буква` имеет приоритет.

```bash
./bin/asiji --print-config                  # показать итоговые значения
./bin/asiji --no-config                     # запустить без конфига
./bin/asiji --config ./config.toml          # свой файл, в том числе портативный
./bin/asiji --config ./config.toml --init-config
```

На Windows те же параметры принимает `.\bin\asiji.exe` или `start.bat`.
Конфиг перечитывается при запуске; изменения клавишами не записываются в файл.
Относительный `media` в TOML отсчитывается от папки конфига. CLI `--media` — от
рабочей папки процесса. В строках путей не раскрываются `~` и переменные окружения:
используйте абсолютные пути или путь относительно конфига.

### Параметры запуска

По умолчанию: **10% громкости**, ASCII, до 30 FPS, размер по окну.
HD использует два цветных пикселя на ячейку терминала. Пропорции сохраняются.

```bat
start.bat --mode blocks --fps 60
start.bat --width 320 --fps 24
start.bat --hwaccel cuda
start.bat --hwaccel cpu
start.bat --media "D:\Music"
```

На Linux замените `start.bat` на `./start.sh`. Все параметры: `--help`.
`--width 0` подстраивает сетку под окно (до 1000 колонок); `--mono` убирает цвет.
Частота ограничена исходным видео: клип на 24 FPS не превращается в 60 FPS.

### CPU и GPU

`--hwaccel auto` коротко замеряет декодирование перед открытием трека.
На Windows сравниваются CPU, CUDA и D3D11VA, на Linux — CPU, CUDA и VAAPI.
Недоступные варианты пропускаются; GPU выбирается при заметном выигрыше.
Для небольших видео CPU часто быстрее. Результат виден в заголовке плеера.

CUDA требует совместимых NVIDIA, драйвера, кодека и сборки FFmpeg.
Если выбранный GPU недоступен или декодер ломается во время воспроизведения,
плеер переходит на CPU. Для строгой проверки используйте
`--hwaccel cuda --hwaccel-fallback false`. В режиме `auto` CPU всегда участвует
в сравнении; `hwaccel_fallback` управляет восстановлением после выбора декодера.
Для Radeon и Intel на Linux используйте VAAPI; на Windows — D3D11VA.
Наличие ускорителя в `ffmpeg -hwaccels` не гарантирует поддержку конкретного кодека.
CUDA Toolkit и ROCm устанавливать не нужно; нужны подходящие системные драйверы.

```bash
./bin/asiji --hwaccel vaapi --hwaccel-device /dev/dri/renderD128
./bin/asiji --doctor
./bin/asiji --doctor --doctor-video "/path/to/clip.mp4"
```

На Windows те же флаги принимает `bin\asiji.exe`; для CUDA/D3D11VA устройство
задаётся индексом, например `--hwaccel cuda --hwaccel-device 0`.
В конфиге: `hwaccel`, `hwaccel_device`, `hwaccel_fallback`.
Если устройство указано явно, выбирайте соответствующий backend вместо `auto`:
пути VAAPI и индексы CUDA имеют разный смысл. Без параметра используется устройство
по умолчанию; автоматический перебор всех видеокарт не выполняется.

`--doctor` работает без терминального интерфейса и аудиоустройства, ничего не
устанавливает. Без видео показывает версии и поддержку в сборке FFmpeg;
с `--doctor-video` декодирует восемь кадров каждым backend и печатает ошибки.
Код возврата 0 означает, что базовые зависимости (и CPU-декодирование при проверке
файла) работают; недоступный необязательный GPU показывается как FAIL.
При ошибке чтения конфига добавьте `--no-config`.

GPU ускоряет декодирование. Масштабирование, подготовка символов и передача
ANSI остаются на CPU. Плеер отправляет только изменившиеся ячейки, переиспользует
буферы и выбирает кадры по позиции звука. Скорость также зависит от терминала:
при рывках уменьшите `--width` или `--fps`. FPS в статусе — полученные новые
кадры, а не измерение обновления монитора.

### Установка в каталог пользователя

Portable-запуск остаётся доступен. В Windows можно запустить `install.bat`:
плеер устанавливается в `%LOCALAPPDATA%\Programs\ASIJI`, создаётся ярлык в меню
«Пуск». FFmpeg скачивается при первом запуске установленного плеера. Драйверы,
PATH и системные настройки установщик не меняет. Для своего пути:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/install-windows.ps1 -Destination "D:\Apps\ASIJI"
```

В Linux выполните `bash install.sh` из распакованного релиза. Файлы попадут в
`${XDG_DATA_HOME:-~/.local/share}/asiji`, ссылка `asiji` — в `~/.local/bin`.
Добавьте этот каталог в PATH, если он ещё не добавлен. Можно передать другой
каталог первым аргументом; `ASIJI_BIN_DIR` задаёт каталог команды.
Системные зависимости устанавливайте пакетным менеджером дистрибутива:
FFmpeg, ALSA, для AMD/Intel — драйвер VAAPI. Доступность аппаратных кодеков зависит
от сборки драйвера и политики дистрибутива. Скрипт не запускает `sudo`.
Готовые `.deb`/`.rpm` пока не выпускаются.

Оба установщика сохраняют существующие настройки и медиатеку при обновлении.
Личные треки из исходной папки не копируются: укажите их каталог через `media`
в конфиге или перенесите самостоятельно. Удаление: удалите каталог установленного
плеера (сначала сохраните добавленные туда треки), затем ярлык «Пуск» или ссылку
`~/.local/bin/asiji`. Пользовательский конфиг находится отдельно и сохраняется.



## Если что-то не работает

- **Нет FFmpeg:** запустите `start.bat` с интернетом либо установите FFmpeg вручную.
  Можно положить `ffmpeg.exe` и `ffprobe.exe` в `bin/` или добавить их в PATH.
  Также поддерживаются `ASIJI_FFMPEG` и `ASIJI_FFPROBE`.
- **Нет звука:** проверьте устройство вывода, системный микшер, `M` и громкость.
- **Первое открытие долгое:** звук декодируется в `.cache/`, затем идёт подбор
  декодера. Кэш занимает примерно 11 МБ на минуту; при закрытом плеере его можно удалить.
- **Ошибка CUDA:** попробуйте `--hwaccel cpu`; проверьте драйвер NVIDIA.
- **Маленькое окно:** требуется не менее 40 колонок и 12 строк.
- **Linux: Permission denied:** выполните `chmod +x start.sh bin/asiji`.
- **Linux: GLIBC not found:** соберите на своей системе по инструкции ниже.

Диагностика FFmpeg, аудиоустройства и медиатеки:

```bat
start.bat --check
```

Сообщая об ошибке в [Issues](https://github.com/Leviuop/ASIJI-Native-Rust-Player/issues),
укажите ОС, терминал, версию плеера, команду запуска и текст ошибки.

## Сборка из исходников

Нужен актуальный stable [Rust](https://rustup.rs/) и FFmpeg.
На Windows также установите Visual Studio Build Tools с C++ workload и Windows SDK.

```bat
build.bat
start.bat
```

Debian/Ubuntu:

```bash
sudo apt install build-essential pkg-config libasound2-dev ffmpeg
bash build.sh
bash start.sh
```

Без готового бинарника скрипт запуска сам вызывает сборку. Для пересборки после
обновления исходников вызовите `build.bat` или `bash build.sh` вручную.

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo test --locked real_decode_seek_loop_and_cache -- --ignored
```

Полная проверка звука требует устройства вывода:
`cargo test --locked -- --include-ignored --test-threads=1`.
CI проверяет Windows/Linux, реальное декодирование FFmpeg и меню в Linux PTY.
Python используется только для упаковки релизов и теста меню.

### Измерения

```bat
bin\asiji.exe --benchmark-video "media\Clip.mp4" --hwaccel cpu --mode blocks --width 500 --bench-rows 140 --bench-frames 120
bin\asiji.exe --play 1 --profile playback.json --profile-seconds 10
```

Первый режим сравнивает подготовку ANSI на одинаковых RGB-кадрах, без терминала.
Второй записывает времена и объём вывода при воспроизведении. `--profile`
перезаписывает указанный файл отчётом последнего трека. Время записи в терминал
не равно времени отображения на экране; время очереди не включает работу FFmpeg.

## Лицензия

[MIT](LICENSE). Зависимости и FFmpeg распространяются по своим лицензиям:
[Third-party components](THIRD_PARTY.md).
