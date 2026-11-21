use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

#[derive(Clone)]
struct FileEntry {
    path: PathBuf,
    size: u64,
}

#[derive(Clone, Copy, PartialEq)]
enum Lang {
    Ru,
    En,
}

impl Lang {
    fn t<'a>(&self, ru: &'a str, en: &'a str) -> &'a str {
        match self { Lang::Ru => ru, Lang::En => en }
    }
}

const MAX_ATTEMPTS: usize = 4;

// АНИМАЦИИ
fn anim_parrot(count: usize) {
    let frames = [
        r"  (\__/)   osc",
        r"  (='.'=)  osc",
        r"  (o_)-(o)  ocl",
        r"  (='.'=)  sco",
    ];
    for i in 0..count {
        print!("\r🦜 {}", frames[i % frames.len()]);
        io::stdout().flush().ok();
        thread::sleep(Duration::from_millis(120));
    }
    println!();
}

fn anim_runner(count: usize) {
    let frames = ["🏃", "🏃‍♂️", "🏃", "🏃‍♂️"];
    for i in 0..count {
        print!("\r{} {}", frames[i % frames.len()], ".".repeat((i % 4) + 1));
        io::stdout().flush().ok();
        thread::sleep(Duration::from_millis(120));
    }
    println!();
}

fn anim_hand(count: usize) {
    let frames = ["🤚", "🖐", "✋", "🖐"];
    for i in 0..count {
        print!("\r{} {}", frames[i % frames.len()], ".".repeat((i % 3) + 1));
        io::stdout().flush().ok();
        thread::sleep(Duration::from_millis(220));
    }
    println!();
}

fn play_animation(lang: Lang) {
    println!("\n{}", lang.t("Выберите анимацию загрузки:", "Choose a loading animation:"));
    println!("  1 — 🦜 {}", lang.t("Попугай", "Parrot"));
    println!("  2 — 🏃 {}", lang.t("Бегущий человечек", "Running man"));
    println!("  3 — 🤚 {}", lang.t("Двигающаяся рука", "Moving hand"));
    println!("  4 — {}", lang.t("Без анимации", "No animation"));
    let c = prompt("> ");
    match c.as_str() {
        "1" => anim_parrot(16),
        "2" => anim_runner(16),
        "3" => anim_hand(12),
        _ => {}
    }
}

// ВСПОМОГАТЕЛЬНОЕ
fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    format!("{:.2} {}", size, UNITS[unit])
}

fn walk(dir: &Path, files: &mut Vec<FileEntry>, errors: &mut usize) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => { *errors += 1; return; }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let meta = match fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(_) => { *errors += 1; continue; }
        };
        if meta.file_type().is_symlink() { continue; }
        if meta.is_dir() { walk(&path, files, errors); }
        else if meta.is_file() { files.push(FileEntry { path, size: meta.len() }); }
    }
}

fn prompt(msg: &str) -> String {
    print!("{}", msg);
    io::stdout().flush().ok();
    let mut s = String::new();
    io::stdin().read_line(&mut s).ok();
    s.trim().to_string()
}

fn choose_lang() -> Lang {
    println!("Выберите язык / Choose language:");
    println!("  1 — Русский");
    println!("  2 — English");
    loop {
        match prompt("> ").as_str() {
            "1" | "р" | "R" | "ру" => return Lang::Ru,
            "2" | "e" | "E" | "en" => return Lang::En,
            _ => println!("Введите 1 или 2 / Enter 1 or 2"),
        }
    }
}

fn greeting(lang: Lang) {
    println!("\n🦫  SpaceHound 0.5 — {}", lang.t("умный поиск больших файлов", "smart large-file finder"));
    println!("{}", lang.t(
        "Сканирую диск, нахожу самое большое и предупреждаю о лишнем.\nНичего не удаляется без подтверждения — всё уходит в резервную копию.",
        "Scans your disk, finds the largest files and warns about junk.\nNothing is deleted without confirmation — everything goes to a backup."
    ));
    println!("⚠️  {}", lang.t(
        "Программа сделана ТОЛЬКО для Linux (Pop!_OS, Ubuntu, Debian и др.). На Windows/macOS не запускается.",
        "This program is made ONLY for Linux (Pop!_OS, Ubuntu, Debian, etc.). Does not run on Windows/macOS."
    ));
}

fn junk_reason(path: &Path, lang: Lang) -> Option<String> {
    let name = path.file_name()?.to_string_lossy().to_lowercase();
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    let full = path.to_string_lossy().to_lowercase();

    if full.contains("/.fr-") || name.starts_with(".com.google") {
        return Some(lang.t("временный файл распаковки", "temporary extraction file").into());
    }
    if matches!(ext.as_str(), "tmp" | "temp" | "log" | "old" | "bak" | "crdownload" | "part") {
        return Some(lang.t("временный/резервный файл", "temporary/backup file").into());
    }
    if name.contains("(1)") || name.contains("(2)") || name.contains("(3)") || name.contains("копия") || name.contains("copy") {
        return Some(lang.t("похоже на дубликат", "looks like a duplicate").into());
    }
    if ext == "deb" || ext == "rpm" || ext == "msi" || ext == "dmg" {
        return Some(lang.t("установщик (после установки не нужен)", "installer (not needed after install)").into());
    }
    if ext == "exe" && (name.contains("igram") || name.contains("mod") || name.contains("hack")) {
        return Some(lang.t("сторонний модифицированный клиент — ВНИМАНИЕ", "third-party modified client — WARNING").into());
    }
    None
}

fn dirs_home() -> PathBuf {
    PathBuf::from(env::var("HOME").unwrap_or_else(|_| "/home/steve".into()))
}

fn backup_dir() -> PathBuf {
    let mut p = dirs_home();
    p.push(".spacehound-backup");
    p
}

fn backup_file(path: &Path, lang: Lang) -> bool {
    let bdir = backup_dir();
    if fs::create_dir_all(&bdir).is_err() {
        println!("❌ {}", lang.t("не удалось создать папку бэкапа", "could not create backup dir"));
        return false;
    }
    let dest = bdir.join(path.file_name().unwrap_or_default());
    match fs::rename(path, &dest) {
        Ok(_) => { println!("   ✅ {} → {}", lang.t("перемещено в бэкап", "moved to backup"), dest.display()); true }
        Err(e) => {
            if fs::copy(path, &dest).is_ok() {
                fs::remove_file(path).ok();
                println!("   ✅ {} → {}", lang.t("скопировано в бэкап", "copied to backup"), dest.display());
                true
            } else {
                println!("   ❌ {}: {}", lang.t("ошибка", "error"), e);
                false
            }
        }
    }
}

/// Возвращает (продолжить?, выбранный_путь, повторить_тот_же?).
fn scan_once(lang: Lang, args: &[String], path_override: Option<&str>, attempt: usize) -> (bool, Option<String>, bool) {
    println!("\n{} {} {}/{}",
        lang.t("Попытка сканирования:", "Scan attempt:"), attempt, "/", MAX_ATTEMPTS);

    let root = if let Some(p) = path_override {
        p.to_string()
    } else if args.len() >= 2 && !args[1].starts_with("--") {
        args[1].clone()
    } else {
        prompt(&lang.t("\nВведите путь для сканирования (Enter = весь дом): ", "\nEnter path to scan (Enter = whole home): "))
    };
    let root = if root.is_empty() { dirs_home().to_string_lossy().to_string() } else { root };

    let n = if let Some(i) = args.iter().position(|a| a == "-n") {
        args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(20)
    } else { 20 };

    let path = Path::new(&root);
    if !path.exists() {
        println!("❌ {}: {}", lang.t("путь не найден", "path not found"), root);
        return (true, Some(root), false);
    }

    println!("\n🔍 {} {} ...", lang.t("Сканирую", "Scanning"), root);
    // анимация во время сканирования
    anim_parrot(12);

    let start = std::time::Instant::now();
    let mut files = Vec::new();
    let mut errors = 0usize;
    walk(path, &mut files, &mut errors);
    let elapsed = start.elapsed();

    files.sort_by(|a, b| b.size.cmp(&a.size));
    let top: Vec<FileEntry> = files.iter().take(n).cloned().collect();

    println!("\n✅ {}: {} {:.2?} ({} {})", lang.t("Найдено", "Found"), top.len(), elapsed, errors, lang.t("ошибок", "errors"));
    println!("{:<5} {:<13} {:<45} {}", "№", lang.t("РАЗМЕР", "SIZE"), lang.t("ФАЙЛ", "FILE"), lang.t("КОММЕНТАРИЙ", "NOTE"));

    let mut suspicious = Vec::new();
    for (i, f) in top.iter().enumerate() {
        let reason = junk_reason(&f.path, lang);
        if reason.is_some() { suspicious.push(f.clone()); }
        println!("{:<5} {:<13} {:<45} {}", i + 1, human_size(f.size), {
            let s = f.path.file_name().unwrap_or_default().to_string_lossy().to_string();
            if s.len() > 44 { format!("{}...", &s[..42]) } else { s }
        }, reason.clone().unwrap_or_default());
    }

    if !suspicious.is_empty() {
        println!("\n⚠️  {}:", lang.t("Рекомендации", "Recommendations"));
        for (i, f) in suspicious.iter().enumerate() {
            println!("  [{}] {} — {}", i + 1, f.path.display(), human_size(f.size));
        }
        println!("\n{}", lang.t(
            "Резервная копия: ~/.spacehound-backup (файл НЕ удаляется, а перемещается туда).",
            "Backup: ~/.spacehound-backup (the file is MOVED there, not deleted)."
        ));
        let ans = prompt(&lang.t(
            "\nВведите номера через запятую для переноса в бэкап (Enter = пропустить): ",
            "\nEnter comma-separated numbers to move to backup (Enter = skip): "
        ));
        for part in ans.split(',') {
            if let Ok(idx) = part.trim().parse::<usize>() {
                if idx >= 1 && idx <= suspicious.len() { backup_file(&suspicious[idx - 1].path, lang); }
            }
        }
    } else {
        println!("\n{}", lang.t("Подозрительных файлов не найдено. Всё чисто!", "No suspicious files found. All clean!"));
    }

    // Если это была 4-я попытка — завершаем по заданию
    if attempt >= MAX_ATTEMPTS {
        println!("\n{}", lang.t(
            "Сканирование окончательно завершено, повторите еще раз spacehound чтобы просканировать еще раз.",
            "Scanning is finally complete, run spacehound again to scan once more."
        ));
        return (false, Some(root), false);
    }

    // Кнопка «вернуться» + выбор продолжения
    println!("\n{}", lang.t("[Вернуться] — начать сначала", "[Back] — start over"));
    let again = prompt(&lang.t(
        "\nПросканировать ещё раз? (Enter=выход, 1=тот же путь, 2=другой путь, 3=вернуться): ",
        "\nScan again? (Enter=exit, 1=same path, 2=different path, 3=back): "
    ));
    match again.as_str() {
        "1" => (true, Some(root), true),
        "2" => (true, Some(root), false),
        "3" => { greeting(lang); play_animation(lang); (true, None, false) }
        _ => (false, Some(root), false),
    }
}

fn main() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        eprintln!("\n⚠️  SpaceHound: {}", info);
        default_hook(info);
    }));

    let lang = if env::args().len() == 1 {
        greeting(Lang::Ru);
        let l = choose_lang();
        greeting(l);
        play_animation(l);
        l
    } else if env::args().any(|a| a == "--ru") { Lang::Ru } else { Lang::En };

    let args: Vec<String> = env::args().collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("\n{}", lang.t(
            "Использование:\n  spacehound              интерактивный режим\n  spacehound ПУТЬ [-n N]   сканировать папку\n  spacehound ПУТЬ --ru     русский вывод",
            "Usage:\n  spacehound              interactive mode\n  spacehound PATH [-n N]   scan a folder\n  spacehound PATH --ru     Russian output"
        ));
        return;
    }

    let mut remembered: Option<String> = None;
    let mut repeat_same = false;
    let mut attempt = 1;
    loop {
        let override_path: Option<&str> = if repeat_same { remembered.as_deref() } else { None };
        let (cont, chosen, again_same) = scan_once(lang, &args, override_path, attempt);
        if let Some(p) = chosen { remembered = Some(p); }
        if !cont { break; }
        repeat_same = again_same;
        attempt += 1;
    }
}
