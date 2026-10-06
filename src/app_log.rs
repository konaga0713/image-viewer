/// app_log
use std::fmt::Arguments;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{OnceLock, RwLock};

static LOG_DIRECTORY: OnceLock<RwLock<Option<PathBuf>>> = OnceLock::new();

fn directory() -> &'static RwLock<Option<PathBuf>> {
    LOG_DIRECTORY.get_or_init(|| RwLock::new(None))
}

pub fn configure(enabled: bool) -> Result<(), String> {
    let path = if enabled {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let parent = exe.parent().ok_or("executable directory unavailable")?;
        let path = parent.join("logs");
        fs::create_dir_all(&path).map_err(|e| e.to_string())?;
        Some(path)
    } else {
        None
    };

    *directory().write().map_err(|e| e.to_string())? = path;
    Ok(())
}

pub fn error(args: Arguments<'_>) {
    write_line("ERROR", args);
}

pub fn info(args: Arguments<'_>) {
    write_line("INFO", args);
}

#[macro_export]
macro_rules! eprintln {
    ($($arg:tt)*) => {
        $crate::app_log::error(format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {
        $crate::app_log::info(format_args!($($arg)*));
    };
}

fn write_line(level: &str, args: Arguments<'_>) {
    let Ok(directory) = directory().read() else {
        return;
    };
    let Some(directory) = directory.as_ref() else {
        let mut output: Box<dyn Write> = if level == "ERROR" {
            Box::new(std::io::stderr())
        } else {
            Box::new(std::io::stdout())
        };
        let _ = writeln!(output, "[{}] {}", level, args);
        return;
    };

    let now = chrono::Local::now();
    let path = directory.join(format!("{}.log", now.format("%Y%m")));

    let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path) else {
            return;
        };

    let _ = writeln!(file, "{} [{}] {}", now.format("%Y-%m-%d %H:%M:%S"), level, args);
}