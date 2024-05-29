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

use anyhow::{anyhow, Result};
use config::{Config as ConfigRs, Environment, File};
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

//

#[derive(Debug, Clone, Deserialize)]
pub struct Manual {
    pub latitude: Option<f32>,
    pub longitude: Option<f32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Randr {
    pub screen: Option<u16>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigFile {
    pub temperature: Option<String>,
    pub brightness: Option<String>,
    pub gamma: Option<String>,
    pub fade: Option<bool>,

    pub elevation_high: Option<f32>,
    pub elevation_low: Option<f32>,
    pub time_dawn: Option<String>,
    pub time_dusk: Option<String>,

    pub location_provider: Option<String>,
    pub adjustment_method: Option<String>,
    pub manual: Option<Manual>,
    pub randr: Option<Randr>,
}

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

// if (options.scheme.high < options.scheme.low) {
// 	fprintf(stderr,
// 		_("High transition elevation cannot be lower than"
// 		  " the low transition elevation.\n"));
// 	exit(EXIT_FAILURE);
// }

// The solar elevations at which the transition begins/ends,
// TODO: Check if fields are offsets from midnight in seconds.
#[derive(Debug, Clone, Copy)]
pub struct Elevation {
    pub high: i8,
    pub low: i8,
}

#[derive(Debug, Clone)]
pub enum TransitionScheme {
    TimeRanges(TimeRanges),
    Elevation(Elevation),
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

#[derive(Debug, Clone)]
pub enum LocationProvider {
    Manual(Location),
    Geoclue2,
}

#[derive(Debug, Clone)]
pub enum AdjustmentMethod {
    Randr { screen: u16 },
    Drm,
    VidMode,
}

pub struct Config {
    pub day: ColorProfile,
    pub night: ColorProfile,
    pub fade: bool,
    pub transition_scheme: TransitionScheme,
    pub location_provider: LocationProvider,
    pub adjustment_method: AdjustmentMethod,
}

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
            _ => s.parse::<f32>()?.try_into(),
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

impl LocationProvider {
    fn try_from(s: &str, manual: Option<Manual>) -> Result<Self> {
        match (s, manual) {
            (
                "manual",
                Some(Manual {
                    latitude: Some(lat),
                    longitude: Some(lon),
                }),
            ) => Ok(Self::Manual(Location {
                latitude: LatitudeDegree::try_from(lat)?,
                longitude: LongitudeDegree::try_from(lon)?,
            })),
            ("geoclue2", _) => Ok(Self::Geoclue2),
            // eprintln!("Latitude and longitude must be set.");
            _ => Err(anyhow!("location_provider")),
        }
    }
}

impl AdjustmentMethod {
    fn try_from(s: &str, randr: Option<Randr>) -> Result<Self> {
        match (s, randr) {
            ("randr", Some(Randr { screen: Some(n) })) => Ok(Self::Randr { screen: n }),
            ("randr", None) => Ok(Self::Randr { screen: 0 }),
            ("drm", _) => Ok(Self::Drm),
            ("vidmode", _) => Ok(Self::VidMode),
            _ => Err(anyhow!("adjustment_method")),
        }
    }
}

//

impl ConfigFile {
    pub fn new() -> Result<Self> {
        #[cfg(unix)]
        let system_config = PathBuf::from(formatcp!("/etc/{CARGO_PKG_NAME}/config.toml"));
        let user_config = dirs::config_dir().map(|d| d.join(CARGO_PKG_NAME).join("config.toml"));

        // merge config files and environment variables into a single struct
        let config_file = ConfigRs::builder();
        #[cfg(unix)]
        let config_file = config_file.add_source(File::from(system_config).required(false));
        let config_file = match user_config {
            Some(path) => config_file.add_source(File::from(path).required(false)),
            None => config_file,
        };
        let config_file: ConfigFile = config_file
            .add_source(Environment::with_prefix(CARGO_PKG_NAME))
            .build()?
            .try_deserialize()?;

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
            temperature: Default::default(),
            gamma: Default::default(),
            brightness: Default::default(),
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

impl Default for Config {
    fn default() -> Self {
        // TODO: replace magic numbers
        Config {
            day: ColorProfile::default_day(),
            night: ColorProfile::default_night(),
            fade: true,
            // TODO: are time_ranges and location_provider related together?
            transition_scheme: TransitionScheme::Elevation(Elevation { high: 3, low: -6 }),
            location_provider: LocationProvider::Manual(Location {
                latitude: LatitudeDegree(48.1),
                longitude: LongitudeDegree(11.6),
            }),
            adjustment_method: AdjustmentMethod::Randr { screen: 0 },
        }
    }
}

impl Config {
    pub fn new() -> Result<Self> {
        let ConfigFile {
            temperature,
            brightness,
            gamma,
            fade,
            elevation_high,
            elevation_low,
            time_dawn,
            time_dusk,
            location_provider,
            adjustment_method,
            manual,
            randr,
        } = ConfigFile::new()?;

        let mut config = Config::default();

        if let Some(t) = temperature {
            let DayNight { day, night }: DayNight<Temperature> = t.as_str().try_into()?;
            config.day.temperature = day;
            config.night.temperature = night;
        }
        if let Some(t) = brightness {
            let DayNight { day, night }: DayNight<Brightness> = t.as_str().try_into()?;
            config.day.brightness = day;
            config.night.brightness = night;
        }
        if let Some(t) = gamma {
            let DayNight { day, night }: DayNight<Gamma> = t.as_str().try_into()?;
            config.day.gamma = day;
            config.night.gamma = night;
        }
        if let Some(t) = fade {
            config.fade = t
        }

        // TODO:
        // match (elevation_high, elevation_low, time_dawn, time_dusk) {
        // }

        if let Some(t) = location_provider {
            config.location_provider = LocationProvider::try_from(&t, manual)?;
        }
        if let Some(t) = adjustment_method {
            config.adjustment_method = AdjustmentMethod::try_from(&t, randr)?;
        }

        Ok(config)
    }
}

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
