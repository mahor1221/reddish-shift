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
    gamma_drm::Drm,
    gamma_dummy::Dummy,
    gamma_randr::Randr,
    gamma_vidmode::Vidmode,
    location_manual::Manual,
    Geoclue2,
};
use anyhow::{anyhow, Result};
use chrono::{NaiveTime, Timelike};
use std::{cmp::Ordering, io::Write, ops::Deref};

// Angular elevation of the sun at which the color temperature
// transition period starts and ends (in degrees).
// Transition during twilight, and while the sun is lower than
// 3.0 degrees above the horizon.
pub const DEFAULT_ELEVATION_LOW: f64 = SOLAR_CIVIL_TWILIGHT_ELEV;
pub const DEFAULT_ELEVATION_HIGH: f64 = 3.0;
pub const DEFAULT_LATITUDE: f64 = 0.0; // Null Island
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

#[derive(Debug, Clone, Copy)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Location {
    pub lat: Latitude,
    pub lon: Longitude,
}

#[derive(Debug)]
pub enum AdjustmentMethod {
    Dummy(Dummy),
    Randr(Randr),
    Drm(Drm),
    Vidmode(Vidmode),
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
    Elevation(ElevationRange),
}

#[derive(Debug, PartialEq)]
pub enum LocationProvider {
    Manual(Manual),
    Geoclue2(Geoclue2),
}

#[derive(Debug, Clone, PartialEq)]
pub enum LocationProviderType {
    Manual(Manual),
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
}

#[derive(Debug, Clone)]
pub enum Verbosity<W: Write> {
    Quite,
    Low(W),
    High(W),
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

#[derive(Debug, Clone)]
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
        Self::Elevation(Default::default())
    }
}

impl Default for AdjustmentMethod {
    fn default() -> Self {
        Self::Dummy(Default::default())
    }
}

impl Default for LocationProvider {
    fn default() -> Self {
        Self::Manual(Default::default())
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

fn gamma(n: f64) -> Result<f64> {
    if (MIN_GAMMA..=MAX_GAMMA).contains(&n) {
        Ok(n)
    } else {
        // b"Gamma value must be between %.1f and %.1f.\n\0" as *const u8 as *const c_char,
        Err(anyhow!("gamma"))
    }
}

impl TryFrom<u16> for Temperature {
    type Error = anyhow::Error;

    fn try_from(n: u16) -> Result<Self, Self::Error> {
        if (MIN_TEMPERATURE..=MAX_TEMPERATURE).contains(&n) {
            Ok(Self(n))
        } else {
            // b"Temperature must be between %uK and %uK.\n\0" as *const u8 as *const c_char,
            Err(anyhow!("temperature"))
        }
    }
}

impl TryFrom<f64> for Brightness {
    type Error = anyhow::Error;

    fn try_from(n: f64) -> Result<Self, Self::Error> {
        if (MIN_BRIGHTNESS..=MAX_BRIGHTNESS).contains(&n) {
            Ok(Self(n))
        } else {
            // b"Brightness values must be between %.1f and %.1f.\n\0" as *const u8 as *const c_char,
            Err(anyhow!("brightness"))
        }
    }
}

impl TryFrom<f64> for Gamma {
    type Error = anyhow::Error;

    fn try_from(n: f64) -> Result<Self, Self::Error> {
        Ok(Self([gamma(n)?; 3]))
    }
}

impl TryFrom<[f64; 3]> for Gamma {
    type Error = anyhow::Error;

    fn try_from([r, g, b]: [f64; 3]) -> Result<Self, Self::Error> {
        Ok(Self([gamma(r)?, gamma(g)?, gamma(b)?]))
    }
}

impl TryFrom<f64> for Latitude {
    type Error = anyhow::Error;

    fn try_from(n: f64) -> Result<Self, Self::Error> {
        if (MIN_LATITUDE..=MAX_LATITUDE).contains(&n) {
            Ok(Self(n))
        } else {
            // // TRANSLATORS: Append degree symbols if possible.
            // eprintln!(
            //     "Latitude must be between {:.1} and {:.1}.",
            //     MIN_LATITUDE, MAX_LATITUDE,
            // );
            Err(anyhow!("latitude"))
        }
    }
}

impl TryFrom<f64> for Longitude {
    type Error = anyhow::Error;

    fn try_from(n: f64) -> Result<Self, Self::Error> {
        if (MIN_LONGITUDE..=MAX_LONGITUDE).contains(&n) {
            Ok(Self(n))
        } else {
            // // TRANSLATORS: Append degree symbols if possible.
            // eprintln!(
            //     "Longitude must be between {:.1} and {:.1}.",
            //     MIN_LONGITUDE, MAX_LONGITUDE,
            // );
            Err(anyhow!("longitude"))
        }
    }
}

impl TryFrom<(f64, f64)> for Location {
    type Error = anyhow::Error;

    fn try_from((lat, lon): (f64, f64)) -> Result<Self, Self::Error> {
        Ok(Self {
            lat: lat.try_into()?,
            lon: lon.try_into()?,
        })
    }
}

impl TryFrom<(u8, u8)> for Time {
    type Error = anyhow::Error;

    fn try_from((hour, minute): (u8, u8)) -> Result<Self, Self::Error> {
        if hour >= 24 {
            Err(anyhow!("hour"))
        } else if minute >= 60 {
            Err(anyhow!("minute"))
        } else {
            Ok(Self { hour, minute })
        }
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
    type Error = anyhow::Error;

    fn try_from(
        (start, end): (TimeOffset, TimeOffset),
    ) -> Result<Self, Self::Error> {
        if start <= end {
            Ok(Self { start, end })
        } else {
            Err(anyhow!("offset_range"))
        }
    }
}

impl TryFrom<f64> for Elevation {
    type Error = anyhow::Error;

    fn try_from(n: f64) -> Result<Self, Self::Error> {
        if (MIN_ELEVATION..=MAX_ELEVATION).contains(&n) {
            Ok(Self(n))
        } else {
            Err(anyhow!("elevation"))
        }
    }
}

impl TryFrom<u16> for TemperatureRange {
    type Error = anyhow::Error;

    fn try_from(n: u16) -> Result<Self, Self::Error> {
        let t = Temperature::try_from(n)?;
        Ok(Self { day: t, night: t })
    }
}

impl TryFrom<f64> for BrightnessRange {
    type Error = anyhow::Error;

    fn try_from(n: f64) -> Result<Self, Self::Error> {
        let t = Brightness::try_from(n)?;
        Ok(Self { day: t, night: t })
    }
}

impl TryFrom<f64> for GammaRange {
    type Error = anyhow::Error;

    fn try_from(n: f64) -> Result<Self, Self::Error> {
        let t = Gamma::try_from(n)?;
        Ok(Self { day: t, night: t })
    }
}

impl TryFrom<f64> for Alpha {
    type Error = anyhow::Error;

    fn try_from(n: f64) -> Result<Self, Self::Error> {
        if (0.0..=1.0).contains(&n) {
            Ok(Self(n))
        } else {
            Err(anyhow!("alpha"))
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

impl<W: Write> Eq for Verbosity<W> {}
impl<W: Write> PartialEq for Verbosity<W> {
    fn eq(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Verbosity::Quite, Verbosity::Quite)
                | (Verbosity::Low(_), Verbosity::Low(_))
                | (Verbosity::High(_), Verbosity::High(_))
        )
    }
}

impl<W: Write> Ord for Verbosity<W> {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Verbosity::Quite, Verbosity::Quite) => Ordering::Equal,
            (Verbosity::Quite, Verbosity::Low(_)) => Ordering::Less,
            (Verbosity::Quite, Verbosity::High(_)) => Ordering::Less,
            (Verbosity::Low(_), Verbosity::Quite) => Ordering::Greater,
            (Verbosity::Low(_), Verbosity::Low(_)) => Ordering::Equal,
            (Verbosity::Low(_), Verbosity::High(_)) => Ordering::Less,
            (Verbosity::High(_), Verbosity::Quite) => Ordering::Greater,
            (Verbosity::High(_), Verbosity::Low(_)) => Ordering::Greater,
            (Verbosity::High(_), Verbosity::High(_)) => Ordering::Equal,
        }
    }
}

impl<W: Write> PartialOrd for Verbosity<W> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
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
