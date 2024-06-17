/*  utils.rs -- Useful types and functions
    This file is part of <https://github.com/mahor1221/reddish-shift>.
    Copyright (C) 2024 Mahor Foruzesh <mahor1221@gmail.com>

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

use anstream::{
    stream::{AsLockedWrite, RawStream},
    AutoStream,
};
use anstyle::{AnsiColor, Color, Style};
use std::io::{Result as IoResult, Write as IoWrite};

pub const WARN: Style = Style::new()
    .bold()
    .fg_color(Some(Color::Ansi(AnsiColor::Yellow)));
pub const HEADER: Style = Style::new().bold().underline();
pub const BODY: Style = Style::new().bold();

/// For [AutoStream] to implement [std::io::Write],
/// [RawStream] and [AsLockedWrite] are required
/// This is just a trait alias for RawStream + AsLockedWrite
pub trait Write: RawStream + AsLockedWrite {}
impl<T: RawStream + AsLockedWrite> Write for T {}

/// Useful type that can be used with these macros:
/// - write!(..) & writeln!(..):
///   write to output when verbosity is not quite
/// - vwrite!(..) && vwriteln!(..):
///   write to output when verbosiry is high
/// - ewrite!(..) & ewriteln!(..)
///   write to error when verbosity is not quite
///
/// output and error can be [std::io::stdout] and [std::io::stderr],
/// or a Rc<RefCell<[String]>> for testing purposes
#[derive(Debug)]
pub enum Verbosity<O: Write, E: Write> {
    Quite,
    Low {
        out: AutoStream<O>,
        err: AutoStream<E>,
    },
    High {
        out: AutoStream<O>,
        err: AutoStream<E>,
    },
}

// write!(..) & writeln!(..)
impl<O: Write, E: Write> IoWrite for Verbosity<O, E> {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        match self {
            Verbosity::Quite => Ok(buf.len()),
            Verbosity::Low { out, err: _ } => out.write(buf),
            Verbosity::High { out, err: _ } => out.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Verbosity::Quite => Ok(()),
            Verbosity::Low { out, err: _ } => out.flush(),
            Verbosity::High { out, err: _ } => out.flush(),
        }
    }
}

// vwrite!(..) && vwriteln!(..)

#[macro_export]
macro_rules! vwrite {
    ($dst:expr, $($arg:tt)*) => {
        match $dst {
            Verbosity::Quite | Verbosity::Low { .. } => Ok(()),
            Verbosity::High { out, err: _ } => write!(out, $($arg)*),
        }
    };
}

#[macro_export]
macro_rules! vwriteln {
    ($dst:expr $(,)?) => {
        $crate::write_verbose!($dst, "\n")
    };
    ($dst:expr, $($arg:tt)*) => {
        match $dst {
            Verbosity::Quite | Verbosity::Low { .. } => Ok(()),
            Verbosity::High { out, err: _ } => writeln!(out, $($arg)*),
        }
    };
}

// ewrite!(..) & ewriteln!(..)

#[macro_export]
macro_rules! ewrite {
    ($dst:expr, $($arg:tt)*) => {
        match $dst {
            Verbosity::Quite => Ok(()),
            Verbosity::Low { out: _, err }
            | Verbosity::High { out: _, err } => write!(err, $($arg)*),
        }
    };
}

#[macro_export]
macro_rules! ewriteln {
    ($dst:expr $(,)?) => {
        $crate::ewrite!($dst, "\n")
    };
    ($dst:expr, $($arg:tt)*) => {
        match $dst {
            Verbosity::Quite => Ok(()),
            Verbosity::Low { out: _, err }
            | Verbosity::High { out: _, err } => writeln!(err, $($arg)*),
        }
    };
}

// is_default

pub trait IsDefault {
    fn is_default(&self) -> bool;
}

impl<T: Default + PartialEq> IsDefault for T {
    fn is_default(&self) -> bool {
        *self == T::default()
    }
}
