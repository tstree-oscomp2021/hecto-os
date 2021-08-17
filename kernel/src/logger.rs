use core::{fmt::Write, panic::PanicInfo};

use crate::{
    arch::{interface::Console, ConsoleImpl},
    backtrace::backtrace,
    sync::SpinLock,
};

pub mod interface {
    use fatfs::Inode;

    pub trait Console: core::fmt::Write + Inode + Send + Sync {
        const CONSOLE_INSTANCE: Self;
    }
}

static CONSOLE_LOCK: SpinLock<()> = SpinLock::new(());

pub fn _print(args: core::fmt::Arguments) {
    CONSOLE_LOCK.critical_section(|_| {
        #[allow(const_item_mutation)]
        ConsoleImpl::CONSOLE_INSTANCE.write_fmt(args).unwrap();
    });
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

/// 前景色 <https://en.wikipedia.org/wiki/ANSI_escape_code#3-bit_and_4-bit>
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

#[allow(unused)]
#[repr(usize)]
#[derive(Clone, Copy)]
pub enum Level {
    Error = 0,
    Warn,
    Info,
    Debug,
    Trace,
}

/// 根据不同日志等级得到颜色。
pub const fn level2color(level: Level) -> u8 {
    use FGColor::*;
    return match level {
        Level::Error => Red,
        Level::Warn => LightYellow,
        Level::Info => Blue,
        Level::Debug => Green,
        Level::Trace => DarkGray,
    } as u8;
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => ({
        #[cfg(any(feature = "trace", feature = "debug", feature = "info", feature = "warn", feature = "error"))]
        println!(
            "[\x1b[{}mERROR\x1b[0m {}] {}",
            $crate::logger::level2color($crate::logger::Level::Error),
            $crate::arch::cpu::get_cpu_id(),
            format_args!($($arg)*)
        );
    })
}
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => ({
        #[cfg(any(feature = "trace", feature = "debug", feature = "info", feature = "warn"))]
        println!(
            "[\x1b[{}mWARN \x1b[0m {}] {}",
            $crate::logger::level2color($crate::logger::Level::Warn),
            $crate::arch::cpu::get_cpu_id(),
            format_args!($($arg)*)
        );
    })
}
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => ({
        #[cfg(any(feature = "trace", feature = "debug", feature = "info"))]
        println!(
            "[\x1b[{}mINFO \x1b[0m {}] {}",
            $crate::logger::level2color($crate::logger::Level::Info),
            $crate::arch::cpu::get_cpu_id(),
            format_args!($($arg)*)
        );
    })
}
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => ({
        #[cfg(any(feature = "trace", feature = "debug"))]
        println!(
            "[\x1b[{}mDEBUG\x1b[0m {}] {}",
            $crate::logger::level2color($crate::logger::Level::Debug),
            $crate::arch::cpu::get_cpu_id(),
            format_args!($($arg)*)
        );
    })
}

#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => ({
        #[cfg(any(feature = "trace"))]
        println!(
            "[\x1b[{}mTRACE\x1b[0m {}] {}",
            $crate::logger::level2color($crate::logger::Level::Trace),
            $crate::arch::cpu::get_cpu_id(),
            format_args!($($arg)*)
        );
    })
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    match info.location() {
        Some(location) => {
            println!(
                "[kernel] panicked at '{}', {}:{}:{}",
                info.message().unwrap(),
                location.file(),
                location.line(),
                location.column()
            );
        }
        None => error!("[kernel] panicked at '{}'", info.message().unwrap()),
    }
    backtrace();

    #[cfg(feature = "qemu")]
    loop {}

    #[cfg(not(feature = "qemu"))]
    unsafe {
        crate::arch::cpu::shutdown()
    }
}
