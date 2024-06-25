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

use crate::{
    cli::{
        CliArgs, CmdArgs, CmdInnerArgs, ColorSettingsArgs, InfoLevel,
        ModeArgs, Verbosity,
    },
    error::{
        config::{ConfigError, ConfigFileError},
        parse::DayNightErrorType,
    },
    types::{
        AdjustmentMethodType, BrightnessRange, ColorSettings, DayNight,
        GammaRange, LocationProviderType, Mode, TemperatureRange,
        TransitionScheme,
    },
    AdjustmentMethod, Drm, LocationProvider, Manual, Randr, Vidmode,
};
// use anyhow::{anyhow, Result};
use chrono::{DateTime, Local};
use clap::ColorChoice;
use clap::Parser;
use const_format::formatcp;
use serde::{de, Deserialize, Deserializer};
use std::{
    fmt::Display, fs::File, io::Read, marker::PhantomData, path::Path,
    str::FromStr, time::Duration,
};
use toml::Value;

pub const PKG_NAME: &str = env!("CARGO_PKG_NAME");
// Length of fade in numbers of fade's sleep durations
pub const FADE_STEPS: u8 = 40;
// Duration of sleep between screen updates (milliseconds)
pub const DEFAULT_SLEEP_DURATION: u64 = 5000;
pub const DEFAULT_SLEEP_DURATION_SHORT: u64 = 100;

pub const RANDR_MINOR_VERSION_MIN: u32 = 3;
pub const RANDR_MAJOR_VERSION: u32 = 1;

/// Merge of cli arguments and config files from highest priority to lowest:
/// 1. CLI arguments
/// 2. User config file
/// 3. System config file (Unix-like OS's only)
/// 4. Default values
#[derive(Debug)]
pub struct Config {
    pub mode: Mode,
    pub verbosity: Verbosity<InfoLevel>,
    pub color: ColorChoice,

    pub day: ColorSettings,
    pub night: ColorSettings,
    pub reset_ramps: bool,
    pub scheme: TransitionScheme,
    pub disable_fade: bool,
    pub sleep_duration: Duration,
    pub sleep_duration_short: Duration,

    pub location: LocationProvider,
    pub method: AdjustmentMethod,
    pub time: fn() -> DateTime<Local>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConfigBuilder {
    mode: Mode,
    verbosity: Verbosity<InfoLevel>,
    color: ColorChoice,

    day: ColorSettings,
    night: ColorSettings,
    reset_ramps: bool,
    disable_fade: bool,
    scheme: TransitionScheme,
    sleep_duration: Duration,
    sleep_duration_short: Duration,

    location: LocationProviderType,
    method: Option<AdjustmentMethodType>,
    time: fn() -> DateTime<Local>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct ConfigFile {
    temperature: Option<Either<u16, TemperatureRange>>,
    gamma: Option<Either<f64, GammaRange>>,
    brightness: Option<Either<f64, BrightnessRange>>,
    scheme: Option<TransitionScheme>,
    location: Option<LocationProviderType>,
    method: Option<AdjustmentMethodType>,
    reset_ramps: Option<bool>,
    disable_fade: Option<bool>,
    sleep_duration_short: Option<u16>,
    sleep_duration: Option<u16>,
}

#[derive(Debug, Clone, Default)]
struct Either<U: TryInto<T>, T> {
    t: T,
    p: PhantomData<U>,
}

impl ConfigBuilder {
    pub fn new() -> Result<Self, ConfigError> {
        let cli_args = CliArgs::parse();
        let mut cfg = Self::default();

        if let Some(path) = Self::config_path_from_mode(&cli_args.mode) {
            let config_file = ConfigFile::new(path)?;
            cfg.merge_with_config_file(config_file);
        }
        cfg.merge_with_cli_args(cli_args);

        Ok(cfg)
    }

    #[allow(clippy::too_many_lines)]
    pub fn build(self) -> Result<Config, ConfigError> {
        let Self {
            mode,
            verbosity,
            color,
            day,
            night,
            reset_ramps,
            disable_fade,
            scheme,
            sleep_duration,
            sleep_duration_short,
            location,
            method,
            time,
        } = self;

        // try all methods until one that works is found.
        // Gamma adjustment not needed for print mode
        //     // Try all methods, use the first that works.
        //     // b"Trying next method...\n\0" as *const u8 as *const c_char,
        //     // b"Using method `%s'.\n\0" as *const u8 as *const c_char,
        //     // Failure if no methods were successful at this point.
        //     // b"No more methods to try.\n\0" as *const u8 as *const c_char,

        let method = match mode {
            Mode::Print => AdjustmentMethodType::Dummy,
            _ => method.ok_or(ConfigError::Wip)?,
        };

        let method = match method {
            AdjustmentMethodType::Dummy => {
                AdjustmentMethod::Dummy(Default::default())
            }
            AdjustmentMethodType::Drm { card_num, crtcs } => {
                AdjustmentMethod::Drm(Drm::new(card_num, crtcs)?)
            }
            AdjustmentMethodType::Randr { screen_num, crtcs } => {
                AdjustmentMethod::Randr(Randr::new(screen_num, crtcs)?)
            }
            AdjustmentMethodType::Vidmode { screen_num } => {
                AdjustmentMethod::Vidmode(Vidmode::new(screen_num)?)
            }
        };

        let location = match location {
            LocationProviderType::Manual(l) => {
                LocationProvider::Manual(Manual::new(l))
            }
            LocationProviderType::Geoclue2 => {
                LocationProvider::Geoclue2(Default::default())
            }
        };

        let c = Config {
            mode,
            color,
            verbosity,
            day,
            night,
            reset_ramps,
            scheme,
            disable_fade,
            sleep_duration_short,
            sleep_duration,
            location,
            method,
            time,
        };

        Ok(c)
    }

    fn config_path_from_mode(mode: &ModeArgs) -> Option<Option<&Path>> {
        match mode {
            ModeArgs::Print { .. } => None,
            ModeArgs::Daemon {
                c:
                    CmdArgs {
                        i: CmdInnerArgs { config, .. },
                        ..
                    },
                ..
            }
            | ModeArgs::Oneshot {
                c:
                    CmdArgs {
                        i: CmdInnerArgs { config, .. },
                        ..
                    },
            }
            | ModeArgs::Set {
                i: CmdInnerArgs { config, .. },
                ..
            }
            | ModeArgs::Reset {
                i: CmdInnerArgs { config, .. },
            } => Some(config.as_deref()),
        }
    }

    #[allow(clippy::too_many_lines)]
    fn merge_with_cli_args(&mut self, cli_args: CliArgs) {
        let CliArgs {
            mode,
            verbosity,
            color,
        } = cli_args;
        self.verbosity = verbosity;
        if let Some(t) = color {
            self.color = t;
        }

        match mode {
            ModeArgs::Daemon {
                c,
                disable_fade,
                sleep_duration,
                sleep_duration_short,
            } => {
                if let Some(t) = sleep_duration {
                    self.sleep_duration = Duration::from_millis(t as u64);
                }
                if let Some(t) = sleep_duration_short {
                    self.sleep_duration_short =
                        Duration::from_millis(t as u64);
                }
                if let Some(t) = disable_fade {
                    self.disable_fade = t;
                }
                self.merge_with_cmd_args(c);
                self.mode = Mode::Daemon;
            }
            ModeArgs::Oneshot { c } => {
                self.merge_with_cmd_args(c);
                self.mode = Mode::Oneshot;
            }
            ModeArgs::Set { cs, i } => {
                self.merge_with_inner_cmd_args(i);
                self.day = cs.into();
                self.mode = Mode::Set;
            }
            ModeArgs::Reset { i } => {
                self.merge_with_inner_cmd_args(i);
                self.mode = Mode::Reset;
            }
            ModeArgs::Print { location } => {
                self.location = location;
                self.mode = Mode::Print;
            }
        }
    }

    fn merge_with_cmd_args(&mut self, args: CmdArgs) {
        let CmdArgs {
            temperature,
            brightness,
            gamma,
            scheme,
            location,
            i,
        } = args;

        if let Some(t) = temperature {
            self.day.temp = t.day;
            self.night.temp = t.night;
        }
        if let Some(t) = brightness {
            self.day.brght = t.day;
            self.night.brght = t.night;
        }
        if let Some(t) = gamma {
            self.day.gamma = t.day;
            self.night.gamma = t.night;
        }

        if let Some(t) = scheme {
            self.scheme = t;
        }
        if let Some(t) = location {
            self.location = t;
        }
        self.merge_with_inner_cmd_args(i);
    }

    fn merge_with_inner_cmd_args(&mut self, args: CmdInnerArgs) {
        let CmdInnerArgs {
            config: _,
            reset_ramps,
            method,
        } = args;

        if let Some(t) = reset_ramps {
            self.reset_ramps = t;
        }
        if let Some(t) = method {
            self.method = Some(t);
        }
    }

    #[allow(clippy::too_many_lines)]
    fn merge_with_config_file(&mut self, config: ConfigFile) {
        let ConfigFile {
            temperature,
            brightness,
            gamma,
            reset_ramps,
            scheme,
            disable_fade,
            sleep_duration_short,
            sleep_duration,
            method,
            location,
        } = config;

        if let Some(t) = temperature {
            self.day.temp = t.t.day;
            self.night.temp = t.t.night;
        }
        if let Some(t) = brightness {
            self.day.brght = t.t.day;
            self.night.brght = t.t.night;
        }
        if let Some(t) = gamma {
            self.day.gamma = t.t.day;
            self.night.gamma = t.t.night;
        }

        if let Some(t) = reset_ramps {
            self.reset_ramps = t;
        }
        if let Some(t) = scheme {
            self.scheme = t;
        }
        if let Some(t) = disable_fade {
            self.disable_fade = t;
        }

        if let Some(t) = sleep_duration_short {
            self.sleep_duration_short = Duration::from_millis(t as u64);
        }
        if let Some(t) = sleep_duration {
            self.sleep_duration = Duration::from_millis(t as u64);
        }

        if let Some(t) = location {
            self.location = t;
        }
        if let Some(t) = method {
            self.method = Some(t);
        }
    }
}

impl ConfigFile {
    fn new(config_path: Option<&Path>) -> Result<Self, ConfigFileError> {
        #[cfg(unix)]
        let system_config =
            Path::new(formatcp!("/etc/{PKG_NAME}/config.toml"));
        let local_config =
            dirs::config_dir().map(|d| d.join(PKG_NAME).join("config.toml"));
        let user_config = config_path
            .map(|p| match p.is_file() {
                true => Ok(p),
                false => Err(ConfigFileError::PathNotFile),
            })
            .transpose()?
            .or(local_config.as_deref())
            .ok_or(ConfigFileError::ConfigDirNotFound)?;

        let mut config = Self::default();
        let mut buf = String::new();
        let mut read = |path: &Path| -> Result<(), ConfigFileError> {
            if path.is_file() {
                File::open(path)
                    .map_err(ConfigFileError::OpenFailed)?
                    .read_to_string(&mut buf)
                    .map_err(ConfigFileError::OpenFailed)?;
                let cfg = toml::from_str(&buf)
                    .map_err(ConfigFileError::DeserializeFailed)?;
                config.merge(cfg);
                Ok(())
            } else {
                Ok(())
            }
        };

        #[cfg(unix)]
        read(system_config)?;
        read(user_config)?;
        Ok(config)
    }

    fn merge(&mut self, other: Self) {
        let Self {
            temperature,
            brightness,
            gamma,
            reset_ramps,
            scheme,
            disable_fade,
            sleep_duration_short,
            sleep_duration,
            method,
            location,
        } = other;

        if let Some(t) = temperature {
            self.temperature = Some(t);
        }
        if let Some(t) = brightness {
            self.brightness = Some(t);
        }
        if let Some(t) = gamma {
            self.gamma = Some(t);
        }
        self.reset_ramps = reset_ramps;
        self.disable_fade = disable_fade;
        if let Some(t) = scheme {
            self.scheme = Some(t);
        }

        if let Some(t) = sleep_duration {
            self.sleep_duration = Some(t);
        }
        if let Some(t) = sleep_duration_short {
            self.sleep_duration_short = Some(t);
        }

        if let Some(t) = location {
            self.location = Some(t);
        }
        if let Some(t) = method {
            self.method = Some(t);
        }
    }
}

//

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self {
            day: ColorSettings::default_day(),
            night: ColorSettings::default_night(),
            mode: Default::default(),
            verbosity: Default::default(),
            color: Default::default(),
            reset_ramps: Default::default(),
            scheme: Default::default(),
            disable_fade: Default::default(),
            sleep_duration_short: Duration::from_millis(
                DEFAULT_SLEEP_DURATION_SHORT,
            ),
            sleep_duration: Duration::from_millis(DEFAULT_SLEEP_DURATION),
            method: Default::default(),
            location: Default::default(),
            time: Local::now,
        }
    }
}

impl From<ColorSettingsArgs> for ColorSettings {
    fn from(t: ColorSettingsArgs) -> Self {
        let mut color_settings = Self::default();
        let ColorSettingsArgs {
            temperature,
            gamma,
            brightness,
        } = t;
        if let Some(t) = temperature {
            color_settings.temp = t;
        }
        if let Some(t) = brightness {
            color_settings.brght = t;
        }
        if let Some(t) = gamma {
            color_settings.gamma = t;
        }
        color_settings
    }
}

impl<'de, T, U> Deserialize<'de> for Either<U, T>
where
    T: Deserialize<'de>,
    U: Deserialize<'de> + TryInto<T>,
    U::Error: Display,
{
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = Value::deserialize(d)?;
        let t = match U::deserialize(v.clone()) {
            Ok(u) => u.try_into().map_err(de::Error::custom)?,
            Err(_) => match T::deserialize(v) {
                Ok(t) => t,
                Err(e) => Err(de::Error::custom(e))?,
            },
        };

        Ok(Self { t, p: PhantomData })
    }
}

impl<'de, E, T> Deserialize<'de> for DayNight<T>
where
    E: DayNightErrorType,
    T: Clone + FromStr<Err = E>,
{
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        String::deserialize(d)?.parse().map_err(de::Error::custom)
    }
}

impl<'de> Deserialize<'de> for TransitionScheme {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        String::deserialize(d)?.parse().map_err(de::Error::custom)
    }
}

impl<'de> Deserialize<'de> for LocationProviderType {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        String::deserialize(d)?.parse().map_err(de::Error::custom)
    }
}

impl<'de> Deserialize<'de> for AdjustmentMethodType {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        String::deserialize(d)?.parse().map_err(de::Error::custom)
    }
}
