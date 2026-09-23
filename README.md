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
На Windows сравниваются CPU, CUDA и D3D11VA, на Linux — CPU и CUDA.
Недоступные варианты пропускаются; GPU выбирается при заметном выигрыше.
Для небольших видео CPU часто быстрее. Результат виден в заголовке плеера.

CUDA требует совместимых NVIDIA, драйвера, кодека и сборки FFmpeg.
Принудительный `--hwaccel cuda` не переключается на CPU при ошибке:
сообщение появляется в меню. Для обычного запуска оставьте `auto`.

GPU ускоряет декодирование. Масштабирование, подготовка символов и передача
ANSI остаются на CPU. Плеер отправляет только изменившиеся ячейки, переиспользует
буферы и выбирает кадры по позиции звука. Скорость также зависит от терминала:
при рывках уменьшите `--width` или `--fps`. FPS в статусе — полученные новые
кадры, а не измерение обновления монитора.

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
