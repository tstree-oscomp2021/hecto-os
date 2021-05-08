use core::{fmt::Write, panic::PanicInfo};

use crate::{
    backtrace::backtrace,
    hart::get_hart_id,
    sbi::{console_putchar, shutdown},
};

use lazy_static::lazy_static;
use log::{Level, LevelFilter, Log, Metadata, Record};
use spin::Mutex;

struct Stdout;

impl Write for Stdout {
    /// 打印一个字符串
    ///
    /// [`console_putchar`] sbi 调用每次接受一个 `usize`，但实际上会把它作为 `u8` 来打印字符。
    /// 因此，如果字符串中存在非 ASCII 字符，需要在 utf-8 编码下，对于每一个 `u8` 调用一次 [`console_putchar`]
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        s.bytes().for_each(|c| console_putchar(c as usize));
        Ok(())
    }
}

lazy_static! {
    /// 给 STDOUT 上锁
    static ref STDOUT: Mutex<Stdout> = Mutex::new(Stdout);
}

pub fn _print(args: core::fmt::Arguments) {
    STDOUT.lock().write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::logger::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ({
        $crate::logger::_print(format_args_nl!($($arg)*));
    })
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    match info.location() {
        Some(location) => {
            log::error!(
                "[kernel] panicked at '{}', {}:{}:{}",
                info.message().unwrap(),
                location.file(),
                location.line(),
                location.column()
            );
        }
        None => log::error!("[kernel] panicked at '{}'", info.message().unwrap()),
    }
    backtrace();

    shutdown()
}

struct EnvLogger;

impl Log for EnvLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        // {:<5} 表示左对齐占 5 格
        // \x1b[31m 表示其之后的前景色都为红，背景色不变。\x1b[0m 表示之后的都重置
        println!(
            "[\x1b[{}m{:<5}\x1b[0m {}] {}",
            level2color(record.level()),
            record.level(),
            get_hart_id(),
            record.args()
        );
    }

    fn flush(&self) {}
}

/// 前景色 https://en.wikipedia.org/wiki/ANSI_escape_code#3-bit_and_4-bit
#[allow(unused)]
#[repr(u8)]
enum FGColor {
    Default = 39,
    Black = 30,
    Red = 31,
    Green = 32,
    Yellow = 33,
    Blue = 34,
    Magenta = 35,
    Cyan = 36,
    LightGray = 37,
    DarkGray = 90,
    LightRed = 91,
    LightGreen = 92,
    LightYellow = 93,
    LightBlue = 94,
    LightMagenta = 95,
    LightCyan = 96,
    White = 97,
}

/// 根据不同日志等级得到颜色。
fn level2color(level: Level) -> u8 {
    use FGColor::*;
    return match level {
        Level::Error => Red,
        Level::Warn => LightYellow,
        Level::Info => Blue,
        Level::Debug => Green,
        Level::Trace => DarkGray,
    } as u8;
}

/// 注意，如果 bss 段在此之后清楚，请确保 logger 初始化时不会使用 bss 段的变量
pub fn init() {
    static LOGGER: EnvLogger = EnvLogger;
    log::set_logger(&LOGGER).unwrap();
    // 根据环境变量 LOG 的值来选择 LevelFilter
    log::set_max_level(match option_env!("LOG") {
        Some("error") => LevelFilter::Error,
        Some("warn") => LevelFilter::Warn,
        Some("info") => LevelFilter::Info,
        Some("debug") => LevelFilter::Debug,
        Some("trace") => LevelFilter::Trace,
        _ => LevelFilter::Off,
    });
}
