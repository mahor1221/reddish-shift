/*  types.rs -- Common types
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

use crate::{
    calc_solar::{solar_elevation, SOLAR_CIVIL_TWILIGHT_ELEV},
    error::{types::*, ProviderError},
    utils::{InjectErr, IntoGeneric},
    LocationProvider, Provider,
};
use chrono::{DateTime, Local, NaiveTime, Timelike};
use frunk::{validated::IntoValidated, Generic};
use std::ops::Deref;

/// Angular elevation of the sun at which the color temperature transition
/// period starts and ends (in degrees).
/// Transition during twilight, and while the sun is lower than 3.0 degrees
/// above the horizon.
pub const DEFAULT_ELEVATION_LOW: f64 = SOLAR_CIVIL_TWILIGHT_ELEV;
pub const DEFAULT_ELEVATION_HIGH: f64 = 3.0;
pub const DEFAULT_LATITUDE: f64 = 0.0;
pub const DEFAULT_LONGITUDE: f64 = 0.0;
pub const DEFAULT_BRIGHTNESS: f64 = 1.0;
pub const DEFAULT_GAMMA: f64 = 1.0;
pub const DEFAULT_TEMPERATURE: u16 = 6500;
pub const DEFAULT_TEMPERATURE_DAY: u16 = 6500;
pub const DEFAULT_TEMPERATURE_NIGHT: u16 = 4500;

pub const MIN_TEMPERATURE: u16 = 1000;
pub const MAX_TEMPERATURE: u16 = 25000;
pub const MIN_BRIGHTNESS: f64 = 0.1;
pub const MAX_BRIGHTNESS: f64 = 1.0;
pub const MIN_GAMMA: f64 = 0.1;
pub const MAX_GAMMA: f64 = 10.0;
pub const MIN_LATITUDE: f64 = -90.0;
pub const MAX_LATITUDE: f64 = 90.0;
pub const MIN_LONGITUDE: f64 = -180.0;
pub const MAX_LONGITUDE: f64 = 180.0;
pub const MIN_ELEVATION: f64 = -90.0;
pub const MAX_ELEVATION: f64 = 90.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Temperature(u16);

#[derive(Debug, Clone, Copy)]
pub struct Brightness(f64);

#[derive(Debug, Clone, Copy)]
pub struct Gamma([f64; 3]);

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ColorSettings {
    pub temp: Temperature,
    pub gamma: Gamma,
    pub brght: Brightness,
}

#[derive(Debug, Clone, Copy, Generic)]
pub struct Time {
    pub hour: u8,
    pub minute: u8,
}

/// Offset from midnight in seconds
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimeOffset(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeRange {
    pub start: TimeOffset,
    pub end: TimeOffset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Generic)]
pub struct TimeRanges {
    pub dawn: TimeRange,
    pub dusk: TimeRange,
}

#[derive(Debug, Clone, Copy, PartialOrd)]
pub struct Elevation(f64);

/// The solar elevations at which the transition begins/ends,
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ElevationRange {
    pub high: Elevation,
    pub low: Elevation,
}

#[derive(Debug, Clone, Copy)]
pub struct Latitude(f64);
#[derive(Debug, Clone, Copy)]
pub struct Longitude(f64);
#[derive(Debug, Clone, Copy, Default, PartialEq, Generic)]
pub struct Location {
    pub lat: Latitude,
    pub lon: Longitude,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum Mode {
    #[default]
    Daemon,
    Oneshot,
    Set,
    Reset,
    Print,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TransitionScheme {
    Time(TimeRanges),
    Elev(ElevationRange),
}

#[derive(Debug, Clone, PartialEq)]
pub enum LocationProviderType {
    Manual(Location),
    Geoclue2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdjustmentMethodType {
    Dummy,
    Drm {
        card_num: Option<usize>,
        crtcs: Vec<u32>,
    },
    Randr {
        screen_num: Option<usize>,
        crtcs: Vec<u32>,
    },
    Vidmode {
        screen_num: Option<usize>,
    },
    Win32Gdi,
}

#[derive(Debug, Clone, Copy)]
pub struct Alpha(f64);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Period {
    Daytime,
    Night,
    Transition {
        progress: u8, // Between 0 and 100
    },
}

#[derive(Debug, Clone, Generic)]
pub struct DayNight<T> {
    pub day: T,
    pub night: T,
}

pub type TemperatureRange = DayNight<Temperature>;
pub type BrightnessRange = DayNight<Brightness>;
pub type GammaRange = DayNight<Gamma>;

//

impl Default for Temperature {
    fn default() -> Self {
        Self(DEFAULT_TEMPERATURE)
    }
}

impl Default for Brightness {
    fn default() -> Self {
        Self(DEFAULT_BRIGHTNESS)
    }
}

impl Default for Gamma {
    fn default() -> Self {
        Self([DEFAULT_GAMMA; 3])
    }
}

impl Default for Elevation {
    fn default() -> Self {
        Self(0.0)
    }
}

impl Default for ElevationRange {
    fn default() -> Self {
        Self {
            high: Elevation(DEFAULT_ELEVATION_HIGH),
            low: Elevation(DEFAULT_ELEVATION_LOW),
        }
    }
}

impl Default for Latitude {
    fn default() -> Self {
        Self(DEFAULT_LATITUDE)
    }
}

impl Default for Longitude {
    fn default() -> Self {
        Self(DEFAULT_LONGITUDE)
    }
}

impl ColorSettings {
    pub fn default_day() -> Self {
        Self {
            temp: Temperature(DEFAULT_TEMPERATURE_DAY),
            ..Default::default()
        }
    }

    pub fn default_night() -> Self {
        Self {
            temp: Temperature(DEFAULT_TEMPERATURE_NIGHT),
            ..Default::default()
        }
    }
}

impl Default for TransitionScheme {
    fn default() -> Self {
        Self::Elev(Default::default())
    }
}

impl Default for LocationProviderType {
    fn default() -> Self {
        Self::Manual(Default::default())
    }
}

impl Default for Period {
    fn default() -> Self {
        Self::Daytime
    }
}

//

pub fn gamma(n: f64) -> Result<f64, GammaError> {
    if (MIN_GAMMA..=MAX_GAMMA).contains(&n) {
        Ok(n)
    } else {
        Err(GammaError(n))
    }
}

impl TryFrom<u16> for Temperature {
    type Error = TemperatureError;

    fn try_from(n: u16) -> Result<Self, Self::Error> {
        if (MIN_TEMPERATURE..=MAX_TEMPERATURE).contains(&n) {
            Ok(Self(n))
        } else {
            Err(TemperatureError(n))
        }
    }
}

impl TryFrom<f64> for Brightness {
    type Error = BrightnessError;

    fn try_from(n: f64) -> Result<Self, Self::Error> {
        if (MIN_BRIGHTNESS..=MAX_BRIGHTNESS).contains(&n) {
            Ok(Self(n))
        } else {
            Err(BrightnessError(n))
        }
    }
}

impl TryFrom<f64> for Gamma {
    type Error = GammaError;

    fn try_from(n: f64) -> Result<Self, Self::Error> {
        Ok(Self([gamma(n)?; 3]))
    }
}

impl TryFrom<(f64, f64, f64)> for Gamma {
    type Error = GammaRgbError;

    fn try_from((r, g, b): (f64, f64, f64)) -> Result<Self, Self::Error> {
        let (r, g, b) = (gamma(r).into_validated() + gamma(g) + gamma(b))
            .into_result()?
            .into_generic();
        Ok(Self([r, g, b]))
    }
}

impl TryFrom<f64> for Latitude {
    type Error = LatitudeError;

    fn try_from(n: f64) -> Result<Self, Self::Error> {
        if (MIN_LATITUDE..=MAX_LATITUDE).contains(&n) {
            Ok(Self(n))
        } else {
            Err(LatitudeError(n))
        }
    }
}

impl TryFrom<f64> for Longitude {
    type Error = LongitudeError;

    fn try_from(n: f64) -> Result<Self, Self::Error> {
        if (MIN_LONGITUDE..=MAX_LONGITUDE).contains(&n) {
            Ok(Self(n))
        } else {
            Err(LongitudeError(n))
        }
    }
}

impl TryFrom<f64> for Elevation {
    type Error = ElevationError;

    fn try_from(n: f64) -> Result<Self, Self::Error> {
        if (MIN_ELEVATION..=MAX_ELEVATION).contains(&n) {
            Ok(Self(n))
        } else {
            Err(ElevationError(n))
        }
    }
}

impl TryFrom<(f64, f64)> for Location {
    type Error = LocationError;

    fn try_from((lat, lon): (f64, f64)) -> Result<Self, Self::Error> {
        Ok((lat.try_into().inject_err().into_validated()
            + lon.try_into().inject_err())
        .into_result()?
        .into_generic())
    }
}

pub fn hour(h: u8) -> Result<u8, HourError> {
    if h < 24 {
        Ok(h)
    } else {
        Err(HourError(h))
    }
}

pub fn minute(m: u8) -> Result<u8, MinuteError> {
    if m < 24 {
        Ok(m)
    } else {
        Err(MinuteError(m))
    }
}

impl TryFrom<(u8, u8)> for Time {
    type Error = TimeError;

    fn try_from((h, m): (u8, u8)) -> Result<Self, Self::Error> {
        let time = (hour(h).inject_err().into_validated()
            + minute(m).inject_err())
        .into_result()?
        .into_generic();
        Ok(time)
    }
}

impl From<Time> for TimeOffset {
    fn from(Time { hour, minute }: Time) -> Self {
        Self(hour as u32 * 3600 + minute as u32 * 60)
    }
}

impl From<TimeOffset> for Time {
    fn from(time: TimeOffset) -> Self {
        Self {
            hour: (*time as f64 / 3600.0) as u8,
            minute: ((*time as f64 % 3600.0) / 60.0) as u8,
        }
    }
}

impl TryFrom<(TimeOffset, TimeOffset)> for TimeRange {
    type Error = TimeRangeError;

    fn try_from(
        (start, end): (TimeOffset, TimeOffset),
    ) -> Result<Self, Self::Error> {
        if start <= end {
            Ok(Self { start, end })
        } else {
            Err(TimeRangeError { start, end })
        }
    }
}

impl TryFrom<(TimeRange, TimeRange)> for TimeRanges {
    type Error = TimeRangesError;

    fn try_from(
        (dawn, dusk): (TimeRange, TimeRange),
    ) -> Result<Self, Self::Error> {
        if dawn.end < dusk.start {
            Ok(Self { dawn, dusk })
        } else {
            Err(TimeRangesError {
                dawn_end: dawn.end,
                dusk_start: dusk.start,
            })
        }
    }
}

impl TryFrom<(Elevation, Elevation)> for ElevationRange {
    type Error = ElevationRangeError;

    fn try_from(
        (high, low): (Elevation, Elevation),
    ) -> Result<Self, Self::Error> {
        if high >= low {
            Ok(Self { high, low })
        } else {
            Err(ElevationRangeError { high, low })
        }
    }
}

impl TryFrom<u16> for TemperatureRange {
    type Error = TemperatureError;

    fn try_from(n: u16) -> Result<Self, Self::Error> {
        let t = Temperature::try_from(n)?;
        Ok(Self { day: t, night: t })
    }
}

impl TryFrom<f64> for BrightnessRange {
    type Error = BrightnessError;

    fn try_from(n: f64) -> Result<Self, Self::Error> {
        let t = Brightness::try_from(n)?;
        Ok(Self { day: t, night: t })
    }
}

impl TryFrom<f64> for GammaRange {
    type Error = GammaError;

    fn try_from(n: f64) -> Result<Self, Self::Error> {
        let t = Gamma::try_from(n)?;
        Ok(Self { day: t, night: t })
    }
}

impl TryFrom<f64> for Alpha {
    type Error = AlphaError;

    fn try_from(n: f64) -> Result<Self, Self::Error> {
        if (0.0..=1.0).contains(&n) {
            Ok(Self(n))
        } else {
            Err(AlphaError(n))
        }
    }
}

// NOTE: Using Deref is not an anti pattern here. These newtypes are plain
// wrappers that only enforce restrictions and boundaries on the inner type.
// I want these wrappers act exactly like their inner type. Obviously DerefMut
// should not be implemented for these types. See this discussion:
// https://users.rust-lang.org/t/understanding-the-perils-of-deref/47958/18

impl Deref for Temperature {
    type Target = u16;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for Brightness {
    type Target = f64;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for Gamma {
    type Target = [f64; 3];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for TimeOffset {
    type Target = u32;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for Elevation {
    type Target = f64;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for Latitude {
    type Target = f64;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for Longitude {
    type Target = f64;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for Alpha {
    type Target = f64;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

//

fn eq(lhs: f64, rhs: f64) -> bool {
    (lhs * 100.0).round() == (rhs * 100.0).round()
}

impl PartialEq for Brightness {
    fn eq(&self, other: &Self) -> bool {
        eq(**self, **other)
    }
}
impl PartialEq for Elevation {
    fn eq(&self, other: &Self) -> bool {
        eq(**self, **other)
    }
}
impl PartialEq for Latitude {
    fn eq(&self, other: &Self) -> bool {
        eq(**self, **other)
    }
}
impl PartialEq for Longitude {
    fn eq(&self, other: &Self) -> bool {
        eq(**self, **other)
    }
}
impl PartialEq for Gamma {
    fn eq(&self, other: &Self) -> bool {
        eq(self[0], other[0]) && eq(self[1], other[1]) && eq(self[2], other[2])
    }
}

//

impl From<NaiveTime> for TimeOffset {
    fn from(time: NaiveTime) -> Self {
        Self(time.num_seconds_from_midnight())
    }
}

impl From<Period> for Alpha {
    fn from(period: Period) -> Self {
        match period {
            Period::Daytime => Self(1.0),
            Period::Night => Self(0.0),
            Period::Transition { progress } => Self(progress as f64 / 100.0),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PeriodInfo {
    Elevation { elev: Elevation, loc: Location },
    Time,
}

impl Default for PeriodInfo {
    fn default() -> Self {
        Self::Elevation {
            elev: Default::default(),
            loc: Default::default(),
        }
    }
}

impl Period {
    pub fn from(
        scheme: &TransitionScheme,
        location: &LocationProvider,
        datetime: impl Fn() -> DateTime<Local>,
    ) -> Result<(Self, PeriodInfo), ProviderError> {
        match scheme {
            TransitionScheme::Elev(elev_range) => {
                let now = (datetime().to_utc() - DateTime::UNIX_EPOCH)
                    .num_seconds() as f64;
                let here = location.get()?;
                let elev = Elevation::new(now, here);
                let period = Period::from_elevation(elev, *elev_range);
                let info = PeriodInfo::Elevation { elev, loc: here };
                Ok((period, info))
            }

            TransitionScheme::Time(time_ranges) => {
                let time = datetime().time().into();
                let period = Period::from_time(time, *time_ranges);
                Ok((period, PeriodInfo::Time))
            }
        }
    }

    /// Determine which period we are currently in based on time offset
    pub fn from_time(time: TimeOffset, time_ranges: TimeRanges) -> Self {
        let TimeRanges { dawn, dusk } = time_ranges;
        let sub =
            |a: TimeOffset, b: TimeOffset| (*a as i32 - *b as i32) as f64;

        if time < dawn.start || time >= dusk.end {
            Self::Night
        } else if time < dawn.end {
            let progress = sub(dawn.start, time) / sub(dawn.start, dawn.end);
            let progress = (progress * 100.0) as u8;
            Self::Transition { progress }
        } else if time > dusk.start {
            let progress = sub(dusk.end, time) / sub(dusk.end, dusk.start);
            let progress = (progress * 100.0) as u8;
            Self::Transition { progress }
        } else {
            Self::Daytime
        }
    }

    /// Determine which period we are currently in based on solar elevation
    pub fn from_elevation(
        elev: Elevation,
        elev_range: ElevationRange,
    ) -> Self {
        let ElevationRange { high, low } = elev_range;
        let sub = |a: Elevation, b: Elevation| (*a - *b);

        if elev < low {
            Self::Night
        } else if elev < high {
            let progress = sub(low, elev) / sub(low, high);
            let progress = (progress * 100.0) as u8;
            Self::Transition { progress }
        } else {
            Self::Daytime
        }
    }
}

impl Elevation {
    pub fn new(secs_from_epoch: f64, loc: Location) -> Self {
        Self(solar_elevation(secs_from_epoch, *loc.lat, *loc.lon))
    }
}

impl ColorSettings {
    /// Interpolate color setting structs given alpha
    pub fn interpolate_with(
        &self,
        other: &ColorSettings,
        alpha: Alpha,
    ) -> ColorSettings {
        let a = *alpha;

        let temp = Temperature(
            ((1.0 - a) * *self.temp as f64 + a * *other.temp as f64) as u16,
        );
        let gamma = Gamma(
            [0, 1, 2].map(|i| (1.0 - a) * self.gamma[i] + a * other.gamma[i]),
        );
        let brght = Brightness((1.0 - a) * *self.brght + a * *other.brght);

        ColorSettings { temp, gamma, brght }
    }

    /// Return true if color settings have major differences
    /// Used to determine if a fade should be applied in continual mode
    pub fn is_very_diff_from(&self, other: &Self) -> bool {
        (*self.temp as i16 - *other.temp as i16).abs() > 25
            || (*self.brght - *other.brght).abs() > 0.1
            || (self.gamma[0] - other.gamma[0]).abs() > 0.1
            || (self.gamma[1] - other.gamma[1]).abs() > 0.1
            || (self.gamma[2] - other.gamma[2]).abs() > 0.1
    }
}
