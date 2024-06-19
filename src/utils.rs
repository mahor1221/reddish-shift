use frunk::Generic;
use itertools::Itertools;

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
/// Display and Error implementation is added for the local Coproduct
#[macro_export]
macro_rules! Coprod {
    () => { $crate::coproduct::CNil };
    (...$Rest:ty) => { $Rest };
    ($A:ty) => { $crate::Coprod![$A,] };
    ($A:ty, $($tok:tt)*) => {
        $crate::coproduct::Coproduct<$A, $crate::Coprod![$($tok)*]>
    };
}

// /// Shortcut for inject_err().map_err(..) on Result types
// pub trait InjectMapErr<T, E> {
//     fn inject_map_err<F1, F2, O, Index>(self, f: O) -> Result<T, F2>
//     where
//         Self: Sized,
//         F1: CoprodInjector<E, Index>,
//         O: FnOnce(F1) -> F2;
// }
// impl<T, E> InjectMapErr<T, E> for Result<T, E> {
//     #[inline(always)]
//     fn inject_map_err<F1, F2, O, Index>(self, f: O) -> Result<T, F2>
//     where
//         Self: Sized,
//         F1: CoprodInjector<E, Index>,
//         O: FnOnce(F1) -> F2,
//     {
//         self.map_err(|e| f(CoprodInjector::inject(e)))
//     }
// }

pub trait CollectResult<T, E> {
    fn collect_result(self) -> Result<Vec<T>, Vec<E>>;
}
impl<I, T, E> CollectResult<T, E> for I
where
    I: Itertools<Item = Result<T, E>>,
{
    fn collect_result(self) -> Result<Vec<T>, Vec<E>> {
        let (v, e): (Vec<T>, Vec<E>) = self.partition_result();
        if e.is_empty() {
            Ok(v)
        } else {
            Err(e)
        }
    }
}
