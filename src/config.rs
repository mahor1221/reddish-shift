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
    gamma_drm::Drm,
    gamma_dummy::Dummy,
    gamma_randr::Randr,
    gamma_vidmode::Vidmode,
    location_manual::Manual,
    solar::{solar_elevation, SOLAR_CIVIL_TWILIGHT_ELEV},
    Alpha,
};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Local, NaiveTime, Timelike};
use clap::{Args, Parser, Subcommand};
use const_format::formatcp;
use serde::{de, Deserialize, Deserializer};
use std::{
    cmp::Ordering,
    env,
    fmt::Display,
    fs::File,
    io::{Read, Result as IoResult, Write},
    marker::PhantomData,
    ops::Deref,
    path::{Path, PathBuf},
    str::FromStr,
    time::Duration,
};
use toml::Value;

// Duration of sleep between screen updates (milliseconds)
const SLEEP_DURATION: Duration = Duration::from_millis(5000);
const FADE_SLEEP_DURATION: Duration = Duration::from_millis(100);
// Length of fade in numbers of fade's sleep durations
pub const FADE_STEPS: u8 = 40;

// Angular elevation of the sun at which the color temperature
// transition period starts and ends (in degrees).
// Transition during twilight, and while the sun is lower than
// 3.0 degrees above the horizon.
const DEFAULT_ELEVATION_LOW: f64 = SOLAR_CIVIL_TWILIGHT_ELEV;
const DEFAULT_ELEVATION_HIGH: f64 = 3.0;
const DEFAULT_LATITUDE: f64 = 0.0; // Null Island
const DEFAULT_LONGITUDE: f64 = 0.0;
const DEFAULT_TEMPERATURE: u16 = 6500;
const DEFAULT_TEMPERATURE_DAY: u16 = 6500;
const DEFAULT_TEMPERATURE_NIGHT: u16 = 4500;
const DEFAULT_BRIGHTNESS: f64 = 1.0;
const DEFAULT_GAMMA: f64 = 1.0;

const MIN_TEMPERATURE: u16 = 1000;
const MAX_TEMPERATURE: u16 = 25000;
const MIN_BRIGHTNESS: f64 = 0.1;
const MAX_BRIGHTNESS: f64 = 1.0;
const MIN_GAMMA: f64 = 0.1;
const MAX_GAMMA: f64 = 10.0;
const MIN_LATITUDE: f64 = -90.0;
const MAX_LATITUDE: f64 = 90.0;
const MIN_LONGITUDE: f64 = -180.0;
const MAX_LONGITUDE: f64 = 180.0;
const MIN_ELEVATION: f64 = -90.0;
const MAX_ELEVATION: f64 = 90.0;

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

const PKG_BUGREPORT: &str =
    "https://github.com/mahor1221/reddish-shift/issues";

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

// fn print_method_list() {
//     println!("Available adjustment methods:");

//     // let mut i: c_int = 0 as c_int;
//     // while !((*gamma_methods.offset(i as isize)).name).is_null() {
//     //     let name = (*gamma_methods.offset(i as isize)).name;
//     //     let name = CStr::from_ptr(name).to_str().unwrap();
//     //     println!("  {name}");
//     //     i += 1;
//     // }

//     // TRANSLATORS: `help' must not be translated.
//     println!(
//         "
// Specify colon-separated options with `-m METHOD:OPTIONS`
// Try `-m METHOD:help' for help."
//     );
// }

// fn print_provider_list() {
//     println!("Available location providers:");

//     // let mut i: c_int = 0 as c_int;
//     // while !((*location_providers.offset(i as isize)).name).is_null() {
//     //     let name = (*location_providers.offset(i as isize)).name;
//     //     let name = CStr::from_ptr(name).to_str().unwrap();
//     //     println!("  {name}");
//     //     i += 1;
//     // }

//     // TRANSLATORS: `help' must not be translated.
//     println!(
//         "
// Specify colon-separated options with`-l PROVIDER:OPTIONS'.
// Try `-l PROVIDER:help' for help.
// "
//     );
// }

// fn print_manual_help() {
//     // TRANSLATORS: Manual location help output
//     // left column must not be translated
//     println!(
//         "Specify location manually.

//   lat=N\t\tLatitude
//   lon=N\t\tLongitude

//   Both values are expected to be floating point numbers,
//   negative values representing west / south, respectively."
//     );
// }

// // "Parameter `{}` is now always on;  Use the `-P` command-line option to disable.",

// fn print_dummy_help() {
//     println!("Does not affect the display but prints the color temperature to the terminal.")
// }

// fn start_dummy() {
//     eprintln!("WARNING: Using dummy gamma method! Display will not be affected by this gamma method.");
// }

// fn print_vidmode_help() {
//     // b"Adjust gamma ramps with the X VidMode extension.\n\0" as *const u8
//     // b"  screen=N\t\tX screen to apply adjustments to\n\0" as *const u8
// }

// fn print_randr_help() {
//     // fputs(_("Adjust gamma ramps with the X RANDR extension.\n"), f);
//     // fputs("\n", f);

//     // /* TRANSLATORS: RANDR help output
//     //    left column must not be translated */
//     // fputs(_("  screen=N\t\tX screen to apply adjustments to\n"
//     //         "  crtc=N\tList of comma separated CRTCs to apply"
//     //         " adjustments to\n"),
//     //       f);
//     // fputs("\n", f);
// }

// fn print_drm_help() {
//     // requires root
//     // b"Adjust gamma ramps with Direct Rendering Manager.\n\0" as *const u8
// }

/// Merge of cli arguments and config files
#[derive(Debug)]
pub struct Config {
    pub mode: Mode,

    pub day: ColorSettings,
    pub night: ColorSettings,
    pub reset_ramps: bool,
    pub scheme: TransitionScheme,
    pub disable_fade: bool,
    pub fade_sleep_duration: Duration,
    pub sleep_duration: Duration,

    pub location: LocationProvider,
    pub method: AdjustmentMethod,
    pub time: fn() -> DateTime<Local>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConfigBuilder {
    verbosity: VerbosityKind,
    mode: Mode,

    day: ColorSettings,
    night: ColorSettings,
    reset_ramps: bool,
    disable_fade: bool,
    scheme: TransitionScheme,
    sleep_duration: Duration,
    fade_sleep_duration: Duration,

    location: LocationProviderType,
    method: Option<AdjustmentMethodType>,
    time: fn() -> DateTime<Local>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Temperature(u16);

#[derive(Debug, Clone, Copy)]
pub struct Brightness(f64);

#[derive(Debug, Clone, Copy)]
pub struct Gamma([f64; 3]);

#[derive(Debug, Clone)]
pub struct DayNight<T> {
    day: T,
    night: T,
}

pub type TemperatureRange = DayNight<Temperature>;
pub type BrightnessRange = DayNight<Brightness>;
pub type GammaRange = DayNight<Gamma>;

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
    // Geoclue2(Geoclue2),
}

#[derive(Debug, Clone, PartialEq)]
enum LocationProviderType {
    Manual(Manual),
    // Geoclue2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AdjustmentMethodType {
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum VerbosityKind {
    Quite,
    #[default]
    Low,
    High,
}

#[derive(Debug, Clone)]
pub enum Verbosity<W: Write> {
    Quite,
    Low(W),
    High(W),
}

//
// Config file
//

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct ConfigFile {
    temperature: Option<Either<u16, TemperatureRange>>,
    brightness: Option<Either<f64, BrightnessRange>>,
    gamma: Option<Either<f64, GammaRange>>,
    reset_ramps: Option<bool>,
    scheme: Option<TransitionScheme>,
    disable_fade: Option<bool>,
    fade_sleep_duration: Option<u16>,
    sleep_duration: Option<u16>,
    location: Option<LocationProviderType>,
    method: Option<AdjustmentMethodType>,
}

#[derive(Debug, Clone, Default)]
struct Either<U: TryInto<T>, T> {
    t: T,
    p: PhantomData<U>,
}

//
// CLI Arguments
//

#[derive(Debug, Parser)]
#[command(version, about)]
#[command(propagate_version = true)]
struct CliArgs {
    #[command(subcommand)]
    mode: ModeArgs,
}

#[derive(Debug, Subcommand)]
enum ModeArgs {
    Daemon {
        #[command(flatten)]
        c: CmdArgs,
        #[arg(long, short = 'r')] // redshift uses -r for disabling fade
        disable_fade: bool,
        #[arg(long, value_name = "MILLISECONDS")]
        fade_sleep_duration: Option<u16>,
        #[arg(long, value_name = "MILLISECONDS")]
        sleep_duration: Option<u16>,
    },

    Oneshot {
        #[command(flatten)]
        c: CmdArgs,
    },

    Set {
        #[command(flatten)]
        cs: ColorSettingsArgs,
        #[command(flatten)]
        i: CmdInnerArgs,
    },

    Reset {
        #[command(flatten)]
        i: CmdInnerArgs,
    },

    Print {
        #[arg(
        long,
        short,
        value_name = "LOCATION_PROVIDER | LOCATION",
        value_parser = LocationProviderType::from_str,
    )]
        location: LocationProviderType,
    },
}

#[derive(Debug, Args)]
#[group(required = true, multiple = true)]
struct ColorSettingsArgs {
    #[arg(long, short, value_parser = Temperature::from_str)]
    temperature: Option<Temperature>,
    #[arg(long, short, value_parser = Gamma::from_str)]
    gamma: Option<Gamma>,
    #[arg(long, short, value_parser = Brightness::from_str)]
    brightness: Option<Brightness>,
}

#[derive(Debug, Args)]
struct CmdInnerArgs {
    #[arg(
        long,
        short,
        value_name = "ADJUSTMENT_METHOD[:DISPLAY[:CRTC1,CRTC2,...]]",
        value_parser = AdjustmentMethodType::from_str
    )]
    method: Option<AdjustmentMethodType>,

    #[arg(long)]
    reset_ramps: bool,

    #[arg(long, short, display_order(100), value_name = "FILE")]
    config: Option<PathBuf>,
    #[command(flatten)]
    verbosity: VerbosityArgs,
}

#[derive(Debug, Args)]
#[group(multiple = false)]
struct VerbosityArgs {
    #[arg(long, short, display_order(100))]
    quite: bool,
    #[arg(long, short, display_order(100))]
    verbose: bool,
}

#[derive(Debug, Args)]
struct CmdArgs {
    #[arg(long, short, value_name = "TEMPERATURE_RANGE", value_parser = TemperatureRange::from_str)]
    temperature: Option<TemperatureRange>,
    #[arg(long, short, value_name = "BRIGHTNESS_RANGE", value_parser = BrightnessRange::from_str)]
    brightness: Option<BrightnessRange>,
    #[arg(long, short, value_name = "GAMMA_RANGE", value_parser = GammaRange::from_str)]
    gamma: Option<GammaRange>,

    #[arg(
        long,
        short,
        value_name = "TIME | ELEVATION",
        value_parser = TransitionScheme::from_str
    )]
    scheme: Option<TransitionScheme>,

    #[arg(
        long,
        short,
        value_name = "LOCATION_PROVIDER | LOCATION",
        value_parser = LocationProviderType::from_str,
    )]
    location: Option<LocationProviderType>,

    #[command(flatten)]
    i: CmdInnerArgs,
}

//
// Merge from highest priority to lowest:
// 1. cli arguments
// 2. user config file
// 3. system config file
// 4. default options
//

impl ConfigBuilder {
    pub fn new() -> Result<Self> {
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
    pub fn build<W: Write>(self, w: W) -> Result<(Config, Verbosity<W>)> {
        let Self {
            verbosity,
            mode,
            day,
            night,
            reset_ramps,
            disable_fade,
            scheme,
            sleep_duration,
            fade_sleep_duration,
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
            _ => method.ok_or(anyhow!("TODO"))?,
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
            LocationProviderType::Manual(m) => LocationProvider::Manual(m),
            // LocationProviderType::Geoclue2 => {
            //     LocationProvider::Geoclue2(Default::default())
            // }
        };

        let c = Config {
            mode,
            day,
            night,
            reset_ramps,
            scheme,
            disable_fade,
            fade_sleep_duration,
            sleep_duration,
            location,
            method,
            time,
        };

        let v = match verbosity {
            VerbosityKind::Quite => Verbosity::Quite,
            VerbosityKind::Low => Verbosity::Low(w),
            VerbosityKind::High => Verbosity::High(w),
        };

        Ok((c, v))
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

    fn merge_with_cli_args(&mut self, cli_args: CliArgs) {
        let CliArgs { mode } = cli_args;

        match mode {
            ModeArgs::Daemon {
                c,
                disable_fade,
                fade_sleep_duration,
                sleep_duration,
            } => {
                if let Some(t) = fade_sleep_duration {
                    self.fade_sleep_duration = Duration::from_millis(t as u64);
                }
                if let Some(t) = sleep_duration {
                    self.sleep_duration = Duration::from_millis(t as u64);
                }
                self.merge_with_cmd_args(c);
                self.disable_fade = disable_fade;
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
            verbosity,
            reset_ramps,
            method,
        } = args;

        let verbosity = match (verbosity.quite, verbosity.verbose) {
            (true, false) => VerbosityKind::Quite,
            (false, false) => VerbosityKind::Low,
            (false, true) => VerbosityKind::High,
            (true, true) => unreachable!(), // clap will return error
        };
        self.verbosity = verbosity;

        self.reset_ramps = reset_ramps;
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
            fade_sleep_duration,
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

        if let Some(t) = fade_sleep_duration {
            self.fade_sleep_duration = Duration::from_millis(t as u64);
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
    fn new(config_path: Option<&Path>) -> Result<Self> {
        #[cfg(unix)]
        let system_config =
            Path::new(formatcp!("/etc/{PKG_NAME}/config.toml"));
        let local_config =
            dirs::config_dir().map(|d| d.join(PKG_NAME).join("config.toml"));
        let user_config = config_path
            .map(|p| match p.is_file() {
                true => Ok(p),
                false => Err(anyhow!("e")),
            })
            .transpose()?
            .or(local_config.as_deref())
            .ok_or(anyhow!("user_config"))?;

        let mut config = Self::default();
        let mut buf = String::new();
        let mut read = |path: &Path| -> Result<()> {
            if path.is_file() {
                File::open(path)?.read_to_string(&mut buf)?;
                let cfg = toml::from_str(&buf)?;
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
            fade_sleep_duration,
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
        if let Some(t) = fade_sleep_duration {
            self.fade_sleep_duration = Some(t);
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

impl Default for LocationProviderType {
    fn default() -> Self {
        Self::Manual(Default::default())
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

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self {
            day: ColorSettings::default_day(),
            night: ColorSettings::default_night(),
            mode: Default::default(),
            verbosity: Default::default(),
            reset_ramps: Default::default(),
            scheme: Default::default(),
            disable_fade: Default::default(),
            fade_sleep_duration: FADE_SLEEP_DURATION,
            sleep_duration: SLEEP_DURATION,
            method: Default::default(),
            location: Default::default(),
            time: Local::now,
        }
    }
}

// impl Default for Config {
//     fn default() -> Self {
//         ConfigBuilder {
//             method: Some(AdjustmentMethodType::Dummy),
//             ..Default::default()
//         }
//         .build()
//         .unwrap_or_else(|_| unreachable!())
//         .0
//     }
// }

//
// Parse strings and numbers to strong types
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

impl From<NaiveTime> for TimeOffset {
    fn from(time: NaiveTime) -> Self {
        Self(time.num_seconds_from_midnight())
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
        s.trim().parse::<f64>()?.try_into()
    }
}

impl FromStr for Gamma {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match *s.split(':').map(str::trim).collect::<Vec<_>>().as_slice() {
            [r, g, b] => {
                [r.parse::<f64>()?, g.parse::<f64>()?, b.parse::<f64>()?]
                    .try_into()
            }
            [rbg] => rbg.parse::<f64>()?.try_into(),
            _ => Err(anyhow!("gamma")),
        }
    }
}

impl FromStr for Latitude {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.trim().parse::<f64>()?.try_into()
    }
}

impl FromStr for Longitude {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.trim().parse::<f64>()?.try_into()
    }
}

impl FromStr for Location {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match *s.split(':').collect::<Vec<_>>().as_slice() {
            [lat, lon] => Ok(Self {
                lat: lat.parse()?,
                lon: lon.parse()?,
            }),
            _ => Err(anyhow!("location")),
        }
    }
}

impl FromStr for Time {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match *s.split(':').map(str::trim).collect::<Vec<_>>().as_slice() {
            [h, m] => (h.parse()?, m.parse()?).try_into(),
            _ => Err(anyhow!("time")),
        }
    }
}

impl FromStr for TimeOffset {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.parse::<Time>()?.into())
    }
}

impl FromStr for Elevation {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.trim().parse::<f64>()?.try_into()
    }
}

impl FromStr for TimeRange {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match *s.split('-').collect::<Vec<_>>().as_slice() {
            [start, end] => {
                (start.parse::<TimeOffset>()?, end.parse::<TimeOffset>()?)
                    .try_into()
            }
            [time] => {
                let t = time.parse::<TimeOffset>()?;
                Ok(Self { start: t, end: t })
            }
            _ => Err(anyhow!("time_range")),
        }
    }
}

impl FromStr for TimeRanges {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let time = match *s.split('-').collect::<Vec<_>>().as_slice() {
            [dawn, dusk] => Self {
                dawn: dawn.parse()?,
                dusk: dusk.parse()?,
            },
            [dawn_start, dawn_end, dusk_start, dusk_end] => Self {
                dawn: (dawn_start.parse()?, dawn_end.parse()?).try_into()?,
                dusk: (dusk_start.parse()?, dusk_end.parse()?).try_into()?,
            },
            _ => Err(anyhow!("time_ranges"))?,
        };

        if time.dawn.end < time.dusk.start {
            Ok(time)
        } else {
            Err(anyhow!("time_ranges"))
        }
    }
}

impl FromStr for ElevationRange {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match *s.split(':').collect::<Vec<_>>().as_slice() {
            [high, low] => {
                let high = high.parse()?;
                let low = low.parse()?;
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

impl FromStr for TransitionScheme {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Err(())
            .or_else(|_| Ok::<_, Self::Err>(Self::Time(s.parse()?)))
            .or_else(|_| Ok::<_, Self::Err>(Self::Elevation(s.parse()?)))
            .map_err(|_| anyhow!("asdf"))
    }
}

impl FromStr for LocationProviderType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        #[allow(clippy::match_single_binding)]
        match s {
            // "geoclue2" => Ok(Self::Geoclue2),
            _ => s.parse().map(|l| Self::Manual(Manual::new(l))),
        }
        .map_err(|_| anyhow!("asdf"))
    }
}

impl FromStr for AdjustmentMethodType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let num = |o: Option<&str>| match o {
            None => Ok::<_, anyhow::Error>(None),
            Some(s) => Ok(Some(s.parse()?)),
        };
        let crtcs = |o: Option<&str>| match o {
            None => Ok(Vec::new()),
            Some(s) => s
                .split(',')
                .map(|s| Ok(s.trim().parse()?))
                .collect::<Result<Vec<_>>>(),
        };

        let drm = |n: Option<&str>, c: Option<&str>| {
            Ok(Self::Drm {
                card_num: num(n)?,
                crtcs: crtcs(c)?,
            })
        };
        let randr = |n: Option<&str>, c: Option<&str>| {
            Ok(Self::Randr {
                screen_num: num(n)?,
                crtcs: crtcs(c)?,
            })
        };

        match s.split(':').map(str::trim).collect::<Vec<_>>().as_slice() {
            ["dummy"] => Ok(Self::Dummy),
            ["drm"] => drm(None, None),
            ["drm", n] => drm(Some(n), None),
            ["drm", n, c] => drm(Some(n), Some(c)),
            ["vidmode"] => Ok(Self::Vidmode { screen_num: None }),
            ["vidmode", n] => Ok(Self::Vidmode {
                screen_num: Some(n.parse()?),
            }),
            ["randr"] => randr(None, None),
            ["randr", n] => randr(Some(n), None),
            ["randr", n, c] => randr(Some(n), Some(c)),
            _ => Err(anyhow!("method")),
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

impl<T: Clone + FromStr<Err = anyhow::Error>> FromStr for DayNight<T> {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match *s.split('-').collect::<Vec<_>>().as_slice() {
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

//

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

impl<'de, T> Deserialize<'de> for DayNight<T>
where
    T: Clone + FromStr<Err = anyhow::Error>,
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

#[macro_export]
macro_rules! write_verbose {
    ($dst:expr, $($arg:tt)*) => {
        match $dst {
            $crate::config::Verbosity::Quite
            | $crate::config::Verbosity::Low(_) => Ok(()),
            $crate::config::Verbosity::High(w) => write!(w, $($arg)*),
        }
    };
}

#[macro_export]
macro_rules! writeln_verbose {
    ($dst:expr $(,)?) => {
        $crate::write_verbose!($dst, "\n")
    };
    ($dst:expr, $($arg:tt)*) => {
        match $dst {
            $crate::config::Verbosity::Quite
            | $crate::config::Verbosity::Low(_) => Ok(()),
            $crate::config::Verbosity::High(w) => writeln!(w, $($arg)*),
        }
    };
}

impl<W: Write> Write for Verbosity<W> {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        match self {
            Verbosity::Quite => Ok(buf.len()),
            Verbosity::Low(w) => w.write(buf),
            Verbosity::High(w) => w.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Verbosity::Quite => Ok(()),
            Verbosity::Low(w) => w.flush(),
            Verbosity::High(w) => w.flush(),
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_config_toml_has_default_values() -> Result<()> {
        const CONFIG_TOML: &str =
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/config.toml"));
        let cfg = toml::from_str(CONFIG_TOML)?;
        let mut config = ConfigBuilder::default();
        config.merge_with_config_file(cfg);
        assert_eq!(config, ConfigBuilder::default());
        Ok(())
    }

    // #[test]
    // fn test_config_default() {
    //     Config::default();
    // }

    // TODO: assert_eq default config with config.toml

    // TODO: add conversion tests

    // TODO: test help for possible values of enums
}
