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

#[macro_export]
macro_rules! write_verbose {
    ($dst:expr, $($arg:tt)*) => {
        match $dst {
            $crate::types::Verbosity::Quite
            | $crate::types::Verbosity::Low(_) => Ok(()),
            $crate::types::Verbosity::High(w) => write!(w, $($arg)*),
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
            | $crate::types::Verbosity::Low(_) => Ok(()),
            $crate::types::Verbosity::High(w) => writeln!(w, $($arg)*),
        }
    };
}

impl<W: Write> Write for Verbosity<W> {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        match self {
            Verbosity::Quite => Ok(buf.len()),
            Verbosity::Low(w) => w.write(buf),
            Verbosity::High(w) => w.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Verbosity::Quite => Ok(()),
            Verbosity::Low(w) => w.flush(),
            Verbosity::High(w) => w.flush(),
        }
    }
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

// anonymous enums

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Enum2<T0, T1> {
    T0(T0),
    T1(T1),
}

impl<T, T0, T1> AsMut<T> for Enum2<T0, T1>
where
    T0: AsMut<T>,
    T1: AsMut<T>,
{
    fn as_mut(&mut self) -> &mut T {
        match self {
            Self::T0(t) => t.as_mut(),
            Self::T1(t) => t.as_mut(),
        }
    }
}

impl<T, T0, T1> AsRef<T> for Enum2<T0, T1>
where
    T0: AsRef<T>,
    T1: AsRef<T>,
{
    fn as_ref(&self) -> &T {
        match self {
            Self::T0(t) => t.as_ref(),
            Self::T1(t) => t.as_ref(),
        }
    }
}
