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

use crate::{
    config::{DEFAULT_SLEEP_DURATION, DEFAULT_SLEEP_DURATION_SHORT},
    types::{
        AdjustmentMethodType as AdjMethod, Brightness, BrightnessRange, Gamma,
        GammaRange, LocationProviderType as LocProvider, Temperature,
        TemperatureRange, TransitionScheme, MAX_TEMPERATURE, MIN_TEMPERATURE,
    },
};
use clap::{Args, Parser, Subcommand};
use const_format::formatcp;
use indoc::formatdoc;
use std::{path::PathBuf, str::FromStr};

const VERSION: &str = {
    const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
    const GIT_DESCRIBE: &str = env!("VERGEN_GIT_DESCRIBE");
    const GIT_COMMIT_DATE: &str = env!("VERGEN_GIT_COMMIT_DATE");

    formatcp!("{PKG_VERSION} ({GIT_DESCRIBE} {GIT_COMMIT_DATE})")
};

const LONG_VERSION: &str = {
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

#[derive(Debug, Parser)]
#[command(about, version = VERSION, long_version = LONG_VERSION)]
#[command(propagate_version = true, next_line_help(false))]
pub struct CliArgs {
    #[command(subcommand)]
    pub mode: ModeArgs,
}

#[derive(Debug, Subcommand)]
pub enum ModeArgs {
    /// Apply screen color settings according to time of day continuously
    #[command(next_line_help(true))]
    Daemon {
        #[command(flatten)]
        c: CmdArgs,

        /// Disable fading between color temperatures
        #[arg(long)]
        disable_fade: bool,

        #[arg(long, value_name = "MILLISECONDS", help = formatdoc!("
        Duration of sleep between screen updates
        default: {DEFAULT_SLEEP_DURATION}ms"))]
        sleep_duration: Option<u16>,

        #[arg(long, value_name = "MILLISECONDS", help = formatdoc!("
        Duration of sleep between screen updates for fade
        default: {DEFAULT_SLEEP_DURATION_SHORT}ms"))]
        sleep_duration_short: Option<u16>,
    },

    /// Like daemon mode, but do not run continuously
    #[command(next_line_help(true))]
    Oneshot {
        #[command(flatten)]
        c: CmdArgs,
    },

    /// Apply a specific screen color settings
    #[command(next_line_help(true))]
    Set {
        #[command(flatten)]
        cs: ColorSettingsArgs,
        #[command(flatten)]
        i: CmdInnerArgs,
    },

    /// Remove adjustment from screen
    #[command(next_line_help(true))]
    Reset {
        #[command(flatten)]
        i: CmdInnerArgs,
    },

    /// Print all solar elevation angles for the next 24 hours
    #[command(next_line_help(true))]
    Print {
        /// Location to use, either set it manually or select a location provider.
        /// Negative values represent west and south, respectively.
        /// form: LATITUDE:LONGITUDE | LOCATION_PROVIDER
        /// location providers: geoclue2 (currently not available)
        /// default: 0:0 (Null island)
        /// e.g.: 51.48:0.0 (Greenwich)
        ///       geoclue2 (automatic geolocation updates)
        #[arg(verbatim_doc_comment)]
        #[arg(long, short, value_parser = LocProvider::from_str)]
        location: LocProvider,
    },
}

#[derive(Debug, Args)]
#[group(required = true, multiple = true)]
pub struct ColorSettingsArgs {
    /// Color temperature to apply
    /// The neutral temperature is 6500K. Using this value will not change the
    /// color temperature of the display. Setting the color temperature to a
    /// value higher than this results in more blue light, and setting a lower
    /// value will result in more red light.
    /// default: 6500
    /// e.g.: 4500
    #[arg(verbatim_doc_comment)]
    #[arg(long, short, value_parser = Temperature::from_str)]
    #[arg(value_name = formatcp!("{MIN_TEMPERATURE}-{MAX_TEMPERATURE}"))]
    pub temperature: Option<Temperature>,

    /// Additional gamma correction to apply
    /// default: 1.0
    /// e.g.: 0.9 (R=G=B=0.9)
    ///       0.8:0.9:0.9 (R=0.8, G=0.9, B=0.9)
    #[arg(verbatim_doc_comment)]
    #[arg(long, short, value_parser = Gamma::from_str)]
    #[arg(value_name = "0.1-1.0")]
    pub gamma: Option<Gamma>,

    /// Screen brightness to apply
    /// default: 1.0
    /// e.g.: 0.8
    #[arg(verbatim_doc_comment)]
    #[arg(long, short, value_parser = Brightness::from_str)]
    #[arg(value_name = "0.1-1.0")]
    pub brightness: Option<Brightness>,
}

#[derive(Debug, Args)]
pub struct CmdInnerArgs {
    /// Adjustment method to use to apply color settings
    /// form: METHOD[:DISPLAY_NUM|CARD_NUM[:CRTC1,CRTC2,...]]
    /// methods: dummy (does not affect the display)
    ///          randr (X RANDR extension)
    ///          vidmode (X VidMode extension)
    ///          drm (Direct Rendering Manager)
    /// e.g.: vidmode (apply to $DISPLAY)
    ///       vidmode:0 (apply to screen 0)
    ///       drm (apply to /dev/dri/card0)
    ///       drm:1 (apply to /dev/dri/card1)
    ///       drm:0:80 (apply to /dev/dri/card0 with crtc 80)
    ///       randr (apply to $DISPLAY)
    ///       randr:0 (apply to screen 0)
    ///       randr$DISPLAY:62,63 (apply to $DISPLAY with crtcs 62 and 63)
    #[arg(verbatim_doc_comment)]
    #[arg(long, short, value_parser = AdjMethod::from_str)]
    pub method: Option<AdjMethod>,

    /// Reset existing gamma ramps before applying new color settings
    #[arg(long)]
    pub reset_ramps: bool,

    /// Path of config file
    #[arg(long, short, value_name = "FILE", display_order(100))]
    pub config: Option<PathBuf>,
    #[command(flatten)]
    pub verbosity: VerbosityArgs,
}

#[derive(Debug, Args)]
#[group(multiple = false)]
pub struct VerbosityArgs {
    /// Suppress all output
    #[arg(long, short, display_order(100))]
    pub quite: bool,

    /// Use verbose output
    #[arg(long, short, display_order(100))]
    pub verbose: bool,
}

#[derive(Debug, Args)]
pub struct CmdArgs {
    /// Color temperature to set for day and night
    /// default: 6500-4500
    /// e.g.: 5000 (day=night=5000)
    ///       6500-4500 (day=6500, night=4500)
    #[arg(verbatim_doc_comment)]
    #[arg(long, short, value_parser = TemperatureRange::from_str)]
    #[arg(value_name = formatcp!("{MIN_TEMPERATURE}-{MAX_TEMPERATURE}"))]
    pub temperature: Option<TemperatureRange>,

    /// Additional gamma correction to apply for day and night
    /// default: 1.0
    /// e.g.: 0.9 (day=night=0.9)
    ///       1.0-0.8:0.9:0.9 (day=1.0, night=(R=0.8, G=0.9, B=0.9))
    #[arg(verbatim_doc_comment)]
    #[arg(long, short, value_parser = GammaRange::from_str)]
    #[arg(value_name = "0.1-1.0")]
    pub gamma: Option<GammaRange>,

    /// Screen brightness to apply for day and night
    /// default: 1.0
    /// e.g.: 0.8 (day=night=0.8)
    ///       1.0-0.8 (day=1.0, night=0.8)
    #[arg(verbatim_doc_comment)]
    #[arg(long, short, value_parser = BrightnessRange::from_str)]
    #[arg(value_name = "0.1-1.0")]
    pub brightness: Option<BrightnessRange>,

    /// Transition scheme to use, either time ranges or elevation angles. The
    /// default value is recommended for most users. You can also use the print
    /// command to see solar elevation angles for the next 24 hours
    /// form: TIME-TIME - TIME-TIME | TIME-TIME | DEGREE:DEGREE
    /// default: 3:-6
    /// e.g.: 6:00-7:45 - 18:35-20:15 (dawn=6:00-7:45, dusk=18:35-20:15)
    ///       7:45 - 18:35 (day starts at 7:45, night starts at 20:15)
    ///       3:-6 (above 3° is day, bellow -6° is night)
    #[arg(verbatim_doc_comment)]
    #[arg(long, short, value_parser = TransitionScheme::from_str)]
    pub scheme: Option<TransitionScheme>,

    /// Location to use, either set it manually or select a location provider.
    /// Negative values represent west and south, respectively.
    /// form: LATITUDE:LONGITUDE | LOCATION_PROVIDER
    /// location providers: geoclue2 (currently not available)
    /// default: 0:0 (Null island)
    /// e.g.: 51.48:0.0 (Greenwich)
    ///       geoclue2 (automatic geolocation updates)
    #[arg(verbatim_doc_comment)]
    #[arg(long, short, value_parser = LocProvider::from_str)]
    pub location: Option<LocProvider>,

    #[command(flatten)]
    pub i: CmdInnerArgs,
}
