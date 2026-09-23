use crate::{Args, config, media::safe_text};
use anyhow::Result;
use std::io::{self, Write};

pub fn open(args: &mut Args) -> Result<bool> {
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
