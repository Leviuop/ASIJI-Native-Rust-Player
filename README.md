# ASIJI

**Музыка и видео символами прямо в терминале.**

[![Build](https://github.com/Leviuop/ASIJI-Native-Rust-Player/actions/workflows/build.yml/badge.svg)](https://github.com/Leviuop/ASIJI-Native-Rust-Player/actions/workflows/build.yml)
[![Download](https://img.shields.io/badge/Download-Releases-25eebf)](https://github.com/Leviuop/ASIJI-Native-Rust-Player/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

![ASIJI — Bars, Wave, Orbit](docs/assets/visualizer.gif)

Нативный плеер на Rust для **Windows и Linux**. Видео превращается в цветной
ASCII или HD-полублоки, а аудио без видео — в настраиваемую визуализацию.
Локальные файлы, управление с клавиатуры, обычный TOML-конфиг.

**[Скачать](https://github.com/Leviuop/ASIJI-Native-Rust-Player/releases/latest)** ·
[Руководство](docs/GUIDE.md) · [Визуализатор](docs/VISUALIZER.md) · [Живые обои](docs/WALLPAPER.md) ·
[Конфиг](config.example.toml) · [Changelog](CHANGELOG.md) · [Contributing](https://github.com/Leviuop/ASIJI-Native-Rust-Player/blob/main/CONTRIBUTING.md)

Демонстрация выше — реальные кадры рендерера на синтетическом аудио.
В терминале картинка зависит от размера ячеек и шрифта.

## Быстрый старт

| Платформа | Скачать из Releases | Запуск |
|---|---|---|
| Windows 10/11 x64 | `windows-x86_64.zip` | Распаковать целиком → `start.bat` |
| Linux x64, glibc 2.35+ | `linux-x86_64.tar.gz` | Распаковать → `./start.sh` |
| Debian 12+ / Ubuntu 22.04+ | `asiji_0.9.0_amd64.deb` | `sudo apt install ./asiji_0.9.0_amd64.deb` → `asiji` |

Готовым сборкам **не нужны Rust и Python**. Windows при первом запуске скачает
недостающий FFmpeg (~109 МБ, проверка SHA-256); права администратора не нужны.
Для Linux-архива установите FFmpeg и ALSA через пакетный менеджер:

```bash
sudo apt install ffmpeg libasound2
./start.sh
```

На Ubuntu 24.04+ ALSA может называться `libasound2t64`. DEB установит зависимости
через APT. Нужны звуковой выход и терминал с UTF-8 и 24-битным цветом.
Для другой архитектуры или старой glibc доступна [сборка из исходников](docs/GUIDE.md#сборка-из-исходников).

Положите файлы в `media/` рядом с запуском или **перетащите их в меню треков и
нажмите Enter**. Одинаковые имена аудио и видео объединяются в пару:

```text
media/
  Track.mp3
  Track.mp4
  Audio only.flac
```

Можно указать свою папку через `--media`. После установки DEB медиатека находится
в `${XDG_DATA_HOME:-~/.local/share}/asiji/media`. Личные медиа в комплект не входят.

## Что умеет

- **Видео:** цветной ASCII и полублоки, синхронизация со звуком, перемотка и повтор.
- **Аудио:** Bars, Wave и Orbit; три цветовые темы, чувствительность, сглаживание и число полос.
- **Настройки:** меню **S**, TOML и CLI; свои бинды, громкость по умолчанию **10%**.
- **Живые обои:** видео или визуализатор под значками основного монитора Windows Explorer (экспериментально).
- **Медиатека:** импорт с прогрессом и отменой, копирование/перенос, обработка совпадающих имён.
- **Кэш:** автоматический лимит размера и возраста; очистка через **C** в меню.
- **Декодирование:** CPU, NVIDIA CUDA, Windows D3D11VA и Linux VAAPI; возврат на CPU при ошибке GPU.

GPU ускоряет декодирование видео. Символы и визуализатор рассчитываются на CPU.
Скорость вывода зависит и от терминала; уменьшите `--width` или `--fps`, если есть рывки.

## Управление

| Клавиши | Действие |
|---|---|
| **Space** | Пауза / продолжить |
| **← →** / **A D** | Перемотка на 5 секунд |
| **↑ ↓** / **+ −** | Громкость |
| **M** | Выключить / включить звук |
| **H** / **C** | ASCII ↔ HD / цвет ↔ монохром |
| **N P** / **R** | Следующий / предыдущий трек; повтор |
| **Q** / **Esc** | Меню треков |
| **X** / **Ctrl+C** | Выход |

В меню: **S** — настройки, **C** — очистка кэша, **R** — обновить список.
Используйте английскую раскладку. [Переназначение клавиш](docs/GUIDE.md#файл-настроек-и-бинды).

## Настроить под себя

В меню нажмите **S**: **↑↓** выбирают настройку, **←→** или **Enter** меняют
значение. Визуализатор, палитра, чувствительность, громкость и режим обоев
выбираются без ввода команд. **W** сохраняет, **Esc** отменяет.
**E** открывает расширенные текстовые настройки для путей, кэша и биндов.

Настройки применятся к следующему запуску трека. Или создайте файл через
`--init-config` и отредактируйте [TOML](config.example.toml). На Windows он лежит
в `%APPDATA%\asiji\config.toml`, на Linux — в `~/.config/asiji/config.toml`
(с поддержкой XDG). Приоритет: **CLI → конфиг → значения по умолчанию**.

```bash
./start.sh --mode blocks --visualizer bars --visual-theme aurora --fps 60
```

На Windows используйте `start.bat` с теми же параметрами.
[Все параметры визуализации](docs/VISUALIZER.md) · [Установка и настройки](docs/GUIDE.md).

## Совместимость и проверки

Windows и Linux проходят CI: сборку, lint, тесты, декодирование FFmpeg,
проверки конфигов, импорта и установщиков. Linux-сеансы воспроизведения
проверяются в PTY с виртуальным ALSA-выходом; это не проверка физических колонок.
Windows-воспроизведение и визуализаторы проверены локально в настоящей консоли.

CUDA и D3D11VA проверены на имеющемся Windows-ПК. **Linux VAAPI на физической
AMD/Intel пока не проверен.** [Оборудование, ограничения и тестовый сценарий](docs/HARDWARE.md).

```bash
./bin/asiji --no-config --doctor
./bin/asiji --no-config --doctor --doctor-video /path/to/clip.mp4
```

На Windows: `bin\asiji.exe` с теми же флагами. При сообщении об ошибке укажите
версию, ОС, терминал и вывод диагностики; не прикладывайте личные медиа.

## English quick start

Download a Windows ZIP, Linux tarball or Debian package from **Releases**.
Extract the archive, add files to `media/`, then run `start.bat` or `./start.sh`.
Windows downloads missing FFmpeg on first launch. Linux needs FFmpeg, ALSA
and glibc 2.35+. Prebuilt releases need neither Rust nor Python.

Audio-only tracks get native Bars, Wave or Orbit visuals. Press **S** in the
track menu to configure them; **H** toggles ASCII/half-block output during playback.
See the commented [config example](config.example.toml) and `--help`.

## Разработка и лицензия

[Сборка и проверки](https://github.com/Leviuop/ASIJI-Native-Rust-Player/blob/main/CONTRIBUTING.md) · [История изменений](CHANGELOG.md).

Код ASIJI: [MIT](LICENSE). Зависимости сохраняют свои лицензии:
[Third-party components](THIRD_PARTY.md). В релизах включены уведомления и
исходники MPL-компонентов. FFmpeg устанавливается отдельно.

[Права использования и распространения](docs/LICENSING.md): зависимости,
иллюстрации и шрифты, FFmpeg, системные компоненты и границы проверки.
