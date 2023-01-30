use crate::sbi::console_putchar;
use core::fmt::{self, Write};

struct StdOut;

impl Write for StdOut {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            console_putchar(c as usize);
        }
        Ok(())
    }
}

pub fn print(args: fmt::Arguments) {
    StdOut.write_fmt(args).unwrap();
}

/// print string macro
#[macro_export]
macro_rules! print {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!($fmt $(, $($arg)+)?));
    }
}

/// println string macro
#[macro_export]
macro_rules! println {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! info {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!(concat!("\x1b[1;34m[kernel] ", $fmt, "\x1b[0m\n") $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! warning {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!(concat!("\x1b[1;33m", $fmt, "\x1b[0m\n") $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! error {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!(concat!("\x1b[1;31m", $fmt, "\x1b[0m\n") $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! kernel {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!(concat!("\x1b[1;35m[kernel] ", $fmt, "\x1b[0m\n") $(, $($arg)+)?));
    }
}