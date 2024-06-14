/*  config.rs -- Command line interface
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

use crate::types::{
    AdjustmentMethodType, Brightness, BrightnessRange, ColorSettings, Gamma,
    GammaRange, LocationProviderType, Temperature, TemperatureRange,
    TransitionScheme, DEFAULT_TEMPERATURE, DEFAULT_TEMPERATURE_DAY,
    DEFAULT_TEMPERATURE_NIGHT,
};
use clap::{Args, Parser, Subcommand};
use const_format::formatcp;
use std::{path::PathBuf, str::FromStr};

const VERSION: &str = {
    const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
    const GIT_DESCRIBE: &str = env!("VERGEN_GIT_DESCRIBE");
    const GIT_COMMIT_DATE: &str = env!("VERGEN_GIT_COMMIT_DATE");

    formatcp!("{PKG_VERSION} ({GIT_DESCRIBE} {GIT_COMMIT_DATE})")
};

const VERSION_LONG: &str = {
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

const ABOUT_GAMMA: &str = "Additional gamma correction to apply";

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
        "Usage: PKG_NAME -l LAT:LON -t DAY:NIGHT [OPTIONS...]

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

#[derive(Debug, Parser)]
#[command(version = VERSION, long_version = VERSION_LONG, about)]
#[command(propagate_version = true)]
pub struct CliArgs {
    #[command(subcommand)]
    pub mode: ModeArgs,
}

#[derive(Debug, Subcommand)]
pub enum ModeArgs {
    #[command(about)]
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
    #[command(
        about = "One shot mode (do not continuously adjust color temperature)    "
    )]
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
pub struct ColorSettingsArgs {
    #[arg(long, short, value_parser = Temperature::from_str)]
    pub temperature: Option<Temperature>,
    #[arg(long, short, value_parser = Gamma::from_str)]
    pub gamma: Option<Gamma>,
    #[arg(long, short, value_parser = Brightness::from_str)]
    pub brightness: Option<Brightness>,
}

#[derive(Debug, Args)]
pub struct CmdInnerArgs {
    #[arg(
        long,
        short,
        value_name = "ADJUSTMENT_METHOD[:DISPLAY[:CRTC1,CRTC2,...]]",
        value_parser = AdjustmentMethodType::from_str
    )]
    pub method: Option<AdjustmentMethodType>,

    #[arg(long)]
    pub reset_ramps: bool,

    #[arg(long, short, display_order(100), value_name = "FILE")]
    pub config: Option<PathBuf>,
    #[command(flatten)]
    pub verbosity: VerbosityArgs,
}

#[derive(Debug, Args)]
#[group(multiple = false)]
pub struct VerbosityArgs {
    #[arg(long, short, display_order(100))]
    pub quite: bool,
    #[arg(long, short, display_order(100))]
    pub verbose: bool,
}

#[derive(Debug, Args)]
pub struct CmdArgs {
    #[arg(long, short, value_name = "TEMPERATURE_RANGE", value_parser = TemperatureRange::from_str)]
    pub temperature: Option<TemperatureRange>,
    #[arg(long, short, value_name = "BRIGHTNESS_RANGE", value_parser = BrightnessRange::from_str)]
    pub brightness: Option<BrightnessRange>,
    #[arg(long, short, value_name = "GAMMA_RANGE", value_parser = GammaRange::from_str)]
    pub gamma: Option<GammaRange>,

    #[arg(
        long,
        short,
        value_name = "TIME | ELEVATION",
        value_parser = TransitionScheme::from_str
    )]
    pub scheme: Option<TransitionScheme>,

    #[arg(
        long,
        short,
        value_name = "LOCATION_PROVIDER | LOCATION",
        value_parser = LocationProviderType::from_str,
    )]
    pub location: Option<LocationProviderType>,

    #[command(flatten)]
    pub i: CmdInnerArgs,
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
