use crate::Coprod;
use core::fmt;
use std::{
    error::Error,
    fmt::{Display, Formatter},
    io,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub struct VecError<E>(pub Vec<E>);
impl<E: Display> Display for VecError<E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for i in &self.0 {
            writeln!(f, "{i}")?;
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("WIP")]
    WIP,
    #[error("{0}")]
    File(#[from] ConfigFileError),
}

#[derive(Debug, Error)]
pub enum ConfigFileError {
    #[error("")]
    PathNotFile,
    #[error("")]
    ConfigDirNotFound,
    #[error("{0}")]
    OpenFailed(io::Error),
    #[error("{0}")]
    DeserializeFailed(toml::de::Error),
}

#[derive(Debug, Error)]
#[error("WIP")]
// #[error("Unable to get location from provider")]
pub struct ProviderError;

type AdjusterErrorInnerRandr = Coprod!(
    VecError<x11rb::errors::ConnectionError>,
    VecError<x11rb::errors::ReplyError>
);

#[derive(Debug, Error)]
pub enum AdjusterErrorInner {
    #[error("")]
    Dummy,
    #[error("{0}")]
    Drm(VecError<io::Error>),
    #[error("{0}")]
    Randr(AdjusterErrorInnerRandr),
}

#[derive(Debug, Error)]
pub enum AdjusterError {
    #[error("Temperature adjustment failed:\n{0}")]
    Set(AdjusterErrorInner),
    #[error("Unable to restore CRTC %i:\n{0}")]
    Restore(AdjusterErrorInner),
}

pub mod gamma {
    use super::*;

    #[derive(Debug, Error)]
    pub enum DrmError {
        #[error("Failed to open DRM device: %s")]
        OpenDeviceFailed(io::Error),
        #[error("{0}")]
        GetResourcesFailed(io::Error),
        #[error("Crtc numbers must be non zero")]
        ZeroValueCrtc,
        #[error("Crtc numbers must be unique")]
        NonUniqueCrtc,
        #[error("Valid CRTCs are: {0:?}")]
        InvalidCrtc(Vec<u32>),
        #[error("{0}")]
        Crtcs(VecError<DrmErrorCrtc>),
    }

    #[derive(Debug, Error)]
    pub enum DrmErrorCrtc {
        #[error("{0}")]
        GetRampSizeFailed(io::Error),
        #[error("Gamma ramp size too small: {0}")]
        InvalidRampSize(u32),
        #[error("{0}")]
        GetRampFailed(io::Error),
    }

    //

    pub type RandrReplyError =
        Coprod!(x11rb::errors::ConnectionError, x11rb::errors::ReplyError);

    #[derive(Debug, Error)]
    pub enum RandrError {
        #[error("{0}")]
        ConnectFailed(x11rb::errors::ConnectError),
        #[error("{0}")]
        GetVersionFailed(RandrReplyError),
        #[error("{0}")]
        GetResourcesFailed(RandrReplyError),
        #[error("Unsupported RANDR version ({major}.{minor})")]
        UnsupportedVersion { major: u32, minor: u32 },
        #[error("Crtc numbers must be unique")]
        NonUniqueCrtc,
        #[error("Valid CRTCs are: {0:?}")]
        InvalidCrtc(Vec<u32>),
        #[error("{0}")]
        SendRequestFailed(VecError<x11rb::errors::ConnectionError>),
        #[error("{0}")]
        Crtcs(#[from] VecError<RandrErrorCrtc>),
    }

    #[derive(Debug, Error)]
    pub enum RandrErrorCrtc {
        #[error("{0}")]
        GetRampSizeFailed(x11rb::errors::ReplyError),
        #[error("Gamma ramp size too small: {0}")]
        InvalidRampSize(u16),
        #[error("{0}")]
        GetRampFailed(x11rb::errors::ReplyError),
    }
}

pub mod types {
    use super::*;
    use crate::types::{
        Elevation, TimeOffset, MAX_BRIGHTNESS, MAX_ELEVATION, MAX_GAMMA,
        MAX_LATITUDE, MAX_LONGITUDE, MAX_TEMPERATURE, MIN_BRIGHTNESS,
        MIN_ELEVATION, MIN_GAMMA, MIN_LATITUDE, MIN_LONGITUDE,
        MIN_TEMPERATURE,
    };

    #[derive(Debug, Error)]
    #[error("Temperature must be between {MIN_TEMPERATURE}K and {MAX_TEMPERATURE}K")]
    pub struct TemperatureError(pub u16);

    #[rustfmt::skip]
    #[derive(Debug, Error)]
    #[error("Brightness must be between {MIN_BRIGHTNESS} and {MAX_BRIGHTNESS}")]
    pub struct BrightnessError(pub f64);

    #[derive(Debug, Error)]
    #[error("Gamma must be between {MIN_GAMMA} and {MAX_GAMMA}")]
    pub struct GammaError(pub f64);

    #[derive(Debug, Error)]
    #[error("Latitude must be between {MAX_LATITUDE}° and {MIN_LATITUDE}°")]
    pub struct LatitudeError(pub f64);

    #[derive(Debug, Error)]
    #[error("Longitude must be between {MAX_LONGITUDE}° and {MIN_LONGITUDE}°")]
    pub struct LongitudeError(pub f64);

    #[derive(Debug, Error)]
    #[error("Elevation must be between {MAX_ELEVATION}° and {MIN_ELEVATION}°")]
    pub struct ElevationError(pub f64);

    #[derive(Debug, Error)]
    #[error("Hour must be between 0 and 23")]
    pub struct HourError(pub u8);

    #[derive(Debug, Error)]
    #[error("Minute must be between 0 and 59")]
    pub struct MinuteError(pub u8);

    #[derive(Debug, Error)]
    #[error("Alpha must be between 0.0 and 1.0")]
    pub struct AlphaError(pub f64);

    #[derive(Debug, Error)]
    #[error("Starting time must be earlier than ending time: {start}-{end}")]
    pub struct TimeRangeError {
        pub start: TimeOffset,
        pub end: TimeOffset,
    }

    #[derive(Debug, Error)]
    #[error("dawn.end < dusk.start")]
    pub struct TimeRangesError {
        pub dawn_end: TimeOffset,
        pub dusk_start: TimeOffset,
    }

    #[derive(Debug, Error)]
    #[error("High transition elevation cannot be lower than the low transition elevation")]
    pub struct ElevationRangeError {
        pub high: Elevation,
        pub low: Elevation,
    }

    #[derive(Debug, Error)]
    #[error("{0}")]
    pub struct GammaRgbError(#[from] VecError<GammaError>);

    type TimeErrorT = Coprod!(HourError, MinuteError);
    #[derive(Debug, Error)]
    #[error("{0}")]
    pub struct TimeError(#[from] VecError<TimeErrorT>);

    type LocationT = Coprod!(LatitudeError, LongitudeError);
    #[derive(Debug, Error)]
    #[error("{0}")]
    pub struct LocationError(#[from] VecError<LocationT>);

    impl From<Vec<GammaError>> for GammaRgbError {
        fn from(v: Vec<GammaError>) -> Self {
            Self(VecError(v))
        }
    }

    impl From<Vec<LocationT>> for LocationError {
        fn from(v: Vec<LocationT>) -> Self {
            Self(VecError(v))
        }
    }

    impl From<Vec<TimeErrorT>> for TimeError {
        fn from(v: Vec<TimeErrorT>) -> Self {
            Self(VecError(v))
        }
    }
}

pub mod parse {
    use super::*;
    use std::num::{ParseFloatError, ParseIntError};

    pub trait DayNightErrorType: Error {}

    #[derive(Debug, Error)]
    #[error("")]
    pub enum DayNightError<E: DayNightErrorType> {
        Multiple(#[from] VecError<E>),
        Single(#[from] E),
        #[error("")]
        Fmt,
    }

    #[derive(Debug, Error)]
    #[error("{0}")]
    pub enum TemperatureError {
        Parse(#[from] ParseIntError),
        Type(#[from] types::TemperatureError),
    }
    impl DayNightErrorType for TemperatureError {}

    #[derive(Debug, Error)]
    #[error("{0}")]
    pub enum BrightnessError {
        Parse(#[from] ParseFloatError),
        Type(#[from] types::BrightnessError),
    }
    impl DayNightErrorType for BrightnessError {}

    pub type GammaErrorT = Coprod!(ParseFloatError, types::GammaError);
    #[derive(Debug, Error)]
    #[error("{0}")]
    pub enum GammaError {
        Multiple(#[from] VecError<GammaErrorT>),
        Single(#[from] GammaErrorT),
        #[error("")]
        Fmt,
    }
    impl DayNightErrorType for GammaError {}

    #[derive(Debug, Error)]
    #[error("{0}")]
    pub enum LatitudeError {
        Parse(#[from] ParseFloatError),
        Type(#[from] types::LatitudeError),
    }

    #[derive(Debug, Error)]
    #[error("{0}")]
    pub enum LongitudeError {
        Parse(#[from] ParseFloatError),
        Type(#[from] types::LongitudeError),
    }

    type LocationErrorT = Coprod!(LatitudeError, LongitudeError);
    #[derive(Debug, Error)]
    #[error("{0}")]
    pub enum LocationError {
        Multiple(#[from] VecError<LocationErrorT>),
        #[error("")]
        Fmt,
    }

    #[derive(Debug, Error)]
    #[error("{0}")]
    pub enum ElevationError {
        Parse(#[from] ParseFloatError),
        Type(#[from] types::ElevationError),
    }

    pub type TimeErrorT =
        Coprod!(ParseIntError, types::HourError, types::MinuteError);
    #[derive(Debug, Error)]
    #[error("{0}")]
    pub enum TimeError {
        Multiple(#[from] VecError<TimeErrorT>),
        #[error("")]
        Fmt,
    }

    #[derive(Debug, Error)]
    #[error("{0}")]
    pub enum TimeRangeError {
        Multiple(#[from] VecError<TimeError>),
        Single(#[from] TimeError),
        Type(#[from] types::TimeRangeError),
        #[error("")]
        Fmt,
    }

    #[derive(Debug, Error)]
    #[error("{0}")]
    pub enum TimeRangesError {
        Multiple(#[from] VecError<TimeRangeError>),
        Type(#[from] types::TimeRangesError),
        #[error("")]
        Fmt,
    }

    #[derive(Debug, Error)]
    #[error("{0}")]
    pub enum ElevationRangeError {
        Multiple(#[from] VecError<ElevationError>),
        Type(#[from] types::ElevationRangeError),
        #[error("")]
        Fmt,
    }

    #[derive(Debug, Error)]
    #[error("{time}\n{elev}")]
    pub struct TransitionSchemeError {
        pub time: TimeRangesError,
        pub elev: ElevationRangeError,
    }

    #[derive(Debug, Error)]
    #[error("{loc}")]
    pub struct LocationProviderError {
        pub loc: LocationError,
    }

    #[derive(Debug, Error)]
    pub enum AdjustmentMethodTypeParamError {
        #[error("")]
        Kind,
        #[error("")]
        Display(#[from] ParseIntError),
        #[error("")]
        Crtcs(#[from] VecError<ParseIntError>),
    }

    #[derive(Debug, Error)]
    #[error("{0}")]
    pub enum AdjustmentMethodTypeError {
        Vec(#[from] VecError<AdjustmentMethodTypeParamError>),
        #[error("")]
        Fmt,
        #[error("")]
        CrtcOnVidmode,
    }

    impl<E: DayNightErrorType> From<Vec<E>> for DayNightError<E> {
        fn from(v: Vec<E>) -> Self {
            Self::Multiple(VecError(v))
        }
    }

    impl From<Vec<GammaErrorT>> for GammaError {
        fn from(v: Vec<GammaErrorT>) -> Self {
            Self::Multiple(VecError(v))
        }
    }

    impl From<Vec<LocationErrorT>> for LocationError {
        fn from(v: Vec<LocationErrorT>) -> Self {
            Self::Multiple(VecError(v))
        }
    }

    impl From<Vec<TimeErrorT>> for TimeError {
        fn from(v: Vec<TimeErrorT>) -> Self {
            Self::Multiple(VecError(v))
        }
    }

    impl From<Vec<TimeError>> for TimeRangeError {
        fn from(v: Vec<TimeError>) -> Self {
            Self::Multiple(VecError(v))
        }
    }

    impl From<Vec<TimeRangeError>> for TimeRangesError {
        fn from(v: Vec<TimeRangeError>) -> Self {
            Self::Multiple(VecError(v))
        }
    }

    impl From<Vec<ElevationError>> for ElevationRangeError {
        fn from(v: Vec<ElevationError>) -> Self {
            Self::Multiple(VecError(v))
        }
    }

    impl From<Vec<ParseIntError>> for AdjustmentMethodTypeParamError {
        fn from(v: Vec<ParseIntError>) -> Self {
            Self::Crtcs(VecError(v))
        }
    }

    impl From<Vec<AdjustmentMethodTypeParamError>> for AdjustmentMethodTypeError {
        fn from(v: Vec<AdjustmentMethodTypeParamError>) -> Self {
            Self::Vec(VecError(v))
        }
    }
}
