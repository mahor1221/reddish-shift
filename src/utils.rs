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

use crate::types::Verbosity;
use std::io::{Result as IoResult, Write};

impl<O: Write, E: Write> Write for Verbosity<O, E> {
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

#[macro_export]
macro_rules! write_verbose {
    ($dst:expr, $($arg:tt)*) => {
        match $dst {
            $crate::types::Verbosity::Quite
            | $crate::types::Verbosity::Low { .. } => Ok(()),
            $crate::types::Verbosity::High { out, err: _ } => write!(out, $($arg)*),
        }
    };
}

#[macro_export]
macro_rules! writeln_verbose {
    ($dst:expr $(,)?) => {
        $crate::write_verbose!($dst, "\n")
    };
    ($dst:expr, $($arg:tt)*) => {
        match $dst {
            $crate::types::Verbosity::Quite
            | $crate::types::Verbosity::Low { .. } => Ok(()),
            $crate::types::Verbosity::High { out, err: _ } => writeln!(out, $($arg)*),
        }
    };
}

#[macro_export]
macro_rules! ewrite {
    ($dst:expr, $($arg:tt)*) => {
        match $dst {
            $crate::types::Verbosity::Quite => Ok(()),
            $crate::types::Verbosity::Low { out: _, err }
            | $crate::types::Verbosity::High { out: _, err } => write!(err, $($arg)*),
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
            $crate::types::Verbosity::Quite => Ok(()),
            $crate::types::Verbosity::Low { out: _, err }
            | $crate::types::Verbosity::High { out: _, err } => writeln!(err, $($arg)*),
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
