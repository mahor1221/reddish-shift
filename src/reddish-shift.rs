/*  redshift.rs -- Main program
    This file is part of <https://github.com/mahor1221/reddish-shift>.
    Copyright (C) 2024 Mahor Foruzesh <mahor1221@gmail.com>
    Ported from Redshift <https://github.com/jonls/redshift>.
    Copyright (c) 2009-2017  Jon Lund Steffensen <jonlst@gmail.com>

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

pub mod colorramp;
pub mod config;
pub mod gamma_drm;
pub mod gamma_dummy;
pub mod gamma_randr;
pub mod gamma_vidmode;
pub mod hooks;
pub mod location_geoclue2;
pub mod location_manual;
pub mod pipeutils;
pub mod signals;
pub mod solar;
pub mod systemtime;

use anyhow::{anyhow, Result};
use config::{
    AdjustmentMethod, Brightness, ColorSettings, Config, Elevation,
    ElevationRange, Gamma, Location, LocationProvider, Mode, Offset,
    Temperature, TimeRanges, TransitionScheme,
};

use gamma_dummy::Dummy;
use hooks::hooks_signal_period_change;
use libc::{
    __errno_location, exit, fprintf, fputs, pause, perror, poll, pollfd,
    printf, setlocale, setvbuf, strcmp,
};
use location_manual::Manual;
use signals::{disable, exiting, signals_install_handlers};
use solar::solar_elevation;
use std::{
    ffi::{c_char, c_double, c_float, c_int, c_short, c_uint, c_ulong, CStr},
    fmt::{Display, Formatter, Result as FmtResult},
    ptr::addr_of_mut,
};
use systemtime::{systemtime_get_time, systemtime_msleep};

pub type Nfds = c_ulong;
pub type SigAtomic = c_int;

extern "C" {
    pub static stdout: *mut libc::FILE;
    pub static stderr: *mut libc::FILE;
}

// TODO: replace magic numbers with the const values below:

// Duration of sleep between screen updates (milliseconds).
const SLEEP_DURATION: u32 = 5000;
const SLEEP_DURATION_SHORT: u32 = 100;
// Length of fade in numbers of short sleep durations.
const FADE_LENGTH: u32 = 40;

// Names of periods supplied to scripts.
pub static mut period_names: [*const c_char; 4] = [
    // TRANSLATORS: Name printed when period of day is unknown
    b"None\0" as *const u8 as *const c_char,
    b"Daytime\0" as *const u8 as *const c_char,
    b"Night\0" as *const u8 as *const c_char,
    b"Transition\0" as *const u8 as *const c_char,
];

// Periods of day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Period {
    Daytime,
    Transition,
    Night,
}

// Between 0 and 1
struct TransitionProgress(f64);

// Between 0 and 1
struct Alpha(f64);

impl AsRef<f64> for TransitionProgress {
    fn as_ref(&self) -> &f64 {
        &self.0
    }
}

impl AsRef<f64> for Alpha {
    fn as_ref(&self) -> &f64 {
        &self.0
    }
}

impl From<TransitionProgress> for Alpha {
    fn from(n: TransitionProgress) -> Self {
        Alpha(n.0)
    }
}

impl Period {
    /// Determine which period we are currently in based on time offset
    fn from_time(time_ranges: &TimeRanges, offset: Offset) -> Self {
        let TimeRanges { dawn, dusk } = time_ranges;
        if offset < dawn.start || offset >= dusk.end {
            Self::Night
        } else if offset >= dawn.end && offset < dusk.start {
            Self::Daytime
        } else {
            Self::Transition
        }
    }

    /// Determine which period we are currently in based on solar elevation
    fn from_elevation(
        elevation_range: ElevationRange,
        elevation: Elevation,
    ) -> Self {
        let ElevationRange { high, low } = elevation_range;
        if elevation < low {
            Self::Night
        } else if elevation < high {
            Self::Daytime
        } else {
            Self::Transition
        }
    }
}

impl TransitionProgress {
    /// Determine how far through the transition we are based on time offset
    fn from_time(time_range: &TimeRanges, offset: Offset) -> Self {
        let TimeRanges { dawn, dusk } = time_range;
        let sub = |a: Offset, b: Offset| (a.as_ref() - b.as_ref()) as f64;

        if offset < dawn.start || offset >= dusk.end {
            Self(0.0)
        } else if offset < dawn.end {
            Self(sub(dawn.start, offset) / sub(dawn.start, dawn.end))
        } else if offset > dusk.start {
            Self(sub(dusk.end, offset) / sub(dusk.end, dusk.start))
        } else {
            Self(1.0)
        }
    }

    /// Determine how far through the transition we are based on elevation
    fn from_elevation(
        elevation_range: ElevationRange,
        elevation: Elevation,
    ) -> Self {
        let ElevationRange { high, low } = elevation_range;
        let sub = |a: Elevation, b: Elevation| (a.as_ref() - b.as_ref());

        if elevation < elevation_range.low {
            Self(0.0)
        } else if elevation < elevation_range.high {
            Self(sub(low, elevation) / sub(low, high))
        } else {
            Self(1.0)
        }
    }
}

// Print verbose description of the given period.
struct PeriodDisplay(Period, TransitionProgress);
impl Display for PeriodDisplay {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let PeriodDisplay(period, transition_profress) = self;
        let n = *transition_profress.as_ref();
        match period {
            Period::Daytime => f.write_str("Period: Daytime"),
            Period::Night => f.write_str("Period: Night"),
            Period::Transition => write!(f, "Period: Transition ({n:.2}% day)"),
        }
    }
}

impl Display for Location {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        const NORTH: &str = "N";
        const EAST: &str = "E";
        const WEST: &str = "W";
        const SOUTH: &str = "S";

        let lat = self.longitude.as_ref().abs();
        let ns = if *self.latitude.as_ref() >= 0.0 {
            NORTH
        } else {
            SOUTH
        };
        let lon = self.longitude.as_ref().abs();
        let ew = if *self.longitude.as_ref() >= 0.0 {
            EAST
        } else {
            WEST
        };

        // TRANSLATORS: Append degree symbols after %f if possible.
        // The string following each number is an abbreviation for
        // north, source, east or west (N, S, E, W).
        write!(f, "Location: {lat:.2} {ns}, {lon:.2} {ew}")
    }
}

// Return number of seconds since midnight from timestamp.
fn get_seconds_since_midnight(_timestamp: c_double) -> c_int {
    // let mut t: time_t = timestamp as time_t;
    // let mut tm: tm = tm {
    //     tm_sec: 0,
    //     tm_min: 0,
    //     tm_hour: 0,
    //     tm_mday: 0,
    //     tm_mon: 0,
    //     tm_year: 0,
    //     tm_wday: 0,
    //     tm_yday: 0,
    //     tm_isdst: 0,
    //     tm_gmtoff: 0,
    //     tm_zone: std::ptr::null::<c_char>(),
    // };
    // localtime_r(&mut t, &mut tm);
    // tm.tm_sec + tm.tm_min * 60 as c_int + tm.tm_hour * 3600 as c_int
    todo!()
}

fn clamp(alpha: impl AsRef<f64>) -> f64 {
    let alpha = *alpha.as_ref();
    if 0.0 > alpha {
        0.0
    } else if alpha < 1.0 {
        alpha
    } else {
        1.0
    }
}

// Interpolate color setting structs given alpha.
fn interpolate_color_settings(
    lhs: &ColorSettings,
    rhs: &ColorSettings,
    alpha: impl Into<Alpha>,
) -> ColorSettings {
    let alpha: Alpha = alpha.into();
    // TODO: what does this do?
    let alpha = clamp(alpha);

    let temperature: Temperature =
        (((1.0 - alpha) * *lhs.temperature.as_ref() as f64
            + alpha * *rhs.temperature.as_ref() as f64) as u16)
            .try_into()
            .unwrap_or_else(|_| unreachable!());

    let brightness: Brightness = ((1.0 - alpha) * *lhs.brightness.as_ref()
        + alpha * *rhs.brightness.as_ref())
    .try_into()
    .unwrap_or_else(|_| unreachable!());

    let gamma: Gamma = (0..3)
        .map(|i| {
            (1.0 - alpha) * lhs.gamma.as_ref()[i]
                + alpha * rhs.gamma.as_ref()[i]
        })
        .collect::<Vec<_>>()
        .try_into()
        .unwrap_or_else(|_| unreachable!());

    ColorSettings {
        temperature,
        gamma,
        brightness,
    }
}

// Return 1 if color settings have major differences, otherwise 0.
// Used to determine if a fade should be applied in continual mode.

impl ColorSettings {
    fn is_very_diff_from(&self, other: &Self) -> bool {
        (*self.temperature.as_ref() as i16 - *other.temperature.as_ref() as i16)
            .abs()
            > 25
            || (self.brightness.as_ref() - other.brightness.as_ref()).abs()
                > 0.1
            || (self.gamma.as_ref()[0] - other.gamma.as_ref()[0]).abs() > 0.1
            || (self.gamma.as_ref()[1] - other.gamma.as_ref()[1]).abs() > 0.1
            || (self.gamma.as_ref()[2] - other.gamma.as_ref()[2]).abs() > 0.1
    }
}

// Easing function for fade.
// See https://github.com/mietek/ease-tween
fn ease_fade(t: f64) -> f64 {
    if t <= 0.0 {
        0.0
    } else if t >= 1.0 {
        1.0
    } else {
        1.0042954579734844
            * (-6.404173895841566 * (-7.290824133098134 * t).exp()).exp()
    }
}

// Run continual mode loop
// This is the main loop of the continual mode which keeps track of the
// current time and continuously updates the screen to the appropriate
// color temperature.
// fn run_continual_mode(config: &Config) -> c_int {
//     let mut r: c_int = 0;

//     // Short fade parameters
//     let mut fade_length = 0;
//     let mut fade_time = 0;
//     let mut fade_start_interp: ColorSetting = ColorSetting {
//         temperature: 0,
//         gamma: [0.; 3],
//         brightness: 0.,
//     };

//     r = signals_install_handlers();
//     if r < 0 as c_int {
//         return r;
//     }

//     // Save previous parameters so we can avoid printing status updates if
//     // the values did not change.
//     let mut prev_period: Period;

//     // Previous target color setting and current actual color setting.
//     // Actual color setting takes into account the current color fade.
//     let mut prev_target_interp = ColorSetting::default();
//     let mut interp = ColorSetting::default();
//     let mut loc: location_t = location_t {
//         lat: ::core::f32::NAN,
//         lon: ::core::f32::NAN,
//     };

//     let need_location = config.transition_scheme.select == TransitionSchemeKind::Elevation;

//     if need_location {
//         // println!("Waiting for initial location to become available...");

//         // Get initial location from provider
//         // r = provider_get_location(provider, location_state, -(1 as c_int), &mut loc);
//         // eprintln!("Unable to get location from provider.");
//         // print_location(&mut loc);
//         todo!()
//     }

//     if config.verbose {
//         // b"Color temperature: %uK\n\0" as *const u8 as *const c_char,
//         // interp.temperature,
//         // b"Brightness: %.2f\n\0" as *const u8 as *const c_char,
//         // interp.brightness as c_double,
//         todo!()
//     }

//     // Continuously adjust color temperature
//     let mut done: c_int = 0 as c_int;
//     let mut prev_disabled: c_int = 1 as c_int;
//     let mut disabled: c_int = 0 as c_int;
//     let mut location_available: c_int = 1 as c_int;
//     loop {
//         // Check to see if disable signal was caught
//         if disable != 0 && done == 0 {
//             disabled = (disabled == 0) as c_int;
//             ::core::ptr::write_volatile(addr_of_mut!(disable) as *mut SigAtomic, 0 as c_int);
//         }

//         // Check to see if exit signal was caught
//         if exiting != 0 {
//             if done != 0 {
//                 // On second signal stop the ongoing fade.
//                 break;
//             }
//             done = 1 as c_int;
//             disabled = 1 as c_int;
//             ::core::ptr::write_volatile(addr_of_mut!(exiting) as *mut SigAtomic, 0 as c_int);
//         }

//         // Print status change
//         if config.verbose && disabled != prev_disabled {
//             // printf(
//             //     // gettext(
//             //     b"Status: %s\n\0" as *const u8 as *const c_char,
//             //     if disabled != 0 {
//             //         // gettext(
//             //         b"Disabled\0" as *const u8 as *const c_char
//             //     } else {
//             //         // gettext(
//             //         b"Enabled\0" as *const u8 as *const c_char
//             //     },
//             // );
//             todo!()
//         }
//         prev_disabled = disabled;

//         // Read timestamp
//         let mut now: c_double = 0.;
//         r = systemtime_get_time(&mut now);
//         if r < 0 as c_int {
//             eprintln!("Unable to read system time.");
//             return -(1 as c_int);
//         }

//         let (period, transition_prog): (Period, TransitionProgress) =
//             match config.transition_scheme.select {
//                 TransitionSchemeKind::TimeRanges => {
//                     // let time_offset: c_int = get_seconds_since_midnight(now);
//                     // period = get_period_from_time(scheme, time_offset);
//                     // transition_prog = get_transition_progress_from_time(scheme, time_offset);
//                     todo!()
//                 }
//                 TransitionSchemeKind::Elevation => {
//                     // let elevation: c_double =
//                     //     solar_elevation(now, loc.lat as c_double, loc.lon as c_double);
//                     // period = get_period_from_elevation(scheme, elevation);
//                     // transition_prog = get_transition_progress_from_elevation(scheme, elevation);
//                     todo!()
//                 }
//             };

//         // Use transition progress to get target color
//         //    temperature.
//         let target_interp = interpolate_color_settings(
//             &config.night_color_setting,
//             &config.day_color_setting,
//             transition_prog,
//         );

//         if disabled != 0 {
//             period = PERIOD_NONE;
//             color_setting_reset(&mut target_interp);
//         }
//         if done != 0 {
//             period = PERIOD_NONE;
//         }

//         // Print period if it changed during this update,
//         // or if we are in the transition period. In transition we
//         // print the progress, so we always print it in
//         // that case.
//         if config.verbose && (period != prev_period || period == Period::Transition) {
//             print_period(period, transition_prog);
//         }

//         /* Activate hooks if period changed */
//         if period != prev_period {
//             hooks_signal_period_change(prev_period, period);
//         }

//         // Start fade if the parameter differences are too big to apply
//         // instantly.
//         if config.fade
//             && (fade_length == 0 && is_color_setting_diff_major(&mut interp, &mut target_interp)
//                 || fade_length != 0 as c_int
//                     && is_color_setting_diff_major(&mut target_interp, &mut prev_target_interp))
//         {
//             fade_length = 40 as c_int;
//             fade_time = 0 as c_int;
//             fade_start_interp = interp;
//         }

//         // Handle ongoing fade
//         if fade_length != 0 as c_int {
//             fade_time += 1;
//             let frac = fade_time as f32 / fade_length as f32;
//             let alpha = if 0.0 > ease_fade(frac) {
//                 Alpha(0.0)
//             } else if ease_fade(frac) < 1.0 {
//                 Alpha(ease_fade(frac))
//             } else {
//                 Alpha(1.0)
//             };
//             let interp =
//                 interpolate_color_settings(&mut fade_start_interp, &mut target_interp, alpha);
//             if fade_time > fade_length {
//                 fade_time = 0;
//                 fade_length = 0;
//             }
//         } else {
//             interp = target_interp;
//         }

//         // Break loop when done and final fade is over
//         if done != 0 && fade_length == 0 as c_int {
//             break;
//         }

//         if config.verbose {
//             if prev_target_interp.temperature != target_interp.temperature {
//                 // b"Color temperature: %uK\n\0" as *const u8 as *const c_char,
//                 // target_interp.temperature,
//                 todo!()
//             }
//             if prev_target_interp.brightness != target_interp.brightness {
//                 // b"Brightness: %.2f\n\0" as *const u8 as *const c_char,
//                 // target_interp.brightness as c_double,
//                 todo!()
//             }
//         }

//         // Adjust temperature
//         r = ((*method).set_temperature).expect("non-null function pointer")(
//             method_state,
//             &mut interp,
//             preserve_gamma,
//         );
//         // eprintln!("Temperature adjustment failed.");

//         // Save period and target color setting as previous
//         prev_period = period;
//         prev_target_interp = target_interp;

//         // Sleep length depends on whether a fade is ongoing.
//         let mut delay = 5000;
//         if fade_length != 0 {
//             delay = 100;
//         }

//         // Update location.
//         let mut loc_fd = -1;
//         if need_location {
//             loc_fd = ((*provider).get_fd).expect("non-null function pointer")(location_state);
//         }

//         if loc_fd >= 0 as c_int {
//             // Provider is dynamic.
//             let mut pollfds: [pollfd; 1] = [pollfd {
//                 fd: 0,
//                 events: 0,
//                 revents: 0,
//             }; 1];

//             pollfds[0 as c_int as usize].fd = loc_fd;
//             pollfds[0 as c_int as usize].events = 0x1 as c_int as c_short;
//             let mut r_0: c_int = poll(pollfds.as_mut_ptr(), 1 as c_int as Nfds, delay);
//             if r_0 < 0 as c_int {
//                 if *__errno_location() == 4 as c_int {
//                     continue;
//                 }
//                 perror(b"poll\0" as *const u8 as *const c_char);
//                 eprintln!("Unable to get location from provider.");
//                 return -(1 as c_int);
//             } else {
//                 if r_0 == 0 as c_int {
//                     continue;
//                 }

//                 // Get new location and availability
//                 // information.
//                 let mut new_loc: location_t = location_t { lat: 0., lon: 0. };
//                 let mut new_available: c_int = 0;
//                 r_0 = ((*provider).handle).expect("non-null function pointer")(
//                     location_state,
//                     &mut new_loc,
//                     &mut new_available,
//                 );
//                 if r_0 < 0 as c_int {
//                     eprintln!("Unable to get location from provider.");
//                     return -(1 as c_int);
//                 }
//                 if new_available == 0 && new_available != location_available {
//                     eprintln!("Location is temporarily unavailable; Using previous location until it becomes available...");
//                 }

//                 if new_available != 0
//                     && (new_loc.lat != loc.lat
//                         || new_loc.lon != loc.lon
//                         || new_available != location_available)
//                 {
//                     loc = new_loc;
//                     print_location(&mut loc);
//                 }
//                 location_available = new_available;
//             }
//         } else {
//             systemtime_msleep(delay as c_uint);
//         }
//     }

//     // Restore saved gamma ramps
//     ((*method).restore).expect("non-null function pointer")(method_state);
//     0 as c_int
// }

pub trait IsDefault {
    fn is_default(&self) -> bool;
}

impl<T: Default + PartialEq> IsDefault for T {
    fn is_default(&self) -> bool {
        *self == T::default()
    }
}

pub trait Provider {
    // Allocate storage and make connections that depend on options
    fn start() {
        todo!()
    }

    // Listen and handle location updates.
    // pub get_fd(&self) -> c_int;
    fn fd() -> Result<()> {
        todo!()
    }
    fn get_location(&self) -> Result<(Location, bool)> {
        Err(anyhow!("Unable to get location from provider"))
    }
}

pub trait Adjuster {
    // If true, this method will be tried if none is explicitly chosen
    // int autostart;

    // Allocate storage and make connections that depend on options
    fn start() {
        todo!()
    }

    // Restore the adjustment to the state before start was called
    fn restore() {
        todo!()
    }

    // Set a specific color temperature
    fn set_color(
        &self,
        cs: &ColorSettings,
        preserve_gamma: bool,
    ) -> Result<()> {
        Err(anyhow!("Temperature adjustment failed"))
    }
}

impl Provider for LocationProvider {
    fn get_location(&self) -> Result<(Location, bool)> {
        match self {
            LocationProvider::Manual(t) => t.get_location(),
            LocationProvider::Geoclue2(t) => t.get_location(),
        }
    }
}

impl Adjuster for AdjustmentMethod {
    fn start() {}
    fn restore() {}
    fn set_color(
        &self,
        cs: &ColorSettings,
        preserve_gamma: bool,
    ) -> Result<()> {
        match self {
            AdjustmentMethod::Dummy(t) => t.set_color(cs, preserve_gamma),
            AdjustmentMethod::Randr(t) => t.set_color(cs, preserve_gamma),
            AdjustmentMethod::Drm(t) => t.set_color(cs, preserve_gamma),
            AdjustmentMethod::Vidmode(t) => t.set_color(cs, preserve_gamma),
        }

        // // In Quartz (OSX) the gamma adjustments will automatically
        // //    revert when the process exits. Therefore, we have to loop
        // //    until CTRL-C is received.
        // if strcmp(options.method.name, "quartz") == 0 as c_int {
        //     // b"Press ctrl-c to stop...\n\0" as *const u8 as *const c_char,
        //     pause();
        //     todo!()
        // }
    }
}

impl Adjuster for Option<AdjustmentMethod> {
    fn set_color(
        &self,
        cs: &ColorSettings,
        preserve_gamma: bool,
    ) -> Result<()> {
        self.as_ref()
            .unwrap_or_else(|| unreachable!())
            .set_color(cs, preserve_gamma)
    }
}

unsafe fn main_0() -> Result<()> {
    // // Init locale
    // setlocale(0 as c_int, b"\0" as *const u8 as *const c_char);
    // setlocale(5 as c_int, b"\0" as *const u8 as *const c_char);

    // // Internationalisation
    // bindtextdomain(
    //     b"redshift\0" as *const u8 as *const c_char,
    //     b"/home/mahor/redshift/root/share/locale\0" as *const u8 as *const c_char,
    // );
    // textdomain(b"redshift\0" as *const u8 as *const c_char);

    // // Flush messages consistently even if redirected to a pipe or
    // // file.  Change the flush behaviour to line-buffered, without
    // // changing the actual buffers being used.
    // setvbuf(stdout, std::ptr::null_mut::<c_char>(), 1 as c_int, 0);
    // setvbuf(stderr, std::ptr::null_mut::<c_char>(), 1 as c_int, 0);

    let cfg = Config::new()?;

    // use TransitionScheme::*;
    // cfg.need_location = match (&cfg.mode, &cfg.scheme) {
    //     (Mode::Daemon | Mode::Oneshot, Elevation(_)) => true,
    //     (Mode::Set | Mode::Reset, _) | (_, Time(_)) => false,
    // };

    // match (cfg.need_location, &cfg.location) {
    //     (true, LocationProvider::Manual(loc)) if loc.is_default() => {
    //         eprintln!("using default location");
    //     }
    //     _ => {}
    // }

    // if cfg.need_location {
    // if !(options.provider).is_null() {
    //     // Use provider specified on command line.
    // } else {
    //     // Try all providers, use the first that works.
    //     // b"Trying location provider `%s'...\n\0" as *const u8 as *const c_char,
    //     //     b"Trying next provider...\n\0" as *const u8 as *const c_char,
    //     //     b"Using provider `%s'.\n\0" as *const u8 as *const c_char,
    //     // b"No more location providers to try.\n\0" as *const u8 as *const c_char,
    // }

    // if options.verbosity {
    //     // TRANSLATORS: Append degree symbols if possible.
    //     // b"Solar elevations: day above %.1f, night below %.1f\n\0" as *const u8
    // }
    // }

    if let Mode::Daemon | Mode::Oneshot = cfg.mode {
        // if options.verbosity {
        //     // b"Temperatures: %dK at day, %dK at night\n\0" as *const u8 as *const c_char,
        // }
    }

    // if options.verbosity {
    //     // b"Brightness: %.2f:%.2f\n\0" as *const u8 as *const c_char,
    //     // options.scheme.day.brightness as c_double,
    //     // options.scheme.night.brightness as c_double,
    // }

    // if options.verbosity {
    //     // printf(
    //     //     // gettext(
    //     //     b"Gamma (%s): %.3f, %.3f, %.3f\n\0" as *const u8 as *const c_char,
    //     //     // gettext(
    //     //     b"Daytime\0" as *const u8 as *const c_char,
    //     //     options.scheme.day.gamma[0 as c_int as usize] as c_double,
    //     //     options.scheme.day.gamma[1 as c_int as usize] as c_double,
    //     //     options.scheme.day.gamma[2 as c_int as usize] as c_double,
    //     // );
    //     // printf(
    //     //     // gettext(
    //     //     b"Gamma (%s): %.3f, %.3f, %.3f\n\0" as *const u8 as *const c_char,
    //     //     // gettext(
    //     //     b"Night\0" as *const u8 as *const c_char,
    //     //     options.scheme.night.gamma[0 as c_int as usize] as c_double,
    //     //     options.scheme.night.gamma[1 as c_int as usize] as c_double,
    //     //     options.scheme.night.gamma[2 as c_int as usize] as c_double,
    //     // );
    // }

    // try all methods until one that works is found.
    // Gamma adjustment not needed for print mode
    if !cfg.dry_run {
        // if !(options.method).is_null() {
        //     // Use method specified on command line.
        // } else {
        //     // Try all methods, use the first that works.
        //     // b"Trying next method...\n\0" as *const u8 as *const c_char,
        //     // b"Using method `%s'.\n\0" as *const u8 as *const c_char,
        //     // Failure if no methods were successful at this point.
        //     // b"No more methods to try.\n\0" as *const u8 as *const c_char,
        // }
    }

    match cfg.mode {
        Mode::Daemon => {
            // r = run_continual_mode(
            //     options.provider,
            //     location_state,
            //     scheme,
            //     options.method,
            //     method_state,
            //     options.use_fade,
            //     options.preserve_gamma,
            //     options.verbose,
            // );
            // if r < 0 as c_int {
            //     exit(1 as c_int);
            // }
        }

        Mode::Oneshot => {
            // if cfg.need_location {
            // b"Waiting for current location to become available...\n\0" as *const u8

            // Wait for location provider.
            // b"Unable to get location from provider.\n\0" as *const u8 as *const c_char,
            // print_location(&mut loc);
            // }

            // let now: c_double = systemtime_get_time();
            // b"Unable to read system time.\n\0" as *const u8 as *const c_char,

            let (period, transition_prog): (Period, TransitionProgress) =
                match cfg.scheme {
                    TransitionScheme::Time(_) => {
                        // let time_offset: c_int = get_seconds_since_midnight(now);
                        // period = get_period_from_time(scheme, time_offset);
                        // transition_prog = get_transition_progress_from_time(scheme, time_offset);
                        todo!()
                    }
                    TransitionScheme::Elevation(_) => {
                        // // Current angular elevation of the sun
                        // let elevation: c_double = solar_elevation(now, loc.lat, loc.lon);
                        // if config.verbose {
                        //     // TRANSLATORS: Append degree symbol if possible.
                        //     // b"Solar elevation: %f\n\0" as *const u8 as *const c_char,
                        //     todo!()
                        // }
                        // period = get_period_from_elevation(scheme, elevation);
                        // transition_prog = get_transition_progress_from_elevation(scheme, elevation);
                        todo!()
                    }
                };

            // Use transition progress to set color temperature
            let interp: ColorSettings = interpolate_color_settings(
                &cfg.night,
                &cfg.day,
                transition_prog,
            );

            // if options.verbosity {
            //     // print_period(period, transition_prog);
            //     // b"Color temperature: %uK\n\0" as *const u8 as *const c_char,
            //     // b"Brightness: %.2f\n\0" as *const u8 as *const c_char,
            // }

            if !cfg.dry_run {
                // Adjust temperature
                // r = ((*options.method).set_temperature).expect("non-null function pointer")(
                // b"Temperature adjustment failed.\n\0" as *const u8 as *const c_char,

                // // In Quartz (macOS) the gamma adjustments will
                // // automatically revert when the process exits.
                // // Therefore, we have to loop until CTRL-C is received.
                // if strcmp(options.method.name, "quartz") == 0 {
                //     // b"Press ctrl-c to stop...\n" as *const u8 as *const c_char,
                //     pause();
                //     todo!()
                // }
            }
        }

        Mode::Set => {
            // if cfg.verbosity {
            //     // "Color temperature: %uK\n"
            // }
            cfg.method.set_color(&cfg.day, cfg.preserve_gamma)?;
        }

        Mode::Reset => {
            // // Reset screen
            // let mut reset = ColorSetting::default();
            // ((*options.method).set_temperature).expect("non-null function pointer")(
            //     method_state,
            //     &mut reset,
            //     0 as c_int,
            // );
            // // b"Temperature adjustment failed.\n\0" as *const u8 as *const c_char,

            // // In Quartz (OSX) the gamma adjustments will automatically
            // // revert when the process exits. Therefore, we have to loop
            // // until CTRL-C is received.
            // if strcmp(options.method.name, "quartz") == 0 as c_int {
            //     // b"Press ctrl-c to stop...\n\0" as *const u8 as *const c_char,
            //     pause();
            // }
        }
    }

    Ok(())
}

fn main() {
    unsafe { main_0().unwrap() }
}
