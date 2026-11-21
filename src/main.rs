use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::thread;

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
const IGNORE_DIRS: [&str; 8] = ["/proc", "/sys", "/dev", "/run", "/.git", "/node_modules", "/.cache/thumbnails", "/snap"];

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

fn parse_size(s: &str) -> Option<u64> {
    let s = s.trim().to_uppercase();
    let (num, mult) = if let Some(v) = s.strip_suffix("GB") { (v, 1024u64 * 1024 * 1024) }
        else if let Some(v) = s.strip_suffix("MB") { (v, 1024 * 1024) }
        else if let Some(v) = s.strip_suffix("KB") { (v, 1024) }
        else { (s.as_str(), 1) };
    num.trim().parse::<u64>().ok().map(|n| n * mult)
}

fn should_ignore(path: &Path) -> bool {
    let s = path.to_string_lossy();
    IGNORE_DIRS.iter().any(|d| s.contains(d))
}

fn collect_dirs(dir: &Path, dirs: &mut Vec<PathBuf>, errors: &AtomicUsize) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => { errors.fetch_add(1, Ordering::Relaxed); return; }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let meta = match fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(_) => { errors.fetch_add(1, Ordering::Relaxed); continue; }
        };
        if meta.file_type().is_symlink() { continue; }
        if meta.is_dir() {
            if !should_ignore(&path) {
                dirs.push(path.clone());
                collect_dirs(&path, dirs, errors);
            }
        }
    }
}

fn scan_dir(dir: &Path, tx: &mpsc::Sender<FileEntry>, errors: &AtomicUsize) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => { errors.fetch_add(1, Ordering::Relaxed); return; }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let meta = match fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(_) => { errors.fetch_add(1, Ordering::Relaxed); continue; }
        };
        if meta.file_type().is_symlink() { continue; }
        if meta.is_file() {
            let _ = tx.send(FileEntry { path, size: meta.len() });
        }
    }
}

fn parallel_scan(root: &Path) -> (Vec<FileEntry>, usize) {
    let errors = AtomicUsize::new(0);
    let mut dirs = Vec::new();
    collect_dirs(root, &mut dirs, &errors);
    if !should_ignore(root) { dirs.push(root.to_path_buf()); }

    let threads = thread::available_parallelism().map(|n| n.get()).unwrap_or(4).max(2);
    let (tx, rx) = mpsc::channel();
    let chunk_size = (dirs.len() + threads - 1) / threads;

    thread::scope(|s| {
        for chunk in dirs.chunks(chunk_size.max(1)) {
            let tx = tx.clone();
            let errors = &errors;
            s.spawn(move || {
                for dir in chunk { scan_dir(dir, &tx, errors); }
            });
        }
    });
    drop(tx);

    let files: Vec<FileEntry> = rx.into_iter().collect();
    (files, errors.load(Ordering::Relaxed))
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
    println!("\n🦫  SpaceHound 1.0 — {}", lang.t("умный поиск больших файлов", "smart large-file finder"));
    println!("{}", lang.t(
        "Параллельное сканирование, фильтры, экспорт и безопасный бэкап.\nНичего не удаляется без подтверждения.",
        "Parallel scan, filters, export and safe backup.\nNothing is deleted without confirmation."
    ));
    println!("⚠️  {}", lang.t(
        "Программа сделана ТОЛЬКО для Linux (Pop!_OS, Ubuntu, Debian и др.).",
        "This program is made ONLY for Linux (Pop!_OS, Ubuntu, Debian, etc.)."
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
    let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
    let mut dest = bdir.join(&name);
    if dest.exists() {
        let stem = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
        let ext = path.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
        let mut i = 1;
        loop {
            dest = bdir.join(format!("{}_{}{}", stem, i, ext));
            if !dest.exists() { break; }
            i += 1;
        }
    }
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

fn export_report(path: &Path, files: &[FileEntry], lang: Lang) {
    let mut content = format!("SpaceHound 1.0 — {}\n", lang.t("отчёт", "report"));
    for (i, f) in files.iter().enumerate() {
        content.push_str(&format!("{}. {} — {}\n", i + 1, f.path.display(), human_size(f.size)));
    }
    if fs::write(path, content).is_ok() {
        println!("✅ {} → {}", lang.t("Отчёт сохранён", "Report saved"), path.display());
    } else {
        println!("❌ {}", lang.t("не удалось сохранить отчёт", "could not save report"));
    }
}

fn scan_once(lang: Lang, args: &[String], path_override: Option<&str>, attempt: usize) -> (bool, Option<String>, bool, bool) {
    println!("\n{} {}/{}",
        lang.t("Попытка сканирования:", "Scan attempt:"), attempt, MAX_ATTEMPTS);

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

    let min_size = if let Some(i) = args.iter().position(|a| a == "--min") {
        args.get(i + 1).and_then(|s| parse_size(s)).unwrap_or(0)
    } else { 0 };

    let ext_filter = if let Some(i) = args.iter().position(|a| a == "--ext") {
        args.get(i + 1).cloned().unwrap_or_default().to_lowercase()
    } else { String::new() };

    let export_path = if let Some(i) = args.iter().position(|a| a == "--export") {
        args.get(i + 1).cloned()
    } else { None };

    let path = Path::new(&root);
    if !path.exists() {
        println!("❌ {}: {}", lang.t("путь не найден", "path not found"), root);
        return (true, Some(root), false, false);
    }

    println!("\n🔍 {} {} ...", lang.t("Параллельное сканирование", "Parallel scan"), root);
    let start = std::time::Instant::now();
    let (mut files, errors) = parallel_scan(path);
    let elapsed = start.elapsed();

    let total_found = files.len();
    let total_bytes: u64 = files.iter().map(|f| f.size).sum();

    files.retain(|f| f.size >= min_size);
    if !ext_filter.is_empty() {
        files.retain(|f| f.path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase() == ext_filter);
    }
    files.sort_by(|a, b| b.size.cmp(&a.size));
    let top: Vec<FileEntry> = files.iter().take(n).cloned().collect();

    let mut ext_stats: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    for f in &files {
        let ext = f.path.extension().and_then(|e| e.to_str()).unwrap_or("(нет)").to_lowercase();
        *ext_stats.entry(ext).or_insert(0) += f.size;
    }
    let mut ext_top: Vec<_> = ext_stats.into_iter().collect();
    ext_top.sort_by(|a, b| b.1.cmp(&a.1));
    ext_top.truncate(5);

    println!("\n✅ {}: {} {} {:.2?} ({} {})",
        lang.t("Найдено", "Found"), files.len(), lang.t("файлов за", "files in"), elapsed, errors, lang.t("ошибок", "errors"));
    println!("{}: {} | {}: {}", lang.t("Всего", "Total"), total_found, lang.t("Суммарно", "Sum"), human_size(total_bytes));
    println!("{}:", lang.t("Топ расширений", "Top extensions"));
    for (ext, size) in &ext_top {
        println!("   .{:<10} {}", ext, human_size(*size));
    }
    println!("\n{:<5} {:<13} {:<45} {}", "№", lang.t("РАЗМЕР", "SIZE"), lang.t("ФАЙЛ", "FILE"), lang.t("КОММЕНТАРИЙ", "NOTE"));

    let mut suspicious = Vec::new();
    for (i, f) in top.iter().enumerate() {
        let reason = junk_reason(&f.path, lang);
        if reason.is_some() { suspicious.push(f.clone()); }
        println!("{:<5} {:<13} {:<45} {}", i + 1, human_size(f.size), {
            let s = f.path.file_name().unwrap_or_default().to_string_lossy().to_string();
            if s.chars().count() > 44 {
                let truncated: String = s.chars().take(42).collect();
                format!("{}...", truncated)
            } else { s }
        }, reason.clone().unwrap_or_default());
    }

    if let Some(ep) = export_path { export_report(Path::new(&ep), &top, lang); }

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

    if attempt >= MAX_ATTEMPTS {
        println!("\n{}", lang.t(
            "Сканирование окончательно завершено, повторите еще раз spacehound чтобы просканировать еще раз.",
            "Scanning is finally complete, run spacehound again to scan once more."
        ));
        return (false, Some(root), false, false);
    }

    println!("\n{}", lang.t("[Вернуться] — начать сначала", "[Back] — start over"));
    let again = prompt(&lang.t(
        "\nПросканировать ещё раз? (Enter=выход, 1=тот же путь, 2=другой путь, 3=вернуться): ",
        "\nScan again? (Enter=exit, 1=same path, 2=different path, 3=back): "
    ));
    match again.as_str() {
        "1" => (true, Some(root), true, false),
        "2" => (true, Some(root), false, false),
        "3" => { greeting(lang); (true, None, false, true) }
        _ => (false, Some(root), false, false),
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
        l
    } else if env::args().any(|a| a == "--ru") { Lang::Ru } else { Lang::En };

    let args: Vec<String> = env::args().collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("\n{}", lang.t(
            "Использование:\n  spacehound                    интерактивный режим\n  spacehound ПУТЬ [-n N]         сканировать папку\n  spacehound ПУТЬ --min 100M     файлы от 100 МБ\n  spacehound ПУТЬ --ext mp4      только .mp4\n  spacehound ПУТЬ --export out.txt  сохранить отчёт\n  spacehound ПУТЬ --ru           русский вывод",
            "Usage:\n  spacehound                    interactive mode\n  spacehound PATH [-n N]         scan a folder\n  spacehound PATH --min 100M     files from 100 MB\n  spacehound PATH --ext mp4      only .mp4\n  spacehound PATH --export out.txt  save report\n  spacehound PATH --ru           Russian output"
        ));
        return;
    }

    let mut remembered: Option<String> = None;
    let mut repeat_same = false;
    let mut attempt = 1;
    loop {
        let override_path: Option<&str> = if repeat_same { remembered.as_deref() } else { None };
        let (cont, chosen, again_same, reset_attempts) =
            scan_once(lang, &args, override_path, attempt);
        if let Some(p) = chosen { remembered = Some(p); }
        if !cont { break; }
        repeat_same = again_same;
        if reset_attempts { attempt = 1; } else { attempt += 1; }
    }
}
