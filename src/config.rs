/*  config.rs -- Hierarchical configuration
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

// TODO: use snafu for error handling

use crate::solar::SOLAR_CIVIL_TWILIGHT_ELEV;
use anyhow::{anyhow, Result};
use clap::Parser;
use config::{Config as ConfigRs, File};
use const_format::formatcp;
use itertools::Itertools;
use serde::Deserialize;
use std::{env, path::PathBuf};

// Bounds for parameters.
pub const MIN_TEMPERATURE: u16 = 1000;
pub const MAX_TEMPERATURE: u16 = 25000;
pub const MIN_BRIGHTNESS: f32 = 0.1;
pub const MAX_BRIGHTNESS: f32 = 1.0;
pub const MIN_GAMMA: f32 = 0.1;
pub const MAX_GAMMA: f32 = 10.0;
pub const MIN_LATITUDE: f32 = -90.0;
pub const MAX_LATITUDE: f32 = 90.0;
pub const MIN_LONGITUDE: f32 = -180.0;
pub const MAX_LONGITUDE: f32 = 180.0;

pub const DEFAULT_TEMPERATURE: u16 = 6500;
pub const DEFAULT_TEMPERATURE_DAY: u16 = 6500;
pub const DEFAULT_TEMPERATURE_NIGHT: u16 = 4500;
pub const DEFAULT_BRIGHTNESS: f32 = 1.0;
pub const DEFAULT_GAMMA: f32 = 1.0;

// Angular elevation of the sun at which the color temperature
// transition period starts and ends (in degrees).
// Transition during twilight, and while the sun is lower than
// 3.0 degrees above the horizon.
pub const DEFAULT_ELEVATION_LOW: f32 = SOLAR_CIVIL_TWILIGHT_ELEV;
pub const DEFAULT_ELEVATION_HIGH: f32 = 3.0;

const CARGO_PKG_NAME: &str = env!("CARGO_PKG_NAME");

const VERSION: &str = {
    const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
    const GIT_DESCRIBE: &str = env!("VERGEN_GIT_DESCRIBE");
    const GIT_COMMIT_DATE: &str = env!("VERGEN_GIT_COMMIT_DATE");

    formatcp!("{CARGO_PKG_NAME} {CARGO_PKG_VERSION} ({GIT_DESCRIBE} {GIT_COMMIT_DATE})")
};

const VERBOSE_VERSION: &str = {
    const RUSTC_SEMVER: &str = env!("VERGEN_RUSTC_SEMVER");
    const RUSTC_HOST_TRIPLE: &str = env!("VERGEN_RUSTC_HOST_TRIPLE");
    const CARGO_FEATURES: &str = env!("VERGEN_CARGO_FEATURES");
    const CARGO_TARGET_TRIPLE: &str = env!("VERGEN_CARGO_TARGET_TRIPLE");

    formatcp!(
        "{VERSION}

rustc version:       {RUSTC_SEMVER}
rustc host triple:   {RUSTC_HOST_TRIPLE}
cargo features:      {CARGO_FEATURES}
cargo target triple: {CARGO_TARGET_TRIPLE}"
    )
};

const PKG_BUGREPORT: &str = "https://github.com/mahor1221/reddish-shift/issues";

// TRANSLATORS: help output
// LAT is latitude, LON is longitude,
// DAY is temperature at daytime,
// NIGHT is temperature at night
// no-wrap
// `list' must not be translated
const HELP: &str = {
    formatcp!("Usage: {CARGO_PKG_NAME} -l LAT:LON -t DAY:NIGHT [OPTIONS...]

Set color temperature of display according to time of day.

  -h\t\tDisplay this help message
  -v\t\tVerbose output
  -V\t\tShow program version

  -b DAY:NIGHT\tScreen brightness to apply (between 0.1 and 1.0)
  -c FILE\tLoad settings from specified configuration file
  -g R:G:B\tAdditional gamma correction to apply
  -l LAT:LON\tYour current location
  -l PROVIDER\tSelect provider for automatic location updates
  \t\t(Type `list` to see available providers)
  -m METHOD\tMethod to use to set color temperature
  \t\t(Type `list` to see available methods)
  -o\t\tOne shot mode (do not continuously adjust color temperature)
  -O TEMP\tOne shot manual mode (set color temperature)
  -p\t\tPrint mode (only print parameters and exit)
  -P\t\tReset existing gamma ramps before applying new color effect
  -x\t\tReset mode (remove adjustment from screen)
  -r\t\tDisable fading between color temperatures
  -t DAY:NIGHT\tColor temperature to set at daytime/night

The neutral temperature is {DEFAULT_TEMPERATURE}K. Using this value will not change the color\ntemperature of the display. Setting the color temperature to a value higher\nthan this results in more blue light, and setting a lower value will result in\nmore red light.

Default values:
  Daytime temperature: {DEFAULT_TEMPERATURE_DAY}K
  Night temperature: {DEFAULT_TEMPERATURE_NIGHT}K

Please report bugs to <{PKG_BUGREPORT}>")
};

//
// Parsed types
//

#[derive(Debug, Clone, Copy)]
pub struct Temperature(u16);

#[derive(Debug, Clone, Copy)]
pub struct Brightness(f32);

#[derive(Debug, Clone, Copy)]
pub struct Gamma(f32, f32, f32);

#[derive(Debug, Clone)]
pub struct ColorProfile {
    pub temperature: Temperature,
    pub gamma: Gamma,
    pub brightness: Brightness,
}

#[derive(Debug, Clone, Copy)]
pub struct Hour(u8);

#[derive(Debug, Clone, Copy)]
pub struct Minute(u8);

#[derive(Debug, Clone, Copy)]
pub struct Time {
    pub hour: Hour,
    pub minute: Minute,
}

#[derive(Debug, Clone, Copy)]
pub struct TimeRange {
    pub from: Time,
    pub to: Time,
}

#[derive(Debug, Clone, Copy)]
pub struct TimeRanges {
    pub dawn: TimeRange,
    pub dusk: TimeRange,
}

// The solar elevations at which the transition begins/ends,
// TODO: Check if fields are offsets from midnight in seconds.
#[derive(Debug, Clone, Copy)]
pub struct Elevation {
    pub high: f32,
    pub low: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct LatitudeDegree(f32);
#[derive(Debug, Clone, Copy)]
pub struct LongitudeDegree(f32);
#[derive(Debug, Clone, Copy)]
pub struct Location {
    latitude: LatitudeDegree,
    longitude: LongitudeDegree,
}

#[derive(Debug, Clone, Copy)]
pub enum TransitionSchemeKind {
    TimeRanges,
    Elevation,
}

#[derive(Debug, Clone)]
pub struct TransitionScheme {
    select: TransitionSchemeKind,
    time_ranges: TimeRanges,
    elevation: Elevation,
}

#[derive(Debug, Clone, Copy)]
pub enum LocationProviderKind {
    Manual,
    GeoClue2,
}

#[derive(Debug, Clone)]
pub struct LocationProvider {
    select: LocationProviderKind,
    manual: Location,
}

#[derive(Debug, Clone, Copy)]
pub enum AdjustmentMethodKind {
    Randr,
    Drm,
    VidMode,
}

#[derive(Debug, Clone, Copy)]
pub struct Randr {
    pub screen: u16,
}

#[derive(Debug, Clone)]
pub struct AdjustmentMethod {
    select: AdjustmentMethodKind,
    randr: Randr,
}

pub struct Config {
    pub day_color_profile: ColorProfile,
    pub night_color_profile: ColorProfile,
    pub preserve_gamma: bool,
    pub fade: bool,
    pub transition_scheme: TransitionScheme,
    pub location_provider: LocationProvider,
    pub adjustment_method: AdjustmentMethod,
}

//
// CLI Arguments
//

enum Mode {
    Continual,
    OneShot,
    Print,
    Reset,
    Manual,
}

// #[derive(Parser, Debug)]
// #[command(version, about, long_about)]
struct Args {
    config_file: Option<PathBuf>,
    mode: Mode,
    verbose: bool,

    temperature: Option<String>,
    brightness: Option<String>,
    gamma: Option<String>,
    fade: Option<bool>,
    preserve_gamma: Option<bool>,

    transition_scheme: Option<TransitionSchemeKind>,
    elevation: Option<String>,
    dawn: Option<String>,
    dusk: Option<String>,

    location_provider: Option<LocationProviderKind>,
    manual: Option<String>,

    adjustment_method: Option<AdjustmentMethodKind>,
    randr_screen: Option<u8>,
}

//
// Config file
//

#[derive(Debug, Clone, Deserialize)]
struct TimeRangesSection {
    dawn: Option<String>,
    dusk: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ElevationSection {
    high: Option<f32>,
    low: Option<f32>,
}

#[derive(Debug, Clone, Deserialize)]
struct LocationSection {
    latitude: Option<f32>,
    longitude: Option<f32>,
}

#[derive(Debug, Clone, Deserialize)]
struct RandrSection {
    screen: Option<u16>,
}

#[derive(Debug, Clone, Deserialize)]
struct ConfigFile {
    temperature: Option<String>,
    brightness: Option<String>,
    gamma: Option<String>,
    preserve_gamma: Option<bool>,
    fade: Option<bool>,

    transition_scheme: Option<String>,
    time_ranges: Option<TimeRangesSection>,
    elevation: Option<ElevationSection>,

    location_provider: Option<String>,
    manual: Option<LocationSection>,

    adjustment_method: Option<String>,
    randr: Option<RandrSection>,
}

//
// Merge configurations from low to high priority:
// 1. default config
// 2. system config file
// 3. user config file
// 4. cli arguments
//

impl Config {
    pub fn new() -> Result<Self> {
        let ConfigFile {
            temperature,
            brightness,
            gamma,
            preserve_gamma,
            fade,
            transition_scheme,
            time_ranges,
            elevation,
            location_provider,
            manual,
            adjustment_method,
            randr,
        } = ConfigFile::new(None)?; // TODO: pass config_file from args

        let mut config = Config::default();

        if let Some(t) = temperature {
            let DayNight { day, night }: DayNight<Temperature> = t.as_str().try_into()?;
            config.day_color_profile.temperature = day;
            config.night_color_profile.temperature = night;
        }
        if let Some(t) = brightness {
            let DayNight { day, night }: DayNight<Brightness> = t.as_str().try_into()?;
            config.day_color_profile.brightness = day;
            config.night_color_profile.brightness = night;
        }
        if let Some(t) = gamma {
            let DayNight { day, night }: DayNight<Gamma> = t.as_str().try_into()?;
            config.day_color_profile.gamma = day;
            config.night_color_profile.gamma = night;
        }
        if let Some(t) = preserve_gamma {
            config.preserve_gamma = t
        }
        if let Some(t) = fade {
            config.fade = t
        }

        if let Some(t) = transition_scheme {
            config.transition_scheme.select = t.as_str().try_into()?;
        }
        if let Some(t) = &time_ranges {
            config.transition_scheme.time_ranges = t.try_into()?;
        }
        if let Some(t) = &elevation {
            config.transition_scheme.elevation = t.try_into()?;
        }

        if let Some(t) = location_provider {
            config.location_provider.select = t.as_str().try_into()?;
        }
        if let Some(t) = &manual {
            config.location_provider.manual = t.try_into()?;
        }

        if let Some(t) = adjustment_method {
            config.adjustment_method.select = t.as_str().try_into()?;
        }
        if let Some(t) = &randr {
            config.adjustment_method.randr = t.try_into()?;
        }

        Ok(config)
    }
}

impl ConfigFile {
    fn new(custom_user_config_path: Option<PathBuf>) -> Result<Self> {
        #[cfg(unix)]
        let system_config = PathBuf::from(formatcp!("/etc/{CARGO_PKG_NAME}/config.toml"));
        let user_config = custom_user_config_path
            .or_else(|| dirs::config_dir().map(|d| d.join(CARGO_PKG_NAME).join("config.toml")));

        // TODO: drop dependency on config-rs and merge manually
        let config_file = ConfigRs::builder();
        #[cfg(unix)]
        let config_file = config_file.add_source(File::from(system_config).required(false));
        let config_file = match user_config {
            Some(path) => config_file.add_source(File::from(path).required(false)),
            None => config_file,
        };
        let config_file: ConfigFile = config_file.build()?.try_deserialize()?;

        Ok(config_file)
    }
}

impl Default for Temperature {
    fn default() -> Self {
        Self(DEFAULT_TEMPERATURE)
    }
}

impl Default for Brightness {
    fn default() -> Self {
        Brightness(DEFAULT_BRIGHTNESS)
    }
}

impl Default for Gamma {
    fn default() -> Self {
        Gamma(DEFAULT_GAMMA, DEFAULT_GAMMA, DEFAULT_GAMMA)
    }
}

impl Default for ColorProfile {
    fn default() -> Self {
        Self {
            temperature: Temperature::default(),
            gamma: Gamma::default(),
            brightness: Brightness::default(),
        }
    }
}

impl ColorProfile {
    pub fn default_day() -> Self {
        Self {
            temperature: Temperature(DEFAULT_TEMPERATURE_DAY),
            ..Default::default()
        }
    }

    pub fn default_night() -> Self {
        Self {
            temperature: Temperature(DEFAULT_TEMPERATURE_NIGHT),
            ..Default::default()
        }
    }
}

impl Default for TimeRanges {
    fn default() -> Self {
        todo!()
    }
}

impl Default for Elevation {
    fn default() -> Self {
        Self {
            high: DEFAULT_ELEVATION_HIGH,
            low: DEFAULT_ELEVATION_LOW,
        }
    }
}

impl Default for TransitionScheme {
    fn default() -> Self {
        Self {
            select: TransitionSchemeKind::TimeRanges,
            time_ranges: TimeRanges::default(),
            elevation: Elevation::default(),
        }
    }
}

impl Default for Location {
    fn default() -> Self {
        // TODO: find something generic
        Self {
            latitude: LatitudeDegree(48.1),
            longitude: LongitudeDegree(11.6),
        }
    }
}

impl Default for LocationProvider {
    fn default() -> Self {
        Self {
            select: LocationProviderKind::Manual,
            manual: Location::default(),
        }
    }
}

impl Default for Randr {
    fn default() -> Self {
        Self { screen: 0 }
    }
}

impl Default for AdjustmentMethod {
    fn default() -> Self {
        todo!()
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            day_color_profile: ColorProfile::default_day(),
            night_color_profile: ColorProfile::default_night(),
            preserve_gamma: true,
            fade: true,
            transition_scheme: TransitionScheme::default(),
            location_provider: LocationProvider::default(),
            adjustment_method: AdjustmentMethod::default(),
        }
    }
}

//
// Parse strings and numbers to strong types
//

#[derive(Debug, Clone)]
pub struct DayNight<T> {
    day: T,
    night: T,
}

impl<'a, T> TryFrom<&'a str> for DayNight<T>
where
    T: Clone + TryFrom<&'a str, Error = anyhow::Error>,
{
    type Error = anyhow::Error;

    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        match *s.split("-").map(str::trim).collect_vec().as_slice() {
            [day, night] => Ok(Self {
                day: day.try_into()?,
                night: night.try_into()?,
            }),
            _ => {
                let temp: T = s.try_into()?;
                Ok(Self {
                    day: temp.clone(),
                    night: temp,
                })
            }
        }
    }
}

impl TryFrom<u16> for Temperature {
    type Error = anyhow::Error;

    fn try_from(n: u16) -> Result<Self, Self::Error> {
        if n >= MIN_TEMPERATURE && n <= MAX_TEMPERATURE {
            Ok(Self(n))
        } else {
            Err(anyhow!("temperature"))
        }
    }
}

impl TryFrom<&str> for Temperature {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let n = s.parse::<u16>()?;
        Self::try_from(n)
    }
}

impl TryFrom<f32> for Brightness {
    type Error = anyhow::Error;

    fn try_from(n: f32) -> Result<Self, Self::Error> {
        if n >= MIN_BRIGHTNESS && n <= MAX_BRIGHTNESS {
            Ok(Self(n))
        } else {
            Err(anyhow!("brightness"))
        }
    }
}

impl TryFrom<&str> for Brightness {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let n = s.parse::<f32>()?;
        Self::try_from(n)
    }
}

fn gamma(n: f32) -> Result<f32> {
    if n >= MIN_GAMMA && n <= MAX_GAMMA {
        Ok(n)
    } else {
        Err(anyhow!("gamma"))
    }
}

impl TryFrom<f32> for Gamma {
    type Error = anyhow::Error;

    fn try_from(n: f32) -> Result<Self, Self::Error> {
        let n = gamma(n)?;
        Ok(Self(n, n, n))
    }
}

impl TryFrom<(f32, f32, f32)> for Gamma {
    type Error = anyhow::Error;

    fn try_from((r, g, b): (f32, f32, f32)) -> Result<Self, Self::Error> {
        Ok(Self(gamma(r)?, gamma(g)?, gamma(b)?))
    }
}

impl TryFrom<&str> for Gamma {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match *s.split(":").map(str::trim).collect_vec().as_slice() {
            [r, g, b] => (r.parse::<f32>()?, g.parse::<f32>()?, b.parse::<f32>()?).try_into(),
            [rbg] => rbg.parse::<f32>()?.try_into(),
            _ => Err(anyhow!("gamma")),
        }
    }
}

impl TryFrom<&str> for Hour {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let n = s.parse::<u8>()?;
        if n < 24 {
            Ok(Self(n))
        } else {
            Err(anyhow!("hour"))
        }
    }
}

impl TryFrom<&str> for Minute {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let n = s.parse::<u8>()?;
        if n < 60 {
            Ok(Self(n))
        } else {
            Err(anyhow!("minute"))
        }
    }
}

impl TryFrom<&str> for Time {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match *s.split(":").map(str::trim).collect_vec().as_slice() {
            [hour, minute] => Ok(Self {
                hour: hour.try_into()?,
                minute: minute.try_into()?,
            }),
            _ => Err(anyhow!("time")),
        }
    }
}

impl TryFrom<&str> for TimeRange {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match *s.split("-").map(str::trim).collect_vec().as_slice() {
            [from, to] => Ok(Self {
                from: from.try_into()?,
                to: to.try_into()?,
            }),
            _ => Err(anyhow!("time_range")),
        }
    }
}

impl TryFrom<(f32, f32)> for Elevation {
    type Error = anyhow::Error;

    fn try_from((high, low): (f32, f32)) -> Result<Self, Self::Error> {
        if high >= low {
            Ok(Elevation { high, low })
        } else {
            // fprintf(stderr,
            // 	_("High transition elevation cannot be lower than"
            // 	  " the low transition elevation.\n"));
            Err(anyhow!("elevation"))
        }
    }
}

impl TryFrom<&str> for Elevation {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match *s.split(":").map(str::trim).collect_vec().as_slice() {
            [high, low] => (high.parse::<f32>()?, low.parse::<f32>()?).try_into(),
            _ => Err(anyhow!("elevation")),
        }
    }
}

impl TryFrom<f32> for LatitudeDegree {
    type Error = anyhow::Error;

    fn try_from(n: f32) -> Result<Self, Self::Error> {
        if n >= MIN_LATITUDE && n <= MAX_LATITUDE {
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

impl TryFrom<f32> for LongitudeDegree {
    type Error = anyhow::Error;

    fn try_from(n: f32) -> Result<Self, Self::Error> {
        if n >= MIN_LONGITUDE && n <= MAX_LONGITUDE {
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

impl TryFrom<&str> for LongitudeDegree {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse::<f32>()?.try_into()
    }
}

impl TryFrom<&str> for LatitudeDegree {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse::<f32>()?.try_into()
    }
}

impl TryFrom<&str> for Location {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match *s.split(":").map(str::trim).collect_vec().as_slice() {
            [lat, lon] => Ok(Self {
                latitude: lat.try_into()?,
                longitude: lon.try_into()?,
            }),
            _ => Err(anyhow!("location")),
        }
    }
}

impl TryFrom<&LocationSection> for Location {
    type Error = anyhow::Error;

    fn try_from(t: &LocationSection) -> Result<Self, Self::Error> {
        match *t {
            LocationSection {
                latitude: Some(lat),
                longitude: Some(lon),
            } => Ok(Self {
                latitude: lat.try_into()?,
                longitude: lon.try_into()?,
            }),
            _ => Err(anyhow!("location")),
        }
    }
}

impl TryFrom<&TimeRangesSection> for TimeRanges {
    type Error = anyhow::Error;

    fn try_from(_t: &TimeRangesSection) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl TryFrom<&ElevationSection> for Elevation {
    type Error = anyhow::Error;

    fn try_from(t: &ElevationSection) -> Result<Self, Self::Error> {
        match *t {
            ElevationSection {
                high: Some(high),
                low: Some(low),
            } => (high, low).try_into(),
            _ => Err(anyhow!("elevation")),
        }
    }
}

impl TryFrom<&RandrSection> for Randr {
    type Error = anyhow::Error;

    fn try_from(t: &RandrSection) -> Result<Self, Self::Error> {
        match *t {
            RandrSection { screen: Some(scr) } => Ok(Self { screen: scr }),
            _ => Err(anyhow!("randr")),
        }
    }
}

impl TryFrom<&str> for TransitionSchemeKind {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "time-ranges" => Ok(Self::TimeRanges),
            "elevation" => Ok(Self::Elevation),
            _ => Err(anyhow!("location_provider")),
        }
    }
}

impl TryFrom<&str> for LocationProviderKind {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "manual" => Ok(Self::Manual),
            "geoclue2" => Ok(Self::GeoClue2),
            _ => Err(anyhow!("location_provider")),
        }
    }
}

impl TryFrom<&str> for AdjustmentMethodKind {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "randr" => Ok(Self::Randr),
            "drm" => Ok(Self::Drm),
            "vidmode" => Ok(Self::VidMode),
            _ => Err(anyhow!("adjustment_method")),
        }
    }
}

//

#[cfg(test)]
mod test {
    use super::*;
    use config::FileFormat;

    #[test]
    fn test_config_template() -> Result<()> {
        const CONFIG_TEMPLATE: &str =
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/config.toml"));
        ConfigRs::builder()
            .add_source(File::from_str(CONFIG_TEMPLATE, FileFormat::Toml))
            .build()?
            .try_deserialize::<ConfigFile>()?;
        Ok(())
    }
}
