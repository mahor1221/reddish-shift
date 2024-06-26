/*  utils.rs -- Useful functions, traits, types or macros
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

use crate::{coproduct::CoprodInjector, error::VecError};
use frunk::Generic;
use itertools::Itertools;
use std::error::Error;

pub trait IsDefault {
    fn is_default(&self) -> bool;
}
impl<T: Default + PartialEq> IsDefault for T {
    fn is_default(&self) -> bool {
        *self == T::default()
    }
}

/// Does the same thing as [frunk::from_generic]
pub trait IntoGeneric {
    fn into_generic<Dst>(self) -> Dst
    where
        Dst: Generic<Repr = Self>;
}
impl<Repr> IntoGeneric for Repr {
    fn into_generic<Dst>(self) -> Dst
    where
        Dst: Generic<Repr = Self>,
    {
        <Dst as Generic>::from(self)
    }
}

/// Copied from frunk::Coprod
/// Display and Error implementations are added for the local Coproduct
#[macro_export]
macro_rules! Coprod {
    () => { $crate::coproduct::CNil };
    (...$Rest:ty) => { $Rest };
    ($A:ty) => { $crate::Coprod![$A,] };
    ($A:ty, $($tok:tt)*) => {
        $crate::coproduct::Coproduct<$A, $crate::Coprod![$($tok)*]>
    };
}

/// Shortcut for inject on Result types
pub trait InjectErr<T, E> {
    fn inject_err<F, Index>(self) -> Result<T, F>
    where
        Self: Sized,
        F: CoprodInjector<E, Index>;
}
impl<T, E> InjectErr<T, E> for Result<T, E> {
    #[inline(always)]
    fn inject_err<F, Index>(self) -> Result<T, F>
    where
        Self: Sized,
        F: CoprodInjector<E, Index>,
    {
        self.map_err(CoprodInjector::inject)
    }
}

/// Shortcut for inject_err().map_err(..) on Result types
pub trait InjectMapErr<T, E> {
    fn inject_map_err<F1, F2, O, Index>(self, f: O) -> Result<T, F2>
    where
        Self: Sized,
        F1: CoprodInjector<E, Index>,
        O: FnOnce(F1) -> F2;
}
impl<T, E> InjectMapErr<T, E> for Result<T, E> {
    #[inline(always)]
    fn inject_map_err<F1, F2, O, Index>(self, f: O) -> Result<T, F2>
    where
        Self: Sized,
        F1: CoprodInjector<E, Index>,
        O: FnOnce(F1) -> F2,
    {
        self.map_err(|e| f(CoprodInjector::inject(e)))
    }
}

/// Shortcut for partition_result
pub trait CollectResult<T, E: Error> {
    fn collect_result(self) -> Result<Vec<T>, VecError<E>>;
}
impl<I, T, E: Error> CollectResult<T, E> for I
where
    I: Itertools<Item = Result<T, E>>,
{
    fn collect_result(self) -> Result<Vec<T>, VecError<E>> {
        let (v, e): (Vec<T>, Vec<E>) = self.partition_result();
        if e.is_empty() {
            Ok(v)
        } else {
            Err(VecError(e))
        }
    }
}
