use crate::{
    types::{
        Elevation, TimeOffset, MAX_BRIGHTNESS, MAX_ELEVATION, MAX_GAMMA,
        MAX_LATITUDE, MAX_LONGITUDE, MAX_TEMPERATURE, MIN_BRIGHTNESS,
        MIN_ELEVATION, MIN_GAMMA, MIN_LATITUDE, MIN_LONGITUDE,
        MIN_TEMPERATURE,
    },
    Coprod,
};
use core::fmt;
use std::{
    fmt::{Display, Formatter},
    num::{ParseFloatError, ParseIntError},
};
use thiserror::Error;

#[derive(Debug, Error)]
struct VecError<E>(Vec<E>);
impl<E: Display> Display for VecError<E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Ok(for i in &self.0 {
            writeln!(f, "{i}")?;
        })
    }
}

// TypeError

#[rustfmt::skip]
#[derive(Debug, Error)]
#[error("Temperature must be between {MIN_TEMPERATURE}K and {MAX_TEMPERATURE}K")]
pub struct TypeErrorTemperature(pub u16);

#[derive(Debug, Error)]
#[error("Brightness must be between {MIN_BRIGHTNESS} and {MAX_BRIGHTNESS}")]
pub struct TypeErrorBrightness(pub f64);

#[derive(Debug, Error)]
#[error("Gamma must be between {MIN_GAMMA} and {MAX_GAMMA}")]
pub struct TypeErrorGamma(pub f64);

#[derive(Debug, Error)]
#[error("Latitude must be between {MAX_LATITUDE}° and {MIN_LATITUDE}°")]
pub struct TypeErrorLatitude(pub f64);

#[derive(Debug, Error)]
#[error("Longitude must be between {MAX_LONGITUDE}° and {MIN_LONGITUDE}°")]
pub struct TypeErrorLongitude(pub f64);

#[derive(Debug, Error)]
#[error("Elevation must be between {MAX_ELEVATION}° and {MIN_ELEVATION}°")]
pub struct TypeErrorElevation(pub f64);

#[derive(Debug, Error)]
#[error("Hour must be between 0 and 23")]
pub struct TypeErrorHour(pub u8);

#[derive(Debug, Error)]
#[error("Minute must be between 0 and 59")]
pub struct TypeErrorMinute(pub u8);

#[derive(Debug, Error)]
#[error("Alpha must be between 0.0 and 1.0")]
pub struct TypeErrorAlpha(pub f64);

#[derive(Debug, Error)]
#[error("Starting time must be earlier than ending time: {start}-{end}")]
pub struct TypeErrorTimeRange {
    pub start: TimeOffset,
    pub end: TimeOffset,
}

#[derive(Debug, Error)]
#[error("dawn.end < dusk.start")]
pub struct TypeErrorTimeRanges {
    pub dawn_end: TimeOffset,
    pub dusk_start: TimeOffset,
}

#[derive(Debug, Error)]
#[error("High transition elevation cannot be lower than the low transition elevation")]
pub struct TypeErrorElevationRange {
    pub high: Elevation,
    pub low: Elevation,
}

type TypeErrorGammaRgbT = TypeErrorGamma;
#[derive(Debug, Error)]
#[error("{0}")]
pub struct TypeErrorGammaRgb(#[from] VecError<TypeErrorGammaRgbT>);

type TypeErrorTimeT = Coprod!(TypeErrorHour, TypeErrorMinute);
#[derive(Debug, Error)]
#[error("{0}")]
pub struct TypeErrorTime(#[from] VecError<TypeErrorTimeT>);

type TypeErrorLocationT = Coprod!(TypeErrorLatitude, TypeErrorLongitude);
#[derive(Debug, Error)]
#[error("{0}")]
pub struct TypeErrorLocation(#[from] VecError<TypeErrorLocationT>);

// ParseError

#[derive(Debug, Error)]
#[error("{0}")]
pub enum ParseErrorTemperature {
    Parse(#[from] ParseIntError),
    Type(#[from] TypeErrorTemperature),
}

#[derive(Debug, Error)]
#[error("{0}")]
pub enum ParseErrorBrightness {
    Parse(#[from] ParseFloatError),
    Type(#[from] TypeErrorBrightness),
}

#[derive(Debug, Error)]
#[error("{0}")]
pub enum ParseErrorLatitude {
    Parse(#[from] ParseFloatError),
    Type(#[from] TypeErrorLatitude),
}

#[derive(Debug, Error)]
#[error("{0}")]
pub enum ParseErrorLongitude {
    Parse(#[from] ParseFloatError),
    Type(#[from] TypeErrorLongitude),
}

#[derive(Debug, Error)]
#[error("{0}")]
pub enum ParseErrorElevation {
    Parse(#[from] ParseFloatError),
    Type(#[from] TypeErrorElevation),
}

type ParseErrorLocationT = Coprod!(ParseErrorLatitude, ParseErrorLongitude);
#[derive(Debug, Error)]
#[error("{0}")]
pub enum ParseErrorLocation {
    Vec(#[from] VecError<ParseErrorLocationT>),
    #[error("")]
    Fmt,
}

pub type ParseErrorGamma = Coprod!(ParseFloatError, TypeErrorGamma);
#[derive(Debug, Error)]
#[error("{0}")]
pub enum ParseErrorGammaRgb {
    Vec(#[from] VecError<ParseErrorGamma>),
    Single(#[from] ParseErrorGamma),
    #[error("")]
    Fmt,
}

pub type ParseErrorTimeT =
    Coprod!(ParseIntError, TypeErrorHour, TypeErrorMinute);
#[derive(Debug, Error)]
#[error("{0}")]
pub enum ParseErrorTime {
    Vec(#[from] VecError<ParseErrorTimeT>),
    #[error("")]
    Fmt,
}

#[derive(Debug, Error)]
#[error("{0}")]
pub enum ParseErrorTimeRange {
    Vec(#[from] VecError<ParseErrorTime>),
    Single(#[from] ParseErrorTime),
    Type(#[from] TypeErrorTimeRange),
    #[error("")]
    Fmt,
}

#[derive(Debug, Error)]
#[error("{0}")]
pub enum ParseErrorTimeRanges {
    Vec(#[from] VecError<ParseErrorTimeRange>),
    Type(#[from] TypeErrorTimeRanges),
    #[error("")]
    Fmt,
}

#[derive(Debug, Error)]
#[error("{0}")]
pub enum ParseErrorElevationRange {
    Vec(#[from] VecError<ParseErrorElevation>),
    Type(#[from] TypeErrorElevationRange),
    #[error("")]
    Fmt,
}

pub enum ParseErrorTransitionScheme {}

//

#[derive(Debug, Error)]
#[error("WIP")]
// #[error("Unable to get location from provider")]
pub struct LocationProviderError;

#[derive(Debug, Error)]
pub enum AdjustmentMethodError {}

//

impl From<Vec<TypeErrorGammaRgbT>> for TypeErrorGammaRgb {
    fn from(v: Vec<TypeErrorGammaRgbT>) -> Self {
        Self(VecError(v))
    }
}

impl From<Vec<TypeErrorLocationT>> for TypeErrorLocation {
    fn from(v: Vec<TypeErrorLocationT>) -> Self {
        Self(VecError(v))
    }
}

impl From<Vec<TypeErrorTimeT>> for TypeErrorTime {
    fn from(v: Vec<TypeErrorTimeT>) -> Self {
        Self(VecError(v))
    }
}

impl From<Vec<ParseErrorGamma>> for ParseErrorGammaRgb {
    fn from(v: Vec<ParseErrorGamma>) -> Self {
        Self::Vec(VecError(v))
    }
}

impl From<Vec<ParseErrorLocationT>> for ParseErrorLocation {
    fn from(v: Vec<ParseErrorLocationT>) -> Self {
        Self::Vec(VecError(v))
    }
}

impl From<Vec<ParseErrorTimeT>> for ParseErrorTime {
    fn from(v: Vec<ParseErrorTimeT>) -> Self {
        Self::Vec(VecError(v))
    }
}

impl From<Vec<ParseErrorTime>> for ParseErrorTimeRange {
    fn from(v: Vec<ParseErrorTime>) -> Self {
        Self::Vec(VecError(v))
    }
}

impl From<Vec<ParseErrorTimeRange>> for ParseErrorTimeRanges {
    fn from(v: Vec<ParseErrorTimeRange>) -> Self {
        Self::Vec(VecError(v))
    }
}

impl From<Vec<ParseErrorElevation>> for ParseErrorElevationRange {
    fn from(v: Vec<ParseErrorElevation>) -> Self {
        Self::Vec(VecError(v))
    }
}
