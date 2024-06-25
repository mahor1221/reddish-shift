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
