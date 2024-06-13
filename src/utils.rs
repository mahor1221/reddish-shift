// anonymous enums

use std::fmt::{Display, Formatter, Result as FmtResult};

#[derive(Debug)]
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

impl<T0, T1> Display for Enum2<T0, T1>
where
    T0: Display,
    T1: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::T0(t) => t.fmt(f),
            Self::T1(t) => t.fmt(f),
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
