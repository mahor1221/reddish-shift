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
use std::{env, fs::File, io::Read, path::PathBuf, str::FromStr};
use strum::{VariantArray, VariantNames};

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
    pub provider: LocationProvider,
}

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

#[derive(Debug, Clone, Deserialize)]
pub struct TimeRanges {
    pub dawn: TimeRange,
    pub dusk: TimeRange,
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Elevation(f32);

/// The solar elevations at which the transition begins/ends,
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct ElevationRange {
    pub high: Elevation,
    pub low: Elevation,
}

#[derive(Debug, Clone, Copy, Deserialize, VariantNames, VariantArray)]
#[strum(serialize_all = "kebab-case")]
pub enum TransitionSchemeKind {
    Time,
    Elevation,
}

#[derive(Debug, Clone)]
pub struct TransitionScheme {
    pub default: TransitionSchemeKind,
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

#[derive(Debug, Clone, Copy, Deserialize, VariantNames, VariantArray)]
#[strum(serialize_all = "kebab-case")]
pub enum LocationProviderKind {
    Manual,
    Geoclue2,
}

#[derive(Debug, Clone)]
pub struct LocationProvider {
    pub default: LocationProviderKind,
    pub manual: Location,
}

#[derive(Debug, Clone, Copy, Deserialize, VariantNames, VariantArray)]
#[strum(serialize_all = "kebab-case")]
pub enum AdjustmentMethodKind {
    Randr,
    Drm,
    Vidmode,
}

#[derive(Debug, Clone)]
pub struct AdjustmentMethod {
    pub default: AdjustmentMethodKind,
    pub randr: u16,
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

#[derive(Debug, Clone)]
enum TransitionSchemeArgs {
    Time(Option<TimeRanges>),
    Elevation(Option<ElevationRange>),
}

#[derive(Debug, Clone)]
enum LocationProviderArgs {
    Manual(Option<Location>),
    Geoclue2,
}

#[derive(Debug, Clone, Default, Args)]
struct AdjustmentMethodArgs {
    #[arg(
        long = "method",
        short = 'm',
        value_name = "ADJUSTMENT_METHOD",
        help(formatcp!("[possible values: {ADJUSTMENT_METHOD_KINDS}]")),
        value_parser = AdjustmentMethodKind::from_str
    )]
    default: Option<AdjustmentMethodKind>,
    #[arg(long, value_name = "SCREEN_NUMBER")]
    randr_screen: Option<u16>,
}

#[derive(Debug, Clone, Default, Args)]
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

    #[command(flatten)]
    adjustment_method: AdjustmentMethodArgs,
}

#[derive(Debug, Clone, Args)]
struct DaemonOneShotArgs {
    #[arg(long, short, value_name = "TEMPERATURE_RANGE", value_parser = DayNight::<Temperature>::from_str)]
    temperature: Option<DayNight<Temperature>>,
    #[arg(long, short, value_name = "BRIGHTNESS_RANGE", value_parser = DayNight::<Brightness>::from_str)]
    brightness: Option<DayNight<Brightness>>,
    #[arg(long, short, value_name = "GAMMA_RANGE", value_parser = DayNight::<Gamma>::from_str)]
    gamma: Option<DayNight<Gamma>>,

    #[command(flatten)]
    c: SetResetArgs,

    #[arg(
        long = "scheme",
        short = 's',
        value_name = "TRANSITION_SCHEME | TIME_RANGE | ELEVATION",
        help(formatcp!("[possible values: {TRANSITION_SCHEME_KINDS}]")),
        value_parser = TransitionSchemeArgs::from_str
    )]
    transition_scheme: Option<TransitionSchemeArgs>,

    #[arg(
        long = "provider",
        short = 'l',
        value_name = "LOCATION_PROVIDER | LOCATION",
        help(formatcp!("[possible values: {LOCATION_PROVIDER_KINDS}]")),
        value_parser = LocationProviderArgs::from_str
    )]
    location_provider: Option<LocationProviderArgs>,
}

//
// Config file
//

#[derive(Debug, Clone, Default, Deserialize)]
struct Config {
    temperature: Option<DayNight<Temperature>>,
    brightness: Option<DayNight<Brightness>>,
    gamma: Option<DayNight<Gamma>>,
    preserve_gamma: Option<bool>,
    fade: Option<bool>,
    adjustment_method: AdjustmentMethodSection,
    transition_scheme: TransitionSchemeSection,
    location_provider: LocationProviderSection,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct TransitionSchemeSection {
    default: Option<TransitionSchemeKind>,
    #[serde(flatten)]
    time_ranges: Option<TimeRanges>,
    #[serde(flatten)]
    elevation_range: Option<ElevationRange>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct LocationProviderSection {
    default: Option<LocationProviderKind>,
    manual: Option<Location>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct AdjustmentMethodSection {
    default: Option<AdjustmentMethodKind>,
    randr_screen: Option<u16>,
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
            c,
            transition_scheme,
            location_provider,
        } = args;

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

        self.merge_with_set_reset_args(c);

        match transition_scheme {
            Some(TransitionSchemeArgs::Time(t)) => {
                self.scheme.default = TransitionSchemeKind::Time;
                if let Some(t) = t {
                    self.scheme.time_ranges = t;
                }
            }
            Some(TransitionSchemeArgs::Elevation(t)) => {
                self.scheme.default = TransitionSchemeKind::Elevation;
                if let Some(t) = t {
                    self.scheme.elevation_range = t;
                }
            }
            None => {}
        }

        match location_provider {
            Some(LocationProviderArgs::Manual(t)) => {
                self.provider.default = LocationProviderKind::Manual;
                if let Some(t) = t {
                    self.provider.manual = t;
                }
            }
            Some(LocationProviderArgs::Geoclue2) => {
                self.provider.default = LocationProviderKind::Geoclue2;
            }
            None => {}
        }
    }

    fn merge_with_set_reset_args(&mut self, args: SetResetArgs) {
        let SetResetArgs {
            preserve_gamma,
            fade,
            adjustment_method:
                AdjustmentMethodArgs {
                    default: m_default,
                    randr_screen,
                },
        } = args;

        if let Some(t) = preserve_gamma {
            self.preserve_gamma = t;
        }
        if let Some(t) = fade {
            self.fade = t;
        }

        if let Some(t) = m_default {
            self.method.default = t;
        }
        if let Some(t) = randr_screen {
            self.method.randr = t;
        }
    }

    fn merge_with_config(&mut self, config: Config) -> Result<()> {
        // TODO: move conversions to ConfigFile filds definition with serde derives
        let Config {
            temperature,
            brightness,
            gamma,

            preserve_gamma,
            fade,
            adjustment_method:
                AdjustmentMethodSection {
                    default: m_default,
                    randr_screen,
                },
            transition_scheme:
                TransitionSchemeSection {
                    default: t_default,
                    time_ranges,
                    elevation_range,
                },
            location_provider:
                LocationProviderSection {
                    default: p_default,
                    manual,
                },
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

        if let Some(t) = preserve_gamma {
            self.preserve_gamma = t;
        }
        if let Some(t) = fade {
            self.fade = t;
        }
        if let Some(t) = m_default {
            self.method.default = t;
        }
        if let Some(t) = randr_screen {
            self.method.randr = t;
        }

        if let Some(t) = t_default {
            self.scheme.default = t
        }
        if let Some(t) = time_ranges {
            self.scheme.time_ranges = t; // TODO: check if dawn is before dusk
        }
        if let Some(t) = elevation_range {
            self.scheme.elevation_range = t;
        }

        if let Some(t) = p_default {
            self.provider.default = t;
        }
        if let Some(t) = manual {
            self.provider.manual = t;
        }

        Ok(())
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
            transition_scheme:
                TransitionSchemeSection {
                    default: s_default,
                    time_ranges,
                    elevation_range,
                },
            location_provider:
                LocationProviderSection {
                    default: p_default,
                    manual,
                },
            adjustment_method:
                AdjustmentMethodSection {
                    default: m_default,
                    randr_screen,
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
        self.preserve_gamma = preserve_gamma;
        self.fade = fade;

        if let Some(t) = s_default {
            self.transition_scheme.default = Some(t);
        }
        if let Some(t) = time_ranges {
            self.transition_scheme.time_ranges = Some(t);
        }
        if let Some(t) = elevation_range {
            self.transition_scheme.elevation_range = Some(t);
        }

        if let Some(t) = p_default {
            self.location_provider.default = Some(t);
        }
        if let Some(t) = manual {
            self.location_provider.manual = Some(t);
        }

        if let Some(t) = m_default {
            self.adjustment_method.default = Some(t);
        }
        if let Some(t) = randr_screen {
            self.adjustment_method.randr_screen = Some(t);
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

impl FromStr for Temperature {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.trim().parse::<u16>()?.try_into()
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

impl FromStr for Brightness {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.trim().parse::<f32>()?.try_into()
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

impl FromStr for Gamma {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
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

impl FromStr for Hour {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.trim().parse::<u8>()?.try_into()
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

impl From<Time> for TimeOffset {
    fn from(Time { hour, minute }: Time) -> Self {
        Self((*hour.as_ref() as u32 * 60 * 60) + (*minute.as_ref() as u32 * 60))
    }
}

impl FromStr for TimeRange {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match *s.split("-").collect::<Vec<_>>().as_slice() {
            [start, end] => {
                let start: TimeOffset = start.parse::<Time>()?.into();
                let end: TimeOffset = end.parse::<Time>()?.into();

                if start <= end {
                    Ok(Self { start, end })
                } else {
                    Err(anyhow!("time_range"))
                }
            }
            [time] => {
                let t: TimeOffset = time.parse::<Time>()?.into();
                Ok(Self { start: t, end: t })
            }
            _ => Err(anyhow!("time_range")),
        }
    }
}

impl FromStr for TimeRanges {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let time_ranges = match *s.split("-").collect::<Vec<_>>().as_slice() {
            [dawn, dusk] => {
                let dawn: TimeRange = dawn.parse()?;
                let dusk: TimeRange = dusk.parse()?;
                Ok(Self { dawn, dusk })
            }
            [dawn_start, dawn_end, "", dusk_start, dusk_end] => {
                let dawn: TimeRange = [dawn_start, dawn_end].concat().parse()?;
                let dusk: TimeRange = [dusk_start, dusk_end].concat().parse()?;
                Ok(Self { dawn, dusk })
            }
            _ => Err(anyhow!("time_ranges")),
        }?;

        if time_ranges.dawn.end < time_ranges.dusk.start {
            Ok(time_ranges)
        } else {
            Err(anyhow!("time_ranges"))
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

impl FromStr for Elevation {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.trim().parse::<f32>()?.try_into()
    }
}

impl FromStr for ElevationRange {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match *s.split(":").collect::<Vec<_>>().as_slice() {
            [high, low] => {
                let high = Elevation::try_from(high.parse::<f32>()?)?;
                let low = Elevation::try_from(low.parse::<f32>()?)?;

                if high >= low {
                    Ok(Self { high, low })
                } else {
                    // b"High transition elevation cannot be lower than the low transition elevation.\n\0"
                    Err(anyhow!("elevation"))
                }
            }
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

impl FromStr for TransitionSchemeKind {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        <Self as VariantNames>::VARIANTS
            .into_iter()
            .enumerate()
            .find(|(_, v)| **v == s.to_lowercase())
            .map(|(i, _)| VariantArray::VARIANTS[i])
            .ok_or(anyhow!("scheme"))
    }
}

impl FromStr for LocationProviderKind {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        <Self as VariantNames>::VARIANTS
            .into_iter()
            .enumerate()
            .find(|(_, v)| **v == s.to_lowercase())
            .map(|(i, _)| VariantArray::VARIANTS[i])
            .ok_or(anyhow!("provider"))
    }
}

impl FromStr for AdjustmentMethodKind {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        <Self as VariantNames>::VARIANTS
            .into_iter()
            .enumerate()
            .find(|(_, v)| **v == s.to_lowercase())
            .map(|(i, _)| VariantArray::VARIANTS[i])
            .ok_or(anyhow!("method"))
    }
}

impl FromStr for TransitionSchemeArgs {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<TransitionSchemeKind>()
            .map(|kind| match kind {
                TransitionSchemeKind::Time => Self::Time(None),
                TransitionSchemeKind::Elevation => Self::Elevation(None),
            })
            .or_else(|_| {
                let t = s.parse::<TimeRanges>()?;
                Ok(Self::Time(Some(t)))
            })
            .or_else(|_: anyhow::Error| {
                let t = s.parse::<ElevationRange>()?;
                Ok(Self::Elevation(Some(t)))
            })
            .map_err(|_: anyhow::Error| anyhow!("asdf"))
    }
}

impl FromStr for LocationProviderArgs {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<LocationProviderKind>()
            .map(|kind| match kind {
                LocationProviderKind::Manual => Self::Manual(None),
                LocationProviderKind::Geoclue2 => Self::Geoclue2,
            })
            .or_else(|_| {
                let t = s.parse::<Location>()?;
                Ok(Self::Manual(Some(t)))
            })
            .map_err(|_: anyhow::Error| anyhow!("asdf"))
    }
}

// impl TryFrom<TimeRangesSection> for TimeRanges {
//     type Error = anyhow::Error;

//     fn try_from(t: TimeRangesSection) -> Result<Self, Self::Error> {
//         match t {
//             TimeRangesArgs {
//                 dawn: Some(dawn),
//                 dusk: Some(dusk),
//             } => Ok(Self { dawn, dusk }),
//             _ => Err(anyhow!("time_ranges")),
//         }
//     }
// }

// impl TryFrom<ElevationRangeSection> for ElevationRange {
//     type Error = anyhow::Error;

//     fn try_from(t: ElevationRangeSection) -> Result<Self, Self::Error> {
//         match t {
//             ElevationRangeArgs {
//                 high: Some(high),
//                 low: Some(low),
//             } => (high, low).try_into(),
//             _ => Err(anyhow!("elevation")),
//         }
//     }
// }

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
    T: Clone + FromStr<Err = anyhow::Error>,
{
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        String::deserialize(d)?
            .parse()
            .map_err(|e| DeError::custom(e))
    }
}

impl<'de> Deserialize<'de> for Temperature {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        u16::deserialize(d)?
            .try_into()
            .map_err(|e| DeError::custom(e))
    }
}

impl<'de> Deserialize<'de> for Brightness {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        f32::deserialize(d)?
            .try_into()
            .map_err(|e| DeError::custom(e))
    }
}

impl<'de> Deserialize<'de> for Gamma {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        f32::deserialize(d)?
            .try_into()
            .map_err(|e| DeError::custom(e))
    }
}

impl<'de> Deserialize<'de> for TimeRange {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        String::deserialize(d)?
            .parse()
            .map_err(|e| DeError::custom(e))
    }
}

impl<'de> Deserialize<'de> for Elevation {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        f32::deserialize(d)?
            .try_into()
            .map_err(|e| DeError::custom(e))
    }
}

impl<'de> Deserialize<'de> for Latitude {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        f32::deserialize(d)?
            .try_into()
            .map_err(|e| DeError::custom(e))
    }
}

impl<'de> Deserialize<'de> for Longitude {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        f32::deserialize(d)?
            .try_into()
            .map_err(|e| DeError::custom(e))
    }
}

impl<'de> Deserialize<'de> for Location {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        String::deserialize(d)?
            .parse()
            .map_err(|e| DeError::custom(e))
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
        Self {
            default: TransitionSchemeKind::Time,
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
            default: LocationProviderKind::Manual,
            manual: Location::default(),
        }
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

    // TODO: assert_eq default config with config.toml

    // TODO: add conversion tests

    // TODO: test help for possible values of enums
}
