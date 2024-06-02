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
use std::{
    env, fmt::Display, fs::File, io::Read, marker::PhantomData, path::PathBuf,
    str::FromStr,
};

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

const TRANSITION_SCHEME_KINDS: &str = "time, elevation";
const ADJUSTMENT_METHOD_KINDS: &str = "dummy, drm, randr";
const LOCATION_PROVIDER_KINDS: &str = "manual, geoclue2";

const PKG_NAME: &str = env!("CARGO_PKG_NAME");

const VERSION: &str = {
    const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
    const GIT_DESCRIBE: &str = env!("VERGEN_GIT_DESCRIBE");
    const GIT_COMMIT_DATE: &str = env!("VERGEN_GIT_COMMIT_DATE");

    formatcp!("{PKG_NAME} {PKG_VERSION} ({GIT_DESCRIBE} {GIT_COMMIT_DATE})")
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
        "Usage: {PKG_NAME} -l LAT:LON -t DAY:NIGHT [OPTIONS...]

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
// Parsed types
//

/// Merge of cli arguments and config files
#[derive(Debug, Clone)]
pub struct Options {
    pub verbosity: Verbosity,
    pub dry_run: bool,
    pub mode: Mode,

    pub day: ColorSetting,
    pub night: ColorSetting,
    pub preserve_gamma: bool,
    pub fade: bool,
    pub method: AdjustmentMethod,
    pub scheme: TransitionScheme,
    pub location: LocationProvider,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Temperature(u16);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Brightness(f32);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gamma([f32; 3]);

#[derive(Debug, Clone)]
pub struct DayNight<T> {
    day: T,
    night: T,
}

pub type TemperatureRange = DayNight<Temperature>;
pub type BrightnessRange = DayNight<Brightness>;
pub type GammaRange = DayNight<Gamma>;

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
pub struct Offset(u32);

#[derive(Debug, Clone, Copy)]
pub struct OffsetRange {
    pub start: Offset,
    pub end: Offset,
}

#[derive(Debug, Clone)]
pub struct TimeRange {
    pub dawn: OffsetRange,
    pub dusk: OffsetRange,
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Elevation(f32);

/// The solar elevations at which the transition begins/ends,
#[derive(Debug, Clone, Copy)]
pub struct ElevationRange {
    pub high: Elevation,
    pub low: Elevation,
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

#[derive(Debug, Clone)]
pub enum TransitionScheme {
    Time(TimeRange),
    Elevation(ElevationRange),
}

#[derive(Debug, Clone)]
pub enum LocationProvider {
    Manual(Location),
    Geoclue2,
}

#[derive(Debug, Clone)]
pub enum AdjustmentMethod {
    Randr(u16),
    Drm,
    Vidmode,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum Mode {
    #[default]
    Daemon,
    Oneshot,
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

//
// Config file
//

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Config {
    temperature: Option<Either<u16, TemperatureRange>>,
    brightness: Option<Either<f32, BrightnessRange>>,
    gamma: Option<Either<f32, GammaRange>>,
    preserve_gamma: Option<bool>,
    fade: Option<bool>,
    scheme: Option<TransitionScheme>,
    location: Option<LocationProvider>,
    method: Option<AdjustmentMethod>,
}

#[derive(Debug, Clone, Default)]
struct Either<U: TryInto<T>, T> {
    t: T,
    p: PhantomData<U>,
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
    #[arg(long, short, global = true, display_order(100), value_name = "FILE")]
    config: Option<PathBuf>,
    #[arg(long, short, global = true, display_order(100))]
    dry_run: bool,
    #[command(flatten)]
    verbosity: VerbosityArgs,
}

#[derive(Args)]
#[group(multiple = false)]
struct VerbosityArgs {
    #[arg(long, short, global = true, display_order(100))]
    quite: bool,
    #[arg(long, short, global = true, display_order(100))]
    verbose: bool,
}

#[derive(Args)]
#[group(required = true, multiple = true)]
struct ColorSettingArgs {
    #[arg(long, short, value_parser = Temperature::from_str)]
    temperature: Option<Temperature>,
    #[arg(long, short, value_parser = Gamma::from_str)]
    gamma: Option<Gamma>,
    #[arg(long, short, value_parser = Brightness::from_str)]
    brightness: Option<Brightness>,
}

#[derive(Subcommand)]
enum ModeArgs {
    Daemon(DaemonOneShotArgs),
    OneShot(DaemonOneShotArgs),
    Set {
        #[command(flatten)]
        cs: ColorSettingArgs,
        #[command(flatten)]
        sa: SetResetArgs,
    },
    Reset(SetResetArgs),
}

#[derive(Debug, Clone, Args)]
struct SetResetArgs {
    #[arg(
        long,
        value_name = "BOOLEAN",
        hide_possible_values(true),
        display_order(99)
    )]
    preserve_gamma: Option<bool>,
    #[arg(
        long,
        value_name = "BOOLEAN",
        hide_possible_values(true),
        display_order(99)
    )]
    fade: Option<bool>,

    #[arg(
        long,
        short,
        value_name = "ADJUSTMENT_METHOD[:<SCREEN_NUMBER>]",
        help(formatcp!("[{ADJUSTMENT_METHOD_KINDS}]")),
        value_parser = AdjustmentMethod::from_str
    )]
    method: Option<AdjustmentMethod>,
}

#[derive(Debug, Clone, Args)]
struct DaemonOneShotArgs {
    #[arg(long, short, value_name = "TEMPERATURE_RANGE", value_parser = TemperatureRange::from_str)]
    temperature: Option<TemperatureRange>,
    #[arg(long, short, value_name = "BRIGHTNESS_RANGE", value_parser = BrightnessRange::from_str)]
    brightness: Option<BrightnessRange>,
    #[arg(long, short, value_name = "GAMMA_RANGE", value_parser = GammaRange::from_str)]
    gamma: Option<GammaRange>,

    #[command(flatten)]
    set_reset_args: SetResetArgs,

    #[arg(
        long,
        short,
        value_name = "TIME | ELEVATION",
        help(formatcp!("[{TRANSITION_SCHEME_KINDS}]")),
        value_parser = TransitionScheme::from_str
    )]
    scheme: Option<TransitionScheme>,

    #[arg(
        long,
        short,
        value_name = "LOCATION_PROVIDER | LOCATION",
        help(formatcp!("[{LOCATION_PROVIDER_KINDS}]")),
        value_parser = LocationProvider::from_str
    )]
    location: Option<LocationProvider>,
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
                self.merge_with_daemon_oneshot_args(c);
                self.mode = Mode::Daemon;
            }
            Some(ModeArgs::OneShot(c)) => {
                self.merge_with_daemon_oneshot_args(c);
                self.mode = Mode::Oneshot;
            }
            Some(ModeArgs::Set { cs, sa: ca }) => {
                self.merge_with_set_reset_args(ca);
                self.day = cs.into();
                self.mode = Mode::Set;
            }
            Some(ModeArgs::Reset(ca)) => {
                self.merge_with_set_reset_args(ca);
                self.mode = Mode::Reset;
            }
            None => {}
        }

        Ok(())
    }

    fn merge_with_daemon_oneshot_args(&mut self, args: DaemonOneShotArgs) {
        let DaemonOneShotArgs {
            temperature,
            brightness,
            gamma,
            set_reset_args,
            scheme,
            location,
        } = args;

        if let Some(t) = temperature {
            self.day.temperature = t.day;
            self.night.temperature = t.night;
        }
        if let Some(t) = brightness {
            self.day.brightness = t.day;
            self.night.brightness = t.night;
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
        self.merge_with_set_reset_args(set_reset_args);
    }

    fn merge_with_set_reset_args(&mut self, args: SetResetArgs) {
        let SetResetArgs {
            preserve_gamma,
            fade,
            method,
        } = args;

        if let Some(t) = preserve_gamma {
            self.preserve_gamma = t;
        }
        if let Some(t) = fade {
            self.fade = t;
        }
        if let Some(t) = method {
            self.method = t;
        }
    }

    fn merge_with_config(&mut self, config: Config) {
        // TODO: move conversions to ConfigFile filds definition with serde derives
        let Config {
            temperature,
            brightness,
            gamma,
            preserve_gamma,
            fade,
            method,
            scheme,
            location,
        } = config;

        if let Some(t) = temperature {
            self.day.temperature = t.t.day;
            self.night.temperature = t.t.night;
        }
        if let Some(t) = brightness {
            self.day.brightness = t.t.day;
            self.night.brightness = t.t.night;
        }
        if let Some(t) = gamma {
            self.day.gamma = t.t.day;
            self.night.gamma = t.t.night;
        }

        if let Some(t) = preserve_gamma {
            self.preserve_gamma = t;
        }
        if let Some(t) = fade {
            self.fade = t;
        }

        if let Some(t) = scheme {
            self.scheme = t;
        }
        if let Some(t) = location {
            self.location = t;
        }
        if let Some(t) = method {
            self.method = t;
        }
    }
}

impl Config {
    fn new(config_path: Option<PathBuf>) -> Result<Self> {
        #[cfg(unix)]
        let system_config =
            PathBuf::from(formatcp!("/etc/{PKG_NAME}/config.toml"));
        let user_config = config_path
            .or_else(|| {
                dirs::config_dir().map(|d| d.join(PKG_NAME).join("config.toml"))
            })
            .ok_or(anyhow!("user_config"))?;

        let mut buf = String::new();
        let mut read = |path| -> Result<Self> {
            File::open(path)?.read_to_string(&mut buf)?;
            Ok(toml::from_str(&buf)?)
        };

        let mut config = Self::default();
        #[cfg(unix)]
        config.merge(read(system_config)?);
        config.merge(read(user_config)?);
        Ok(config)
    }

    fn merge(&mut self, other: Self) {
        let Self {
            temperature,
            brightness,
            gamma,
            preserve_gamma,
            fade,
            method,
            scheme,
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
        self.preserve_gamma = preserve_gamma;
        self.fade = fade;

        if let Some(t) = scheme {
            self.scheme = Some(t);
        }
        if let Some(t) = location {
            self.location = Some(t);
        }
        if let Some(t) = method {
            self.method = Some(t);
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
            method: Default::default(),
            scheme: Default::default(),
            location: Default::default(),
        }
    }
}

//
// Parse strings and numbers to strong types
//

fn gamma(n: f32) -> Result<f32> {
    if n >= MIN_GAMMA && n <= MAX_GAMMA {
        Ok(n)
    } else {
        // b"Gamma value must be between %.1f and %.1f.\n\0" as *const u8 as *const c_char,
        Err(anyhow!("gamma"))
    }
}

fn time_range(time: TimeRange) -> Result<TimeRange> {
    if time.dawn.end < time.dusk.start {
        Ok(time)
    } else {
        Err(anyhow!("time_range"))
    }
}

fn elevation_range(elev: ElevationRange) -> Result<ElevationRange> {
    if elev.high >= elev.low {
        Ok(elev)
    } else {
        // b"High transition elevation cannot be lower than the low transition elevation.\n\0"
        Err(anyhow!("elevation"))
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

impl TryFrom<f32> for Gamma {
    type Error = anyhow::Error;

    fn try_from(n: f32) -> Result<Self, Self::Error> {
        Ok(Self([gamma(n)?; 3]))
    }
}

impl TryFrom<[f32; 3]> for Gamma {
    type Error = anyhow::Error;

    fn try_from([r, g, b]: [f32; 3]) -> Result<Self, Self::Error> {
        Ok(Self([gamma(r)?, gamma(g)?, gamma(b)?]))
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

impl From<Time> for Offset {
    fn from(Time { hour, minute }: Time) -> Self {
        Self((*hour.as_ref() as u32 * 60 * 60) + (*minute.as_ref() as u32 * 60))
    }
}

impl TryFrom<(Offset, Offset)> for OffsetRange {
    type Error = anyhow::Error;

    fn try_from((start, end): (Offset, Offset)) -> Result<Self, Self::Error> {
        if start <= end {
            Ok(Self { start, end })
        } else {
            Err(anyhow!("offset_range"))
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

impl TryFrom<u16> for TemperatureRange {
    type Error = anyhow::Error;

    fn try_from(n: u16) -> Result<Self, Self::Error> {
        let t = Temperature::try_from(n)?;
        Ok(Self { day: t, night: t })
    }
}

impl TryFrom<f32> for BrightnessRange {
    type Error = anyhow::Error;

    fn try_from(n: f32) -> Result<Self, Self::Error> {
        let t = Brightness::try_from(n)?;
        Ok(Self { day: t, night: t })
    }
}

impl TryFrom<f32> for GammaRange {
    type Error = anyhow::Error;

    fn try_from(n: f32) -> Result<Self, Self::Error> {
        let t = Gamma::try_from(n)?;
        Ok(Self { day: t, night: t })
    }
}

//

impl FromStr for Temperature {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.trim().parse::<u16>()?.try_into()
    }
}

impl FromStr for Brightness {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.trim().parse::<f32>()?.try_into()
    }
}

impl FromStr for Gamma {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match *s.split(":").map(str::trim).collect::<Vec<_>>().as_slice() {
            [r, g, b] => {
                [r.parse::<f32>()?, g.parse::<f32>()?, b.parse::<f32>()?]
                    .try_into()
            }
            [rbg] => rbg.parse::<f32>()?.try_into(),
            _ => Err(anyhow!("gamma")),
        }
    }
}

impl FromStr for Latitude {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.trim().parse::<f32>()?.try_into()
    }
}

impl FromStr for Longitude {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.trim().parse::<f32>()?.try_into()
    }
}

impl FromStr for Location {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match *s.split(":").collect::<Vec<_>>().as_slice() {
            [lat, lon] => Ok(Self {
                latitude: lat.parse()?,
                longitude: lon.parse()?,
            }),
            _ => Err(anyhow!("location")),
        }
    }
}

impl FromStr for Hour {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.trim().parse::<u8>()?.try_into()
    }
}

impl FromStr for Minute {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.trim().parse::<u8>()?.try_into()
    }
}

impl FromStr for Time {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match *s.split(":").collect::<Vec<_>>().as_slice() {
            [hour, minute] => Ok(Self {
                hour: hour.parse()?,
                minute: minute.parse()?,
            }),
            _ => Err(anyhow!("time")),
        }
    }
}

impl FromStr for Offset {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.parse::<Time>()?.into())
    }
}

impl FromStr for Elevation {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.trim().parse::<f32>()?.try_into()
    }
}

impl FromStr for OffsetRange {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match *s.split("-").collect::<Vec<_>>().as_slice() {
            [start, end] => {
                (start.parse::<Offset>()?, end.parse::<Offset>()?).try_into()
            }
            [time] => {
                let t = time.parse::<Offset>()?;
                Ok(Self { start: t, end: t })
            }
            _ => Err(anyhow!("time_range")),
        }
    }
}

impl FromStr for TimeRange {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match *s.split("-").collect::<Vec<_>>().as_slice() {
            [dawn, dusk] => {
                let dawn: OffsetRange = dawn.parse()?;
                let dusk: OffsetRange = dusk.parse()?;
                time_range(Self { dawn, dusk })
            }
            [dawn_start, dawn_end, dusk_start, dusk_end] => {
                let dawn: OffsetRange =
                    [dawn_start, dawn_end].concat().parse()?;
                let dusk: OffsetRange =
                    [dusk_start, dusk_end].concat().parse()?;
                time_range(Self { dawn, dusk })
            }
            _ => Err(anyhow!("time_range")),
        }
    }
}

impl FromStr for ElevationRange {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match *s.split(":").collect::<Vec<_>>().as_slice() {
            [high, low] => {
                let high: Elevation = high.parse()?;
                let low: Elevation = low.parse()?;
                elevation_range(Self { high, low })
            }
            _ => Err(anyhow!("elevation")),
        }
    }
}

impl FromStr for TransitionScheme {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Err(())
            .or_else(|_| {
                Ok::<_, Self::Err>(Self::Time(TimeRange::from_str(s)?))
            })
            .or_else(|_| {
                Ok::<_, Self::Err>(Self::Elevation(ElevationRange::from_str(
                    s,
                )?))
            })
            .map_err(|_| anyhow!("asdf"))
    }
}

impl FromStr for LocationProvider {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // TODO: map cities or countries to locations
        match s {
            "geoclue2" => Ok(Self::Geoclue2),
            _ => s.parse().map(Self::Manual),
        }
        .map_err(|_| anyhow!("asdf"))
    }
}

impl FromStr for AdjustmentMethod {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.split(":").map(str::trim).collect::<Vec<_>>().as_slice() {
            ["randr"] => Ok(Self::Randr(Default::default())),
            ["randr", n] => {
                n.parse().map(Self::Randr).map_err(|_| anyhow!("asdf"))
            }
            ["drm"] => Ok(Self::Drm),
            ["vidmode"] => Ok(Self::Vidmode),
            _ => Err(anyhow!("method")),
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

impl<T: Clone + FromStr<Err = anyhow::Error>> FromStr for DayNight<T> {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match *s.split("-").collect::<Vec<_>>().as_slice() {
            [day_night] => {
                let day_night = day_night.parse::<T>()?;
                Ok(Self {
                    day: day_night.clone(),
                    night: day_night,
                })
            }
            [day, night] => Ok(Self {
                day: day.parse()?,
                night: night.parse()?,
            }),
            _ => Err(anyhow!("day_night")),
        }
    }
}

//
// boilerplates
//

#[derive(Deserialize)]
#[serde(untagged)]
enum EitherInner<U, T> {
    A(T),
    B(U),
}

impl<'de, T, U> Deserialize<'de> for Either<U, T>
where
    EitherInner<U, T>: Deserialize<'de>,
    U: TryInto<T>,
    U::Error: Display,
{
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let t = match EitherInner::<U, T>::deserialize(d)? {
            EitherInner::A(t) => t,
            EitherInner::B(u) => u.try_into().map_err(DeError::custom)?,
        };
        Ok(Self { t, p: PhantomData })
    }
}

impl<'de, T> Deserialize<'de> for DayNight<T>
where
    T: Clone + FromStr<Err = anyhow::Error>,
{
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        String::deserialize(d)?.parse().map_err(DeError::custom)
    }
}

impl<'de> Deserialize<'de> for TransitionScheme {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        String::deserialize(d)?.parse().map_err(DeError::custom)
    }
}

impl<'de> Deserialize<'de> for LocationProvider {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        String::deserialize(d)?.parse().map_err(DeError::custom)
    }
}

impl<'de> Deserialize<'de> for AdjustmentMethod {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        String::deserialize(d)?.parse().map_err(DeError::custom)
    }
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

impl AsRef<u32> for Offset {
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

//

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

impl Default for TimeRange {
    fn default() -> Self {
        Self {
            dawn: DEFAULT_TIME_RANGE_DAWN
                .parse()
                .unwrap_or_else(|_| unreachable!()),
            dusk: DEFAULT_TIME_RANGE_DUSK
                .parse()
                .unwrap_or_else(|_| unreachable!()),
        }
    }
}

impl Default for TransitionScheme {
    fn default() -> Self {
        Self::Elevation(Default::default())
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
        Self::Manual(Default::default())
    }
}

impl Default for AdjustmentMethod {
    fn default() -> Self {
        // TODO: change default depending on OS
        Self::Randr(Default::default())
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
        let cfg = toml::from_str::<Config>(CONFIG_TEMPLATE)?;

        dbg!(cfg.location);
        panic!()
        // Ok(())
    }

    // TODO: assert_eq default config with config.toml

    // TODO: add conversion tests

    // TODO: test help for possible values of enums
}
