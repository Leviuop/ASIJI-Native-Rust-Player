use crate::{Args, config, media::safe_text};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use std::io::{self, IsTerminal, Write};

pub fn open(args: &mut Args) -> Result<bool> {
    if io::stdin().is_terminal() && io::stdout().is_terminal() {
        return interactive(args);
    }
    text_menu(args)
}

fn text_menu(args: &mut Args) -> Result<bool> {
    let mut draft = args.clone();
    loop {
        println!(
            "\nНастройки: volume={} fps={} width={} mode={:?} color={} GPU={:?}",
            draft.volume, draft.fps, draft.width, draft.mode, !draft.mono, draft.hwaccel
        );
        println!(
            "Кэш: {} МиБ / {} дней. Импорт: {:?}, конфликт: {:?}",
            draft.cache_max_mb, draft.cache_max_days, draft.import_mode, draft.import_conflict
        );
        println!(
            "Визуал: {:?} / {:?}; gain={} smoothing={} bands={}",
            draft.visualizer,
            draft.visual_theme,
            draft.visual_gain,
            draft.visual_smoothing,
            draft.visual_bands
        );
        println!(
            "visualizer=bars/wave/orbit, visual_theme=aurora/ember/ice, visual_gain=10..400, visual_smoothing=0..99, visual_bands=8..128"
        );
        println!("Бинды: {:?}", draft.bindings.names);
        println!(
            "Введите настройка=значение, например volume=10, fps=60, mode=blocks, hwaccel=auto."
        );
        println!(
            "Также: width, color, hwaccel_device, hwaccel_fallback, cache_max_mb, cache_max_days,"
        );
        println!(
            "cache_dir, media, import_mode=copy/move, import_conflict=skip/rename, bind pause=k,space"
        );
        print!("W — сохранить / Q — отменить: ");
        io::stdout().flush()?;
        let mut line = String::new();
        if io::stdin().read_line(&mut line)? == 0 {
            return Ok(false);
        }
        let line = line.trim();
        if line.eq_ignore_ascii_case("q") {
            return Ok(false);
        }
        if line.eq_ignore_ascii_case("w") {
            match config::save(&draft) {
                Ok(path) => {
                    println!("Сохранено: {}", path.display());
                    draft.no_config = false;
                    *args = draft;
                    return Ok(true);
                }
                Err(error) => println!("Не сохранено: {}", safe_text(&format!("{error:#}"))),
            }
        } else if let Some((key, value)) = line.split_once('=') {
            if let Err(error) = config::edit(&mut draft, key.trim(), value.trim()) {
                println!("Ошибка: {}", safe_text(&format!("{error:#}")));
            }
        } else {
            println!("Ожидается настройка=значение");
        }
    }
}

const FIELDS: &[(&str, &str, &[&str])] = &[
    ("visualizer", "Визуализатор", &["bars", "wave", "orbit"]),
    ("visual_theme", "Палитра", &["aurora", "ember", "ice"]),
    ("mode", "Картинка", &["ascii", "blocks"]),
    ("visual_gain", "Чувствительность", &[]),
    ("visual_smoothing", "Сглаживание", &[]),
    ("visual_bands", "Полосы спектра", &[]),
    ("volume", "Громкость", &[]),
    ("muted", "Без звука", &["false", "true"]),
    ("fps", "Частота кадров", &[]),
    ("color", "Цвет", &["true", "false"]),
    ("repeat", "Повтор", &["false", "true"]),
    (
        "hwaccel",
        "Декодер",
        &["auto", "cpu", "cuda", "d3d11va", "vaapi"],
    ),
    ("wallpaper", "Живые обои (Windows)", &["false", "true"]),
];

fn adjust(draft: &mut Args, row: usize, direction: i32) -> Result<()> {
    let (key, _, choices) = FIELDS[row];
    let values: toml::Table = toml::from_str(&config::effective(draft)?)?;
    let value = &values[key];
    let next = if choices.is_empty() {
        let (min, max, step) = match key {
            "visual_gain" => (10, 400, 10),
            "visual_smoothing" => (0, 99, 5),
            "visual_bands" => (8, 128, 8),
            "fps" => (10, 60, 5),
            _ => (0, 100, 5),
        };
        (value.as_integer().unwrap_or(min) + direction as i64 * step)
            .clamp(min, max)
            .to_string()
    } else {
        let current = value
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| value.to_string());
        let index = choices.iter().position(|c| *c == current).unwrap_or(0);
        choices[(index as i32 + direction).rem_euclid(choices.len() as i32) as usize].to_owned()
    };
    if key == "wallpaper" && next == "true" && !cfg!(windows) {
        anyhow::bail!("Живые обои пока доступны только в Windows Explorer");
    }
    config::edit(draft, key, &next)
}

fn interactive(args: &mut Args) -> Result<bool> {
    let mut draft = args.clone();
    let mut selected: usize = 0;
    let mut message = String::new();
    let mut terminal = Some(crate::Terminal::open()?);
    loop {
        let (width, height) = crossterm::terminal::size()?;
        let values: toml::Table = toml::from_str(&config::effective(&draft)?)?;
        let visible = usize::from(height.saturating_sub(7)).max(1);
        let start = selected.saturating_sub(visible - 1);
        let mut output = String::from("\x1b[H\x1b[2J");
        let mut line = |text: &str| {
            output.push_str(&crate::render::clip(text, width.saturating_sub(1) as usize));
            output.push_str("\r\n");
        };
        line(" ASIJI / Настройки");
        line(" ↑↓ выбор   ←→ / Enter изменить");
        line(" W сохранить   Esc отмена   E расширенные настройки");
        line("");
        for (row, (key, label, _)) in FIELDS.iter().enumerate().skip(start).take(visible) {
            let value = values[*key]
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| values[*key].to_string());
            line(&format!(
                " {} {label}:  < {value} >",
                if row == selected { ">" } else { " " }
            ));
        }
        line(&message);
        io::stdout().write_all(output.as_bytes())?;
        io::stdout().flush()?;
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind == KeyEventKind::Release {
            continue;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Ok(false);
        }
        match key.code {
            KeyCode::Up => selected = (selected + FIELDS.len() - 1) % FIELDS.len(),
            KeyCode::Down => selected = (selected + 1) % FIELDS.len(),
            KeyCode::Left | KeyCode::Right | KeyCode::Enter => {
                message = match adjust(
                    &mut draft,
                    selected,
                    if key.code == KeyCode::Left { -1 } else { 1 },
                ) {
                    Ok(()) => String::new(),
                    Err(e) => safe_text(&e.to_string()),
                };
            }
            KeyCode::Esc | KeyCode::Char('q' | 'Q') => return Ok(false),
            KeyCode::Char('w' | 'W') => match config::save(&draft) {
                Ok(_) => {
                    draft.no_config = false;
                    *args = draft;
                    return Ok(true);
                }
                Err(e) => message = safe_text(&format!("Не сохранено: {e:#}")),
            },
            KeyCode::Char('e' | 'E') => {
                drop(terminal.take());
                if text_menu(&mut draft)? {
                    *args = draft;
                    return Ok(true);
                }
                terminal = Some(crate::Terminal::open()?);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    #[test]
    fn choices_wrap_and_numeric_values_stay_in_range() {
        let mut args = Args::parse_from(["asiji"]);
        adjust(&mut args, 0, -1).unwrap();
        assert_eq!(args.visualizer, crate::visualizer::Style::Orbit);
        adjust(&mut args, 0, 1).unwrap();
        assert_eq!(args.visualizer, crate::visualizer::Style::Bars);
        for _ in 0..30 {
            adjust(&mut args, 6, -1).unwrap();
        }
        assert_eq!(args.volume, 0);
        adjust(&mut args, 7, 1).unwrap();
        assert!(args.muted);
    }
}
