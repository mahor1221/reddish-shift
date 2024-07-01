/*  errors.rs -- Errors
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

use crate::{types_display::ERR, Coprod};
use config::ConfigError;
use gamma::Win32GdiError;
use itertools::Itertools;
use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    io,
    path::PathBuf,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub struct VecError<E: Error>(pub Vec<E>);
impl<E: Error> Display for VecError<E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let err =
            |e: &E| "- ".to_string() + &e.to_string().lines().join("\n  ");
        let s = self.0.iter().map(err).join("\n");
        f.write_str(&s)
    }
}

impl<E: Error> Default for VecError<E> {
    fn default() -> Self {
        Self(vec![])
    }
}

impl<E: Error> VecError<E> {
    pub fn push(mut self, e: E) -> Self {
        self.0.push(e);
        self
    }

    pub fn into_push<F: Into<E>>(self, f: F) -> Self {
        self.push(f.into())
    }
}

//

#[derive(Debug)]
pub struct ReddishError(ReddishErrorKind);

impl Display for ReddishError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let sep = " ".repeat("error: ".len()) + "\n";
        let s = format!("{ERR}error:{ERR:#} {}", self.0).lines().join(&sep);
        f.write_str(&s)
    }
}

//

#[derive(Debug, Error)]
pub enum ReddishErrorKind {
    #[error("configuration failed:\n{0}")]
    Config(#[from] ConfigError),
    #[error("screen adjustment failed:\n{0}")]
    Adjuster(#[from] AdjusterError),
    #[error("failed to retrieve location:\n{0}")]
    Provider(#[from] ProviderError),
    #[error("failed to set CTRL-C handler:\n{0}")]
    Ctrlc(#[from] ctrlc::Error),
    #[error("failed to handle CTRL-C:\n{0}")]
    Mpsc(#[from] std::sync::mpsc::RecvTimeoutError),
}

#[derive(Debug, Error)]
#[error("WIP")]
// #[error("Unable to get location from provider")]
pub struct ProviderError;

#[derive(Debug, Error)]
pub enum AdjusterError {
    #[error("set gamma ramps:\n{0}")]
    Set(AdjusterErrorInner),
    #[error("restore gamma ramps:\n{0}")]
    Restore(AdjusterErrorInner),
}

#[derive(Debug, Error)]
pub enum AdjusterErrorInner {
    #[error("vidmode:\n{0}")]
    Vidmode(#[from] X11Error),
    #[error("randr:\n{0}")]
    Randr(#[from] RandrError),
    #[error("drm:\n{0}")]
    Drm(#[from] VecError<io::Error>),
    #[error("win32gdi:\n{0}")]
    Win32Gdi(#[from] Win32GdiError),
}

type X11Error =
    Coprod!(x11rb::errors::ConnectionError, x11rb::errors::ReplyError);

type RandrError = Coprod!(
    VecError<x11rb::errors::ConnectionError>,
    VecError<x11rb::errors::ReplyError>
);

pub mod config {
    use super::*;
    use gamma::{AdjustmentMethodError, DrmError, RandrError, VidmodeError};

    #[derive(Debug, Error)]
    pub enum ConfigError {
        #[error("none of the available methods worked:\n{0}")]
        NoAvailableMethod(VecError<AdjustmentMethodError>),
        #[error("adjustment method initialization:\n{0}")]
        MethodInit(#[from] AdjustmentMethodError),
        #[error("{0}")]
        File(#[from] ConfigFileError),
    }

    #[derive(Debug, Error)]
    pub enum ConfigFileError {
        #[error("given path is not a file ({0})")]
        PathNotFile(PathBuf),
        #[error("unable to find configuration file. Use the -c flag.")]
        ConfigDirNotFound,
        #[error("unable to read file ({1}):\n{0}")]
        OpenFailed(io::Error, PathBuf),
        #[error("unable to deserialize file ({1}):\n{0}")]
        DeserializeFailed(toml::de::Error, PathBuf),
    }

    impl From<DrmError> for ConfigError {
        fn from(e: DrmError) -> Self {
            Self::MethodInit(AdjustmentMethodError::Drm(e))
        }
    }

    impl From<RandrError> for ConfigError {
        fn from(e: RandrError) -> Self {
            Self::MethodInit(AdjustmentMethodError::Randr(e))
        }
    }

    impl From<VidmodeError> for ConfigError {
        fn from(e: VidmodeError) -> Self {
            Self::MethodInit(AdjustmentMethodError::Vidmode(e))
        }
    }

    impl From<Win32GdiError> for ConfigError {
        fn from(e: Win32GdiError) -> Self {
            Self::MethodInit(AdjustmentMethodError::Win32Gdi(e))
        }
    }
}

pub mod gamma {
    use std::path::PathBuf;

    use super::*;

    #[derive(Debug, Error)]
    pub enum AdjustmentMethodError {
        #[error("vidmode:\n{0}")]
        Vidmode(#[from] VidmodeError),
        #[error("randr:\n{0}")]
        Randr(#[from] RandrError),
        #[error("drm:\n{0}")]
        Drm(#[from] DrmError),
        #[error("drm:\n{0}")]
        Win32Gdi(#[from] Win32GdiError),
    }

    #[derive(Debug, Error)]
    #[error("id ({id}):\n{err}")]
    pub struct CrtcError<ID: Display, E: Error> {
        pub id: ID,
        pub err: E,
    }

    //

    #[derive(Debug, Error)]
    pub enum VidmodeError {
        #[error("connection failed:\n{0}")]
        ConnectFailed(#[from] x11rb::errors::ConnectError),
        #[error("unable to get version:\n{0}")]
        GetVersionFailed(X11Error),
        #[error("unable to get gamma ramp:\n{0}")]
        GetRampFailed(X11Error),
        #[error("unable to get gamma ramp size:\n{0}")]
        GetRampSizeFailed(X11Error),
        #[error("gamma ramp size too small: {0}")]
        InvalidRampSize(u16),
    }

    //

    #[derive(Debug, Error)]
    pub enum RandrError {
        #[error("connection failed:\n{0}")]
        ConnectFailed(#[from] x11rb::errors::ConnectError),
        #[error("unable to get version:\n{0}")]
        GetVersionFailed(X11Error),
        #[error("unable to get resources:\n{0}")]
        GetResourcesFailed(X11Error),
        #[error("unsupported version ({major}.{minor})")]
        UnsupportedVersion { major: u32, minor: u32 },
        #[error("crtc numbers must be unique")]
        NonUniqueCrtc,
        #[error("valid crtcs are: {0:?}")]
        InvalidCrtc(Vec<u32>),
        #[error("unable to send requests:\n{0}")]
        SendRequestFailed(VecError<x11rb::errors::ConnectionError>),
        #[error("crtc:\n{0}")]
        GetCrtcs(#[from] VecError<CrtcError<u32, RandrCrtcError>>),
    }

    #[derive(Debug, Error)]
    pub enum RandrCrtcError {
        #[error("unable to get gamma ramp:\n{0}")]
        GetRampFailed(x11rb::errors::ReplyError),
        #[error("unable to get gamma ramp size:\n{0}")]
        GetRampSizeFailed(x11rb::errors::ReplyError),
        #[error("gamma ramp size too small: {0}")]
        InvalidRampSize(u16),
    }

    //

    #[derive(Debug, Error)]
    pub enum DrmError {
        #[error("failed to open device ({1}):\n{0}")]
        OpenDeviceFailed(io::Error, PathBuf),
        #[error("unable to get resources:\n{0}")]
        GetResourcesFailed(io::Error),
        #[error("crtc numbers must be non zero")]
        ZeroValueCrtc,
        #[error("crtc numbers must be unique")]
        NonUniqueCrtc,
        #[error("valid crtcs are: {0:?}")]
        InvalidCrtc(Vec<u32>),
        #[error("crtc:\n{0}")]
        GetCrtcs(VecError<CrtcError<u32, DrmCrtcError>>),
    }

    #[derive(Debug, Error)]
    pub enum DrmCrtcError {
        #[error("unable to get gamma ramp:\n{0}")]
        GetRampFailed(io::Error),
        #[error("unable to get gamma ramp size:\n{0}")]
        GetRampSizeFailed(io::Error),
        #[error("gamma ramp size too small: {0}")]
        InvalidRampSize(u32),
    }

    //

    #[derive(Debug, Error)]
    pub enum Win32GdiError {
        #[error("Unable to open device context")]
        GetDCFailed,
        #[error("Display device does not support gamma ramps")]
        NotSupported,
        #[error("unable to get gamma ramp")]
        GetRampFailed,
        #[error("unable to set gamma ramp")]
        SetRampFailed,
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
    #[error("temperature must be between {MIN_TEMPERATURE}K and {MAX_TEMPERATURE}K ({0})")]
    pub struct TemperatureError(pub u16);

    #[rustfmt::skip]
    #[derive(Debug, Error)]
    #[error("brightness must be between {MIN_BRIGHTNESS} and {MAX_BRIGHTNESS} ({0})")]
    pub struct BrightnessError(pub f64);

    #[derive(Debug, Error)]
    #[error("gamma must be between {MIN_GAMMA} and {MAX_GAMMA} ({0})")]
    pub struct GammaError(pub f64);

    #[derive(Debug, Error)]
    #[error(
        "latitude must be between {MAX_LATITUDE}° and {MIN_LATITUDE}° ({0})"
    )]
    pub struct LatitudeError(pub f64);

    #[derive(Debug, Error)]
    #[error("longitude must be between {MAX_LONGITUDE}° and {MIN_LONGITUDE}° ({0})")]
    pub struct LongitudeError(pub f64);

    #[derive(Debug, Error)]
    #[error("elevation must be between {MAX_ELEVATION}° and {MIN_ELEVATION}° ({0})")]
    pub struct ElevationError(pub f64);

    #[derive(Debug, Error)]
    #[error("hour must be between 0 and 23 ({0})")]
    pub struct HourError(pub u8);

    #[derive(Debug, Error)]
    #[error("minute must be between 0 and 59 ({0})")]
    pub struct MinuteError(pub u8);

    #[derive(Debug, Error)]
    #[error("alpha must be between 0.0 and 1.0 ({0})")]
    pub struct AlphaError(pub f64);

    #[derive(Debug, Error)]
    #[error("starting time must be earlier than ending time (start: {start}, end: {end})")]
    pub struct TimeRangeError {
        pub start: TimeOffset,
        pub end: TimeOffset,
    }

    #[derive(Debug, Error)]
    #[error("dawn's ending time must be earlier than dusk's starting time (dawn's end: {dawn_end}, dusk's start: {dusk_start})")]
    pub struct TimeRangesError {
        pub dawn_end: TimeOffset,
        pub dusk_start: TimeOffset,
    }

    #[derive(Debug, Error)]
    #[error("high transition elevation must be higher than the low transition elevation (high: {high}, low: {low})")]
    pub struct ElevationRangeError {
        pub high: Elevation,
        pub low: Elevation,
    }

    #[derive(Debug, Error)]
    #[error("gamma:\n{0}")]
    pub struct GammaRgbError(#[from] VecError<GammaError>);

    type TimeErrorT = Coprod!(HourError, MinuteError);
    #[derive(Debug, Error)]
    #[error("time:\n{0}")]
    pub struct TimeError(#[from] VecError<TimeErrorT>);

    type LocationT = Coprod!(LatitudeError, LongitudeError);
    #[derive(Debug, Error)]
    #[error("location:\n{0}")]
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
    use gamma::CrtcError;
    use std::num::{ParseFloatError, ParseIntError};

    pub trait DayNightErrorType: Error {}

    #[derive(Debug, Error)]
    pub enum DayNightError<E: DayNightErrorType> {
        #[error("{0}")]
        Multiple(#[from] VecError<E>),
        #[error("- {0}")]
        Single(#[from] E),
        #[error("- invalid format")]
        Fmt,
    }

    #[derive(Debug, Error)]
    pub enum TemperatureError {
        #[error("{0} ({1})")]
        Parse(ParseIntError, String),
        #[error("{0}")]
        Type(#[from] types::TemperatureError),
    }
    impl DayNightErrorType for TemperatureError {}

    #[derive(Debug, Error)]
    pub enum BrightnessError {
        #[error("{0} ({1})")]
        Parse(ParseFloatError, String),
        #[error("{0}")]
        Type(#[from] types::BrightnessError),
    }
    impl DayNightErrorType for BrightnessError {}

    pub type GammaErrorT = Coprod!(ParseFloatError, types::GammaError);
    #[derive(Debug, Error)]
    pub enum GammaError {
        #[error("{0}")]
        Multiple(#[from] VecError<GammaErrorT>),
        #[error("- {0}")]
        Single(#[from] GammaErrorT),
        #[error("- invalid format")]
        Fmt,
    }
    impl DayNightErrorType for GammaError {}

    #[derive(Debug, Error)]
    pub enum LatitudeError {
        #[error("{0} ({1})")]
        Parse(ParseFloatError, String),
        #[error("{0}")]
        Type(#[from] types::LatitudeError),
    }

    #[derive(Debug, Error)]
    pub enum LongitudeError {
        #[error("{0} ({1})")]
        Parse(ParseFloatError, String),
        #[error("{0}")]
        Type(#[from] types::LongitudeError),
    }

    type LocationErrorT = Coprod!(LatitudeError, LongitudeError);
    #[derive(Debug, Error)]
    pub enum LocationError {
        #[error("{0}")]
        Multiple(#[from] VecError<LocationErrorT>),
        #[error("- invalid format")]
        Fmt,
    }

    #[derive(Debug, Error)]
    pub enum ElevationError {
        #[error("{0} ({1})")]
        Parse(ParseFloatError, String),
        #[error("{0}")]
        Type(#[from] types::ElevationError),
    }

    pub type TimeErrorT =
        Coprod!(ParseIntError, types::HourError, types::MinuteError);
    #[derive(Debug, Error)]
    pub enum TimeError {
        #[error("{0}")]
        Multiple(#[from] VecError<TimeErrorT>),
        #[error("- invalid format")]
        Fmt,
    }

    #[derive(Debug, Error)]
    pub enum TimeRangeError {
        #[error("{0}")]
        Multiple(#[from] VecError<TimeError>),
        #[error("- {0}")]
        Single(#[from] TimeError),
        #[error("- {0}")]
        Type(#[from] types::TimeRangeError),
        #[error("- invalid format")]
        Fmt,
    }

    #[derive(Debug, Error)]
    pub enum TimeRangesError {
        #[error("{0}")]
        Multiple(#[from] VecError<TimeRangeError>),
        #[error("- {0}")]
        Type(#[from] types::TimeRangesError),
        #[error("- invalid format")]
        Fmt,
    }

    #[derive(Debug, Error)]
    pub enum ElevationRangeError {
        #[error("{0}")]
        Multiple(#[from] VecError<ElevationError>),
        #[error("- {0}")]
        Type(#[from] types::ElevationRangeError),
        #[error("- invalid format")]
        Fmt,
    }

    #[derive(Debug, Error)]
    #[error("as time ranges:\n{time}\nas elevation range:\n{elev}")]
    pub struct TransitionSchemeError {
        pub time: TimeRangesError,
        pub elev: ElevationRangeError,
    }

    #[derive(Debug, Error)]
    #[error("as automatic:\n- did not match any provider\nas manual:\n{loc}")]
    pub struct LocationProviderError {
        pub loc: LocationError,
    }

    #[derive(Debug, Error)]
    pub enum AdjustmentMethodTypeParamError {
        #[error("there is no adjustment method with this name ({0})")]
        InvalidName(String),
        #[error("display number ({1}):\n{0}")]
        Display(ParseIntError, String),
        #[error("crtc numbers:\n{0}")]
        Crtcs(#[from] VecError<CrtcError<String, ParseIntError>>),
    }

    #[derive(Debug, Error)]
    pub enum AdjustmentMethodTypeError {
        #[error("{0}")]
        Vec(#[from] VecError<AdjustmentMethodTypeParamError>),
        #[error("videmode does not support selecting crtcs")]
        CrtcOnVidmode,
        #[error("win32gdi does not support selecting crtcs")]
        CrtcOnWin32Gdi,
        #[error("win32gdi does not support selecting display device")]
        ScreenOnWin32Gdi,
        #[error("invalid format")]
        Fmt,
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

    impl From<Vec<AdjustmentMethodTypeParamError>> for AdjustmentMethodTypeError {
        fn from(v: Vec<AdjustmentMethodTypeParamError>) -> Self {
            Self::Vec(VecError(v))
        }
    }
}

impl From<ConfigError> for ReddishError {
    fn from(e: ConfigError) -> Self {
        Self(ReddishErrorKind::Config(e))
    }
}

impl From<AdjusterError> for ReddishError {
    fn from(e: AdjusterError) -> Self {
        Self(ReddishErrorKind::Adjuster(e))
    }
}

impl From<ProviderError> for ReddishError {
    fn from(e: ProviderError) -> Self {
        Self(ReddishErrorKind::Provider(e))
    }
}

impl From<ctrlc::Error> for ReddishError {
    fn from(e: ctrlc::Error) -> Self {
        Self(ReddishErrorKind::Ctrlc(e))
    }
}

impl From<std::sync::mpsc::RecvTimeoutError> for ReddishError {
    fn from(e: std::sync::mpsc::RecvTimeoutError) -> Self {
        Self(ReddishErrorKind::Mpsc(e))
    }
}
