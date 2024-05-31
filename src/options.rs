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
use clap::{Args, Parser, Subcommand};
use const_format::formatcp;
use serde::{de::Error as DeError, Deserialize, Deserializer};
use std::{env, fs::File, io::Read, path::PathBuf};

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
pub const DEFAULT_TIME_RANGE_DAWN: &str = "06:00-07:00";
pub const DEFAULT_TIME_RANGE_DUSK: &str = "18:00-19:00";
// TODO: find something generic
pub const DEFAULT_LATITUDE: f32 = 48.1;
pub const DEFAULT_LONGITUDE: f32 = 11.6;

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
    formatcp!(
        "Usage: {CARGO_PKG_NAME} -l LAT:LON -t DAY:NIGHT [OPTIONS...]

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

The neutral temperature is {DEFAULT_TEMPERATURE}K. Using this value will not change the color
temperature of the display. Setting the color temperature to a value higher
than this results in more blue light, and setting a lower value will result in
more red light.

Default values:
  Daytime temperature: {DEFAULT_TEMPERATURE_DAY}K
  Night temperature: {DEFAULT_TEMPERATURE_NIGHT}K

Please report bugs to <{PKG_BUGREPORT}>"
    )
};

fn print_method_list() {
    println!("Available adjustment methods:");

    // let mut i: c_int = 0 as c_int;
    // while !((*gamma_methods.offset(i as isize)).name).is_null() {
    //     let name = (*gamma_methods.offset(i as isize)).name;
    //     let name = CStr::from_ptr(name).to_str().unwrap();
    //     println!("  {name}");
    //     i += 1;
    // }

    // TRANSLATORS: `help' must not be translated.
    println!(
        "
Specify colon-separated options with `-m METHOD:OPTIONS`
Try `-m METHOD:help' for help."
    );
}

fn print_provider_list() {
    println!("Available location providers:");

    // let mut i: c_int = 0 as c_int;
    // while !((*location_providers.offset(i as isize)).name).is_null() {
    //     let name = (*location_providers.offset(i as isize)).name;
    //     let name = CStr::from_ptr(name).to_str().unwrap();
    //     println!("  {name}");
    //     i += 1;
    // }

    // TRANSLATORS: `help' must not be translated.
    println!(
        "
Specify colon-separated options with`-l PROVIDER:OPTIONS'.
Try `-l PROVIDER:help' for help.
"
    );
}

//
// CLI Arguments
//

#[derive(Parser)]
#[command(version, about)]
#[command(propagate_version = true)]
struct CliArgs {
    #[command(subcommand)]
    mode: Option<ModeArgs>,
    #[arg(long, short, value_name = "FILE")]
    config: Option<PathBuf>,
    #[command(flatten)]
    verbosity: VerbosityArgs,
    #[arg(long, short)]
    dry_run: bool,
}

#[derive(Args)]
#[group(multiple = false)]
struct VerbosityArgs {
    #[arg(long, short)]
    quite: bool,
    #[arg(long, short)]
    verbose: bool,
}

#[derive(Subcommand)]
enum ModeArgs {
    Daemon(Config),
    OneShot(Config),
    Reset(ConfigArgs),
    Set {
        #[command(flatten)]
        cs: ColorSettingArgs,
        #[command(flatten)]
        ca: ConfigArgs,
    },
}

#[derive(Args)]
#[group(required = true, multiple = true)]
struct ColorSettingArgs {
    #[arg(long, short, value_parser = temperature)]
    temperature: Option<Temperature>,
    #[arg(long, short, value_parser = gamma)]
    gamma: Option<Gamma>,
    #[arg(long, short, value_parser = brightness)]
    brightness: Option<Brightness>,
}

//
// Config file
//

#[derive(Debug, Clone, Deserialize, Args)]
#[group(requires_all = ["dawn", "dusk"])]
struct TimeRangesArgs {
    #[arg(long, value_parser = time_range)]
    dawn: Option<TimeRange>,
    #[arg(long, value_parser = time_range)]
    dusk: Option<TimeRange>,
}

#[derive(Debug, Clone, Deserialize, Args)]
#[group(requires_all = ["high", "low"])]
struct ElevationArgs {
    #[arg(long, value_parser = elevation)]
    high: Option<Elevation>,
    #[arg(long, value_parser = elevation)]
    low: Option<Elevation>,
}

#[derive(Debug, Clone, Deserialize, Args)]
struct LocationArgs {
    #[arg(long, value_parser = latitude)]
    latitude: Option<Latitude>,
    #[arg(long, value_parser = longitude)]
    longitude: Option<Longitude>,
}

#[derive(Debug, Clone, Deserialize, Args)]
struct RandrArgs {
    #[arg(long)]
    screen: Option<u16>,
}

/// Part of [Config] that is used in [ModeArgs::Set] and [ModeArgs::Reset]
#[derive(Debug, Clone, Default, Deserialize, Args)]
struct ConfigArgs {
    #[serde(default = "return_true")]
    #[arg(long)]
    preserve_gamma: bool,
    #[serde(default = "return_true")]
    #[arg(long)]
    fade: bool,

    #[arg(long, short, value_parser = adjustment_method_kind)]
    adjustment_method: Option<AdjustmentMethodKind>,
    #[command(flatten)]
    randr: Option<RandrArgs>,
}

#[derive(Debug, Clone, Default, Deserialize, Args)]
struct Config {
    #[arg(long, value_parser = day_night_temperature)]
    temperature: Option<DayNight<Temperature>>,
    #[arg(long, value_parser = day_night_brightness)]
    brightness: Option<DayNight<Brightness>>,
    #[arg(long, value_parser = day_night_gamma)]
    gamma: Option<DayNight<Gamma>>,

    #[arg(long, short, value_parser = transition_scheme_kind)]
    transition_scheme: Option<TransitionSchemeKind>,
    #[command(flatten)]
    time_ranges: Option<TimeRangesArgs>,
    #[command(flatten)]
    elevation: Option<ElevationArgs>,

    #[arg(long, short, value_parser = location_provider_kind)]
    location_provider: Option<LocationProviderKind>,
    #[arg(long, value_parser = location)]
    manual: Option<Location>,

    #[serde(flatten)]
    #[command(flatten)]
    c: ConfigArgs,
}

//
// Parsed types
//

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Temperature(u16);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Brightness(f32);

#[derive(Debug, Clone, PartialEq)]
pub struct Gamma([f32; 3]);

#[derive(Debug, Clone, PartialEq)]
pub struct ColorSetting {
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

/// Offset from midnight in seconds
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimeOffset(u32);

#[derive(Debug, Clone, Copy)]
pub struct TimeRange {
    pub start: TimeOffset,
    pub end: TimeOffset,
}

#[derive(Debug, Clone)]
pub struct TimeRanges {
    pub dawn: TimeRange,
    pub dusk: TimeRange,
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Elevation(f32);

// The solar elevations at which the transition begins/ends,
#[derive(Debug, Clone, Copy)]
pub struct ElevationRange {
    pub high: Elevation,
    pub low: Elevation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum TransitionSchemeKind {
    TimeRanges,
    Elevation,
}

#[derive(Debug, Clone)]
pub struct TransitionScheme {
    pub select: TransitionSchemeKind,
    pub time_ranges: TimeRanges,
    pub elevation_range: ElevationRange,
}

#[derive(Debug, Clone, Copy)]
pub struct Latitude(f32);
#[derive(Debug, Clone, Copy)]
pub struct Longitude(f32);
#[derive(Debug, Clone, Copy, Default)]
pub struct Location {
    pub latitude: Latitude,
    pub longitude: Longitude,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub enum LocationProviderKind {
    Manual,
    GeoClue2,
}

#[derive(Debug, Clone)]
pub struct LocationProvider {
    pub select: LocationProviderKind,
    pub manual: Location,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub enum AdjustmentMethodKind {
    Randr,
    Drm,
    VidMode,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Randr {
    pub screen: u16,
}

#[derive(Debug, Clone)]
pub struct AdjustmentMethod {
    pub select: AdjustmentMethodKind,
    pub randr: Randr,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum Mode {
    #[default]
    Daemon,
    OneShot,
    Set,
    Reset,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verbosity {
    Quite,
    #[default]
    Low,
    High,
}

// merge of cli arguments and config files
#[derive(Debug, Clone)]
pub struct Options {
    pub verbosity: Verbosity,
    pub dry_run: bool,
    pub mode: Mode,

    pub day: ColorSetting,
    pub night: ColorSetting,
    pub preserve_gamma: bool,
    pub fade: bool,
    pub scheme: TransitionScheme,
    pub provider: LocationProvider,
    pub method: AdjustmentMethod,
}

//
// Merge from highest priority to lowest:
// 1. cli arguments
// 2. user config file
// 3. system config file
// 4. default options
//

impl Options {
    pub fn new() -> Result<Self> {
        let args = CliArgs::parse();
        // let config = Config::new(args.config)?;
        let mut options = Options::default();
        // options.merge_with_config(&config);
        options.merge_with_args(args)?;
        Ok(options)
    }

    fn merge_with_args(&mut self, args: CliArgs) -> Result<()> {
        let CliArgs {
            config: _,
            verbosity: VerbosityArgs { quite, verbose },
            dry_run,
            mode,
        } = args;

        let verbosity = match (quite, verbose) {
            (true, false) => Verbosity::Quite,
            (false, false) => Verbosity::Low,
            (false, true) => Verbosity::High,
            (true, true) => unreachable!(), // clap returns error
        };

        self.verbosity = verbosity;
        self.dry_run = dry_run;
        match mode {
            Some(ModeArgs::Daemon(c)) => {
                self.merge_with_config(c)?;
                self.mode = Mode::Daemon;
            }
            Some(ModeArgs::OneShot(c)) => {
                self.merge_with_config(c)?;
                self.mode = Mode::OneShot;
            }
            Some(ModeArgs::Set { cs, ca }) => {
                self.merge_with_config_args(ca)?;
                self.day = cs.into();
                self.mode = Mode::Set;
            }
            Some(ModeArgs::Reset(ca)) => {
                self.merge_with_config_args(ca)?;
                self.mode = Mode::Reset;
            }
            None => {}
        }

        Ok(())
    }

    fn merge_with_config_args(&mut self, config_args: ConfigArgs) -> Result<()> {
        let ConfigArgs {
            preserve_gamma,
            fade,
            adjustment_method,
            randr,
        } = config_args;

        self.preserve_gamma = preserve_gamma;
        self.fade = fade;
        if let Some(t) = adjustment_method {
            self.method.select = t;
        }
        if let Some(t) = randr {
            self.method.randr = t.try_into()?;
        }

        Ok(())
    }

    fn merge_with_config(&mut self, config: Config) -> Result<()> {
        // TODO: move conversions to ConfigFile filds definition with serde derives
        let Config {
            temperature,
            brightness,
            gamma,
            transition_scheme,
            time_ranges,
            elevation,
            location_provider,
            manual,
            c,
        } = config;

        if let Some(DayNight { day, night }) = temperature {
            self.day.temperature = day;
            self.night.temperature = night;
        }
        if let Some(DayNight { day, night }) = brightness {
            self.day.brightness = day;
            self.night.brightness = night;
        }
        if let Some(DayNight { day, night }) = gamma {
            self.day.gamma = day;
            self.night.gamma = night;
        }

        if let Some(t) = transition_scheme {
            self.scheme.select = t
        }
        if let Some(t) = time_ranges {
            self.scheme.time_ranges = t.try_into()?;
        }
        if let Some(t) = elevation {
            self.scheme.elevation_range = t.try_into()?;
        }

        if let Some(t) = location_provider {
            self.provider.select = t;
        }
        if let Some(t) = manual {
            self.provider.manual = t;
        }

        self.merge_with_config_args(c)
    }
}

impl Config {
    fn new(config_path: Option<PathBuf>) -> Result<Self> {
        #[cfg(unix)]
        let system_config = PathBuf::from(formatcp!("/etc/{CARGO_PKG_NAME}/config.toml"));
        let user_config = config_path
            .or_else(|| dirs::config_dir().map(|d| d.join(CARGO_PKG_NAME).join("config.toml")))
            .ok_or(anyhow!("user_config"))?;

        let mut buf = String::new();
        let mut read = |path| -> Result<Config> {
            File::open(path)?.read_to_string(&mut buf)?;
            Ok(toml::from_str(&buf)?)
        };

        let mut config = Config::default();
        #[cfg(unix)]
        config.merge(read(system_config)?);
        config.merge(read(user_config)?);
        Ok(config)
    }

    fn merge(&mut self, other: Self) {
        let Config {
            temperature,
            brightness,
            gamma,
            transition_scheme,
            time_ranges,
            elevation,
            location_provider,
            manual,
            c:
                ConfigArgs {
                    preserve_gamma,
                    fade,
                    adjustment_method,
                    randr,
                },
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
        if let Some(t) = transition_scheme {
            self.transition_scheme = Some(t);
        }
        if let Some(t) = time_ranges {
            self.time_ranges = Some(t);
        }
        if let Some(t) = elevation {
            self.elevation = Some(t);
        }
        if let Some(t) = location_provider {
            self.location_provider = Some(t);
        }
        if let Some(t) = manual {
            self.manual = Some(t);
        }

        self.c.preserve_gamma = preserve_gamma;
        self.c.fade = fade;
        if let Some(t) = adjustment_method {
            self.c.adjustment_method = Some(t);
        }
        if let Some(t) = randr {
            self.c.randr = Some(t);
        }
    }
}

impl Default for Options {
    fn default() -> Self {
        Options {
            preserve_gamma: true,
            fade: true,
            mode: Default::default(),
            verbosity: Default::default(),
            dry_run: Default::default(),
            day: Default::default(),
            night: Default::default(),
            scheme: Default::default(),
            provider: Default::default(),
            method: Default::default(),
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

impl<'a, 'b: 'a, T> TryFrom<&'b str> for DayNight<T>
where
    T: Clone + TryFrom<&'a str, Error = anyhow::Error>,
{
    type Error = anyhow::Error;

    fn try_from(s: &'b str) -> Result<Self, Self::Error> {
        match *s.split("-").map(str::trim).collect::<Vec<_>>().as_slice() {
            [day, night] => Ok(Self {
                day: day.try_into()?,
                night: night.try_into()?,
            }),
            _ => {
                let temp = T::try_from(s)?;
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
            // b"Temperature must be between %uK and %uK.\n\0" as *const u8 as *const c_char,
            Err(anyhow!("temperature"))
        }
    }
}

impl TryFrom<&str> for Temperature {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse::<u16>()?.try_into()
    }
}

impl TryFrom<f32> for Brightness {
    type Error = anyhow::Error;

    fn try_from(n: f32) -> Result<Self, Self::Error> {
        if n >= MIN_BRIGHTNESS && n <= MAX_BRIGHTNESS {
            Ok(Self(n))
        } else {
            // b"Brightness values must be between %.1f and %.1f.\n\0" as *const u8 as *const c_char,
            Err(anyhow!("brightness"))
        }
    }
}

impl TryFrom<&str> for Brightness {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse::<f32>()?.try_into()
    }
}

fn is_gamma(n: f32) -> Result<f32> {
    if n >= MIN_GAMMA && n <= MAX_GAMMA {
        Ok(n)
    } else {
        // b"Gamma value must be between %.1f and %.1f.\n\0" as *const u8 as *const c_char,
        Err(anyhow!("gamma"))
    }
}

impl TryFrom<f32> for Gamma {
    type Error = anyhow::Error;

    fn try_from(n: f32) -> Result<Self, Self::Error> {
        let n = is_gamma(n)?;
        Ok(Self([n; 3]))
    }
}

impl TryFrom<[f32; 3]> for Gamma {
    type Error = anyhow::Error;

    fn try_from([r, g, b]: [f32; 3]) -> Result<Self, Self::Error> {
        Ok(Self([is_gamma(r)?, is_gamma(g)?, is_gamma(b)?]))
    }
}

impl TryFrom<Vec<f32>> for Gamma {
    type Error = anyhow::Error;

    fn try_from(vec: Vec<f32>) -> Result<Self, Self::Error> {
        TryInto::<[f32; 3]>::try_into(vec)
            .map_err(|_| anyhow!("wrong size"))?
            .try_into()
    }
}

impl TryFrom<&str> for Gamma {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match *s.split(":").map(str::trim).collect::<Vec<_>>().as_slice() {
            [r, g, b] => [r.parse::<f32>()?, g.parse::<f32>()?, b.parse::<f32>()?].try_into(),
            [rbg] => rbg.parse::<f32>()?.try_into(),
            _ => Err(anyhow!("gamma")),
        }
    }
}

impl TryFrom<u8> for Hour {
    type Error = anyhow::Error;

    fn try_from(n: u8) -> Result<Self, Self::Error> {
        if n < 24 {
            Ok(Self(n))
        } else {
            Err(anyhow!("hour"))
        }
    }
}

impl TryFrom<&str> for Hour {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse::<u8>()?.try_into()
    }
}

impl TryFrom<u8> for Minute {
    type Error = anyhow::Error;

    fn try_from(n: u8) -> Result<Self, Self::Error> {
        if n < 60 {
            Ok(Self(n))
        } else {
            Err(anyhow!("minute"))
        }
    }
}

impl TryFrom<&str> for Minute {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse::<u8>()?.try_into()
    }
}

impl TryFrom<&str> for Time {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match *s.split(":").map(str::trim).collect::<Vec<_>>().as_slice() {
            [hour, minute] => Ok(Self {
                hour: hour.try_into()?,
                minute: minute.try_into()?,
            }),
            _ => Err(anyhow!("time")),
        }
    }
}

impl From<Time> for TimeOffset {
    fn from(Time { hour, minute }: Time) -> Self {
        Self((*hour.as_ref() as u32 * 60 * 60) + (*minute.as_ref() as u32 * 60))
    }
}

impl TryFrom<&str> for TimeRange {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match *s.split("-").map(str::trim).collect::<Vec<_>>().as_slice() {
            [start, end] => {
                let start: TimeOffset = Time::try_from(start)?.into();
                let end: TimeOffset = Time::try_from(end)?.into();

                if start <= end {
                    Ok(Self { start, end })
                } else {
                    Err(anyhow!("time_range"))
                }
            }
            [s] => {
                let t: TimeOffset = Time::try_from(s)?.into();
                Ok(Self { start: t, end: t })
            }
            _ => Err(anyhow!("time_range")),
        }
    }
}

impl TryFrom<f32> for Elevation {
    type Error = anyhow::Error;

    fn try_from(n: f32) -> Result<Self, Self::Error> {
        // TODO: any bound? probably lower than a certain degree
        Ok(Self(n))
    }
}

impl TryFrom<&str> for Elevation {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse::<f32>()?.try_into()
    }
}

impl TryFrom<(Elevation, Elevation)> for ElevationRange {
    type Error = anyhow::Error;

    fn try_from((high, low): (Elevation, Elevation)) -> Result<Self, Self::Error> {
        if high >= low {
            Ok(Self { high, low })
        } else {
            // b"High transition elevation cannot be lower than the low transition elevation.\n\0"
            Err(anyhow!("elevation"))
        }
    }
}

impl TryFrom<&str> for ElevationRange {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match *s.split(":").map(str::trim).collect::<Vec<_>>().as_slice() {
            [high, low] => (
                Elevation::try_from(high.parse::<f32>()?)?,
                Elevation::try_from(low.parse::<f32>()?)?,
            )
                .try_into(),
            _ => Err(anyhow!("elevation")),
        }
    }
}

impl TryFrom<f32> for Latitude {
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

impl TryFrom<f32> for Longitude {
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

impl TryFrom<&str> for Longitude {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse::<f32>()?.try_into()
    }
}

impl TryFrom<&str> for Latitude {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse::<f32>()?.try_into()
    }
}

impl TryFrom<&str> for Location {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match *s.split(":").map(str::trim).collect::<Vec<_>>().as_slice() {
            [lat, lon] => Ok(Self {
                latitude: lat.try_into()?,
                longitude: lon.try_into()?,
            }),
            _ => Err(anyhow!("location")),
        }
    }
}

impl TryFrom<LocationArgs> for Location {
    type Error = anyhow::Error;

    fn try_from(t: LocationArgs) -> Result<Self, Self::Error> {
        match t {
            LocationArgs {
                latitude: Some(latitude),
                longitude: Some(longitude),
            } => Ok(Self {
                latitude,
                longitude,
            }),
            _ => Err(anyhow!("location")),
        }
    }
}

impl TryFrom<TimeRangesArgs> for TimeRanges {
    type Error = anyhow::Error;

    fn try_from(t: TimeRangesArgs) -> Result<Self, Self::Error> {
        match t {
            TimeRangesArgs {
                dawn: Some(dawn),
                dusk: Some(dusk),
            } => Ok(Self { dawn, dusk }),
            _ => Err(anyhow!("time_ranges")),
        }
    }
}

impl TryFrom<ElevationArgs> for ElevationRange {
    type Error = anyhow::Error;

    fn try_from(t: ElevationArgs) -> Result<Self, Self::Error> {
        match t {
            ElevationArgs {
                high: Some(high),
                low: Some(low),
            } => (high, low).try_into(),
            _ => Err(anyhow!("elevation")),
        }
    }
}

impl TryFrom<RandrArgs> for Randr {
    type Error = anyhow::Error;

    fn try_from(t: RandrArgs) -> Result<Self, Self::Error> {
        match t {
            RandrArgs { screen: Some(scr) } => Ok(Self { screen: scr }),
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
            _ => Err(anyhow!("transition_scheme")),
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

impl From<ColorSettingArgs> for ColorSetting {
    fn from(t: ColorSettingArgs) -> Self {
        let mut color_settings = Self::default();
        let ColorSettingArgs {
            temperature,
            gamma,
            brightness,
        } = t;
        if let Some(t) = temperature {
            color_settings.temperature = t;
        }
        if let Some(t) = brightness {
            color_settings.brightness = t;
        }
        if let Some(t) = gamma {
            color_settings.gamma = t;
        }
        color_settings
    }
}

//
// boilerplates
//

impl<'de, T> Deserialize<'de> for DayNight<T>
where
    T: Clone + for<'a> TryFrom<&'a str, Error = anyhow::Error>,
{
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let t = String::deserialize(d)?;
        Self::try_from(t.as_str()).map_err(|e| DeError::custom(e))
    }
}

impl<'de> Deserialize<'de> for Temperature {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let t = u16::deserialize(d)?;
        Self::try_from(t).map_err(|e| DeError::custom(e))
    }
}

impl<'de> Deserialize<'de> for Brightness {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let t = f32::deserialize(d)?;
        Self::try_from(t).map_err(|e| DeError::custom(e))
    }
}

impl<'de> Deserialize<'de> for Gamma {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let t = f32::deserialize(d)?;
        Self::try_from(t).map_err(|e| DeError::custom(e))
    }
}

impl<'de> Deserialize<'de> for TimeRange {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let t = String::deserialize(d)?;
        Self::try_from(t.as_str()).map_err(|e| DeError::custom(e))
    }
}

impl<'de> Deserialize<'de> for Elevation {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let t = f32::deserialize(d)?;
        Self::try_from(t).map_err(|e| DeError::custom(e))
    }
}

impl<'de> Deserialize<'de> for Latitude {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let t = f32::deserialize(d)?;
        Self::try_from(t).map_err(|e| DeError::custom(e))
    }
}

impl<'de> Deserialize<'de> for Longitude {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let t = f32::deserialize(d)?;
        Self::try_from(t).map_err(|e| DeError::custom(e))
    }
}

impl<'de> Deserialize<'de> for Location {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let t = String::deserialize(d)?;
        Self::try_from(t.as_str()).map_err(|e| DeError::custom(e))
    }
}

// clap_derive doesn't accept the trait form of the functions for some reason
fn temperature(t: &str) -> Result<Temperature> {
    t.try_into()
}
fn brightness(t: &str) -> Result<Brightness> {
    t.try_into()
}
fn gamma(t: &str) -> Result<Gamma> {
    t.try_into()
}

fn day_night_temperature(t: &str) -> Result<DayNight<Temperature>> {
    t.try_into()
}
fn day_night_brightness(t: &str) -> Result<DayNight<Brightness>> {
    t.try_into()
}
fn day_night_gamma(t: &str) -> Result<DayNight<Gamma>> {
    t.try_into()
}

fn time_range(t: &str) -> Result<TimeRange> {
    t.try_into()
}
fn elevation(t: &str) -> Result<Elevation> {
    t.try_into()
}
fn latitude(t: &str) -> Result<Latitude> {
    t.try_into()
}
fn longitude(t: &str) -> Result<Longitude> {
    t.try_into()
}

fn location(t: &str) -> Result<Location> {
    t.try_into()
}
fn transition_scheme_kind(t: &str) -> Result<TransitionSchemeKind> {
    t.try_into()
}
fn adjustment_method_kind(t: &str) -> Result<AdjustmentMethodKind> {
    t.try_into()
}
fn location_provider_kind(t: &str) -> Result<LocationProviderKind> {
    t.try_into()
}

fn return_true() -> bool {
    true
}

//

impl AsRef<u16> for Temperature {
    fn as_ref(&self) -> &u16 {
        &self.0
    }
}

impl AsRef<f32> for Brightness {
    fn as_ref(&self) -> &f32 {
        &self.0
    }
}

impl AsRef<[f32; 3]> for Gamma {
    fn as_ref(&self) -> &[f32; 3] {
        &self.0
    }
}

impl AsRef<u8> for Hour {
    fn as_ref(&self) -> &u8 {
        &self.0
    }
}

impl AsRef<u8> for Minute {
    fn as_ref(&self) -> &u8 {
        &self.0
    }
}

impl AsRef<u32> for TimeOffset {
    fn as_ref(&self) -> &u32 {
        &self.0
    }
}

impl AsRef<f32> for Elevation {
    fn as_ref(&self) -> &f32 {
        &self.0
    }
}

impl AsRef<f32> for Latitude {
    fn as_ref(&self) -> &f32 {
        &self.0
    }
}

impl AsRef<f32> for Longitude {
    fn as_ref(&self) -> &f32 {
        &self.0
    }
}

impl Default for Temperature {
    fn default() -> Self {
        Self(DEFAULT_TEMPERATURE)
    }
}

//

impl Default for Brightness {
    fn default() -> Self {
        Brightness(DEFAULT_BRIGHTNESS)
    }
}

impl Default for Gamma {
    fn default() -> Self {
        Gamma([DEFAULT_GAMMA; 3])
    }
}

impl Default for ColorSetting {
    fn default() -> Self {
        Self {
            temperature: Temperature::default(),
            gamma: Gamma::default(),
            brightness: Brightness::default(),
        }
    }
}

impl ColorSetting {
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

impl Default for ElevationRange {
    fn default() -> Self {
        Self {
            high: Elevation(DEFAULT_ELEVATION_HIGH),
            low: Elevation(DEFAULT_ELEVATION_LOW),
        }
    }
}

impl Default for TimeRanges {
    fn default() -> Self {
        Self {
            dawn: DEFAULT_TIME_RANGE_DAWN
                .try_into()
                .unwrap_or_else(|_| unreachable!()),
            dusk: DEFAULT_TIME_RANGE_DUSK
                .try_into()
                .unwrap_or_else(|_| unreachable!()),
        }
    }
}

impl Default for TransitionScheme {
    fn default() -> Self {
        Self {
            select: TransitionSchemeKind::TimeRanges,
            time_ranges: TimeRanges::default(),
            elevation_range: ElevationRange::default(),
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

//
// Tests
//

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_config_template() -> Result<()> {
        const CONFIG_TEMPLATE: &str =
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/config.toml"));
        toml::from_str::<Config>(CONFIG_TEMPLATE)?;
        Ok(())
    }

    // TODO: add conversion tests
}
