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

use crate::{location_provider_t, location_state_t, location_t};
use anyhow::Result;
use config::{
    Brightness, ColorSetting, Config, Elevation, ElevationRange, Gamma, Location, Mode,
    Temperature, TimeOffset, TimeRanges, TransitionSchemeKind,
};
use gamma_drm::drm_gamma_method;
use gamma_dummy::{Dummy, GammaAdjuster};
use gamma_randr::randr_gamma_method;
use gamma_vidmode::vidmode_gamma_method;
use hooks::hooks_signal_period_change;
use itertools::Itertools;
use libc::{
    __errno_location, exit, fprintf, fputs, pause, perror, poll, pollfd, printf, setlocale,
    setvbuf, strcmp,
};
use location_geoclue2::geoclue2_location_provider;
use location_manual::{LocationProvider, Manual};
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

// // Location provider
// pub struct location_state_t;
// pub type location_provider_init_func = unsafe extern "C" fn(*mut *mut location_state_t) -> c_int;
// pub type location_provider_start_func = unsafe extern "C" fn(*mut location_state_t) -> c_int;
// pub type location_provider_free_func = unsafe extern "C" fn(*mut location_state_t) -> ();
// pub type location_provider_print_help_func = unsafe extern "C" fn(*mut FILE) -> ();
// pub type location_provider_set_option_func =
//     unsafe extern "C" fn(*mut location_state_t, *const c_char, *const c_char) -> c_int;
// pub type location_provider_get_fd_func = unsafe extern "C" fn(*mut location_state_t) -> c_int;
// pub type location_provider_handle_func =
//     unsafe extern "C" fn(*mut location_state_t, *mut location_t, *mut c_int) -> c_int;

// #[derive(Copy, Clone)]
// #[repr(C)]
// pub struct location_provider_t {
//     pub name: *mut c_char,

//     // Initialize state. Options can be set between init and start.
//     pub init: Option<location_provider_init_func>,
//     // Allocate storage and make connections that depend on options.
//     pub start: Option<location_provider_start_func>,
//     // Free all allocated storage and close connections.
//     pub free: Option<location_provider_free_func>,

//     // Print help on options for this location provider.
//     pub print_help: Option<location_provider_print_help_func>,
//     // Set an option key, value-pair.
//     pub set_option: Option<location_provider_set_option_func>,

//     // Listen and handle location updates.
//     pub get_fd: Option<location_provider_get_fd_func>,
//     pub handle: Option<location_provider_handle_func>,
// }

// // Names of periods supplied to scripts.
// pub static mut period_names: [*const c_char; 4] = [
//     // TRANSLATORS: Name printed when period of day is unknown
//     b"None\0" as *const u8 as *const c_char,
//     b"Daytime\0" as *const u8 as *const c_char,
//     b"Night\0" as *const u8 as *const c_char,
//     b"Transition\0" as *const u8 as *const c_char,
// ];

// Periods of day.
enum Period {
    Daytime,
    Transition,
    Night,
}

impl Period {
    // Determine which period we are currently in based on time offset.
    fn from_time(time_ranges: &TimeRanges, time: TimeOffset) -> Self {
        let TimeRanges { dawn, dusk } = time_ranges;
        if time < dawn.start || time >= dusk.end {
            Self::Night
        } else if time >= dawn.end && time < dusk.start {
            Self::Daytime
        } else {
            Self::Transition
        }
    }

    // Determine which period we are currently in based on solar elevation.
    fn from_elevation(elevation_range: ElevationRange, elevation: Elevation) -> Self {
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

// Between 0 and 1
struct TransitionProgress(f32);

impl AsRef<f32> for TransitionProgress {
    fn as_ref(&self) -> &f32 {
        &self.0
    }
}

impl TransitionProgress {
    // Determine how far through the transition we are based on time offset.
    fn from_time(time_ranges: &TimeRanges, time: TimeOffset) -> Self {
        let TimeRanges { dawn, dusk } = time_ranges;
        let sub = |a: TimeOffset, b: TimeOffset| (a.as_ref() - b.as_ref()) as f32;

        if time < dawn.start || time >= dusk.end {
            Self(0.0)
        } else if time < dawn.end {
            Self(sub(dawn.start, time) / sub(dawn.start, dawn.end))
        } else if time > dusk.start {
            Self(sub(dusk.end, time) / sub(dusk.end, dusk.start))
        } else {
            Self(1.0)
        }
    }

    // Determine how far through the transition we are based on elevation.
    fn from_elevation(elevation_range: ElevationRange, elevation: Elevation) -> Self {
        let ElevationRange { high, low } = elevation_range;
        let sub = |a: Elevation, b: Elevation| (a.as_ref() - b.as_ref()) as f32;

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

fn clamp(alpha: f32) -> f32 {
    if 0.0 > alpha {
        0.0
    } else if alpha < 1.0 {
        alpha
    } else {
        1.0
    }
}

// Between 0 and 1
struct Alpha(f32);

impl AsRef<f32> for Alpha {
    fn as_ref(&self) -> &f32 {
        &self.0
    }
}

// Interpolate color setting structs given alpha.
fn interpolate_color_settings(
    lhs: &ColorSetting,
    rhs: &ColorSetting,
    alpha: Alpha,
) -> ColorSetting {
    // let alpha = clamp(alpha);
    let alpha = alpha.as_ref();

    let temperature: Temperature = (((1.0 - alpha) * *lhs.temperature.as_ref() as f32
        + alpha * *rhs.temperature.as_ref() as f32) as u16)
        .try_into()
        .unwrap_or_else(|_| unreachable!());

    let brightness: Brightness = ((1.0 - alpha) * *lhs.brightness.as_ref()
        + alpha * *rhs.brightness.as_ref())
    .try_into()
    .unwrap_or_else(|_| unreachable!());

    let gamma: Gamma = (0..3)
        .map(|i| (1.0 - alpha) * lhs.gamma.as_ref()[i] + alpha * rhs.gamma.as_ref()[i])
        .collect_tuple::<(f32, f32, f32)>()
        .unwrap_or_else(|| unreachable!())
        .try_into()
        .unwrap_or_else(|_| unreachable!());

    ColorSetting {
        temperature,
        gamma,
        brightness,
    }
}

// Return 1 if color settings have major differences, otherwise 0.
// Used to determine if a fade should be applied in continual mode.
fn is_color_setting_diff_major(lhs: &ColorSetting, rhs: &ColorSetting) -> bool {
    (*lhs.temperature.as_ref() as i16 - *rhs.temperature.as_ref() as i16).abs() > 25
        || (lhs.brightness.as_ref() - rhs.brightness.as_ref()).abs() > 0.1
        || (lhs.gamma.as_ref()[0] - rhs.gamma.as_ref()[0]).abs() > 0.1
        || (lhs.gamma.as_ref()[1] - rhs.gamma.as_ref()[1]).abs() > 0.1
        || (lhs.gamma.as_ref()[2] - rhs.gamma.as_ref()[2]).abs() > 0.1
}

// Reset color setting to default values.
fn color_setting_reset(_color: &mut ColorSetting) {
    // (*color).temperature = 6500 as c_int;
    // (*color).gamma[0 as c_int as usize] = 1.0f64 as c_float;
    // (*color).gamma[1 as c_int as usize] = 1.0f64 as c_float;
    // (*color).gamma[2 as c_int as usize] = 1.0f64 as c_float;
    // (*color).brightness = 1.0f64 as c_float;
    todo!()
}

fn provider_try_start(config: &Config) {
    // let mut r: c_int = 0;
    // r = ((*provider).init).expect("non-null function pointer")(state);
    // if r < 0 as c_int {
    //     eprintln!(
    //         "Initialization of {} failed.",
    //         CStr::from_ptr((*provider).name).to_str().unwrap()
    //     );
    //     return -(1 as c_int);
    // }
    // // Set provider options from config file.
    // let section: *mut config_ini_section_t = config_ini_get_section(config, (*provider).name);
    // if !section.is_null() {
    //     let mut setting: *mut config_ini_setting_t = (*section).settings;
    //     while !setting.is_null() {
    //         r = ((*provider).set_option).expect("non-null function pointer")(
    //             *state,
    //             (*setting).name,
    //             (*setting).value,
    //         );
    //         if r < 0 as c_int {
    //             ((*provider).free).expect("non-null function pointer")(*state);
    //             eprintln!(
    //                 "Failed to set {} option.",
    //                 CStr::from_ptr((*provider).name).to_str().unwrap()
    //             );
    //             // TRANSLATORS: `help' must not be
    //             // translated.
    //             eprintln!(
    //                 "Try `-l {}:help' for more information.",
    //                 CStr::from_ptr((*provider).name).to_str().unwrap()
    //             );
    //             return -(1 as c_int);
    //         }
    //         setting = (*setting).next;
    //     }
    // }

    // // Set provider options from command line.
    // let manual_keys: [*const c_char; 2] = [
    //     b"lat\0" as *const u8 as *const c_char,
    //     b"lon\0" as *const u8 as *const c_char,
    // ];
    // let mut i: c_int = 0 as c_int;
    // while !args.is_null() {
    //     let mut next_arg: *mut c_char = strchr(args, ':' as i32);
    //     if !next_arg.is_null() {
    //         let fresh0 = next_arg;
    //         next_arg = next_arg.offset(1);
    //         *fresh0 = '\0' as i32 as c_char;
    //     }
    //     let mut key: *const c_char = args;
    //     let mut value: *mut c_char = strchr(args, '=' as i32);
    //     if value.is_null() {
    //         // The options for the "manual" method can be set
    //         //    without keys on the command line for convencience
    //         //    and for backwards compatibility. We add the proper
    //         //    keys here before calling set_option().
    //         if strcmp((*provider).name, b"manual\0" as *const u8 as *const c_char) == 0 as c_int
    //             && (i as c_ulong)
    //                 < (::core::mem::size_of::<[*const c_char; 2]>() as c_ulong)
    //                     .wrapping_div(::core::mem::size_of::<*const c_char>() as c_ulong)
    //         {
    //             key = manual_keys[i as usize];
    //             value = args;
    //         } else {
    //             eprintln!(
    //                 "Failed to parse option `{}`.",
    //                 CStr::from_ptr(args).to_str().unwrap()
    //             );
    //             return -(1 as c_int);
    //         }
    //     } else {
    //         let fresh1 = value;
    //         value = value.offset(1);
    //         *fresh1 = '\0' as i32 as c_char;
    //     }

    //     r = ((*provider).set_option).expect("non-null function pointer")(*state, key, value);
    //     if r < 0 as c_int {
    //         ((*provider).free).expect("non-null function pointer")(*state);
    //         eprintln!(
    //             "Failed to set {} option.",
    //             CStr::from_ptr((*provider).name).to_str().unwrap()
    //         );
    //         // TRANSLATORS: `help' must not be translated.
    //         eprintln!(
    //             "Try `-l {}:help' for more information.",
    //             CStr::from_ptr((*provider).name).to_str().unwrap()
    //         );
    //         return -(1 as c_int);
    //     }
    //     args = next_arg;
    //     i += 1 as c_int;
    // }

    // Start provider.
    Manual::start();
    // r = ((*provider).start).expect("non-null function pointer")(*state);
    // if r < 0 as c_int {
    //     ((*provider).free).expect("non-null function pointer")(*state);
    //     eprintln!(
    //         "Failed to start provider {}.",
    //         CStr::from_ptr((*provider).name).to_str().unwrap()
    //     );
    //     return -(1 as c_int);
    // }
    // 0 as c_int
}

fn method_try_start(config: &Config) {
    // let mut r: c_int = 0;
    // r = ((*method).init).expect("non-null function pointer")(state);
    // if r < 0 as c_int {
    //     eprintln!(
    //         "Initialization of {} failed",
    //         CStr::from_ptr((*method).name).to_str().unwrap()
    //     );
    //     return -(1 as c_int);
    // }

    // // Set method options from config file.
    // let section: *mut config_ini_section_t = config_ini_get_section(config, (*method).name);
    // if !section.is_null() {
    //     let mut setting: *mut config_ini_setting_t = (*section).settings;
    //     while !setting.is_null() {
    //         r = ((*method).set_option).expect("non-null function pointer")(
    //             *state,
    //             (*setting).name,
    //             (*setting).value,
    //         );
    //         if r < 0 as c_int {
    //             ((*method).free).expect("non-null function pointer")(*state);
    //             eprintln!(
    //                 "Failed to set {} option.",
    //                 CStr::from_ptr((*method).name).to_str().unwrap()
    //             );
    //             // TRANSLATORS: `help' must not be
    //             // translated.
    //             eprintln!(
    //                 "Try `-m {}:help' for more information.",
    //                 CStr::from_ptr((*method).name).to_str().unwrap()
    //             );
    //             return -(1 as c_int);
    //         }
    //         setting = (*setting).next;
    //     }
    // }

    // Set method options from command line.
    // while !args.is_null() {
    //     let mut next_arg: *mut c_char = strchr(args, ':' as i32);
    //     if !next_arg.is_null() {
    //         let fresh2 = next_arg;
    //         next_arg = next_arg.offset(1);
    //         *fresh2 = '\0' as i32 as c_char;
    //     }
    //     let key: *const c_char = args;
    //     let mut value: *mut c_char = strchr(args, '=' as i32);
    //     if value.is_null() {
    //         eprintln!(
    //             "Failed to parse option `{}`.",
    //             CStr::from_ptr(args).to_str().unwrap()
    //         );
    //         return -(1 as c_int);
    //     } else {
    //         let fresh3 = value;
    //         value = value.offset(1);
    //         *fresh3 = '\0' as i32 as c_char;
    //     }
    //     r = ((*method).set_option).expect("non-null function pointer")(*state, key, value);
    //     if r < 0 as c_int {
    //         ((*method).free).expect("non-null function pointer")(*state);
    //         eprintln!(
    //             "Failed to set {} option.",
    //             CStr::from_ptr((*method).name).to_str().unwrap()
    //         );
    //         eprintln!(
    //             "Try `-m {}:help' for more information.",
    //             CStr::from_ptr((*method).name).to_str().unwrap()
    //         );
    //         return -(1 as c_int);
    //     }
    //     args = next_arg;
    // }

    // Start method.
    Dummy::start();
    // r = ((*method).start).expect("non-null function pointer")(*state);
    // if r < 0 as c_int {
    //     ((*method).free).expect("non-null function pointer")(*state);
    //     eprintln!(
    //         "Failed to start adjustment method {}.",
    //         CStr::from_ptr((*method).name).to_str().unwrap()
    //     );
    //     return -(1 as c_int);
    // }
    // 0 as c_int
}

// // Wait for location to become available from provider.
// // Waits until timeout (milliseconds) has elapsed or forever if timeout
// // is -1. Writes location to loc. Returns -1 on error,
// // 0 if timeout was reached, 1 if location became available.
// unsafe extern "C" fn provider_get_location(
//     provider: *const location_provider_t,
//     state: *mut location_state_t,
//     mut timeout: c_int,
//     loc: *mut location_t,
// ) -> c_int {
//     let mut available: c_int = 0 as c_int;
//     let mut pollfds: [pollfd; 1] = [pollfd {
//         fd: 0,
//         events: 0,
//         revents: 0,
//     }; 1];
//     while available == 0 {
//         let loc_fd: c_int = ((*provider).get_fd).expect("non-null function pointer")(state);
//         if loc_fd >= 0 as c_int {
//             // Provider is dynamic.
//             // TODO: This should use a monotonic time source.
//             let mut now: c_double = 0.;
//             let mut r: c_int = systemtime_get_time(&mut now);
//             if r < 0 as c_int {
//                 eprintln!("Unable to read system time.");
//                 return -(1 as c_int);
//             }

//             // Poll on file descriptor until ready.
//             pollfds[0 as c_int as usize].fd = loc_fd;
//             pollfds[0 as c_int as usize].events = 0x1 as c_int as c_short;
//             r = poll(pollfds.as_mut_ptr(), 1 as c_int as Nfds, timeout);
//             if r < 0 as c_int {
//                 perror(b"poll\0" as *const u8 as *const c_char);
//                 return -(1 as c_int);
//             } else if r == 0 as c_int {
//                 return 0 as c_int;
//             }
//             let mut later: c_double = 0.;
//             r = systemtime_get_time(&mut later);
//             if r < 0 as c_int {
//                 eprintln!("Unable to read system time.");
//                 return -(1 as c_int);
//             }

//             // Adjust timeout by elapsed time
//             if timeout >= 0 as c_int {
//                 timeout =
//                     (timeout as c_double - (later - now) * 1000 as c_int as c_double) as c_int;
//                 timeout = if timeout < 0 as c_int {
//                     0 as c_int
//                 } else {
//                     timeout
//                 };
//             }
//         }
//         let r_0: c_int =
//             ((*provider).handle).expect("non-null function pointer")(state, loc, &mut available);
//         if r_0 < 0 as c_int {
//             return -(1 as c_int);
//         }
//     }
//     1 as c_int
// }

// Easing function for fade.
// See https://github.com/mietek/ease-tween
fn ease_fade(t: f64) -> f64 {
    if t <= 0.0 {
        0.0
    } else if t >= 1.0 {
        1.0
    } else {
        1.0042954579734844 * (-6.404173895841566 * (-7.290824133098134 * t).exp()).exp()
    }
}

// // Run continual mode loop
// // This is the main loop of the continual mode which keeps track of the
// // current time and continuously updates the screen to the appropriate
// // color temperature.
// unsafe extern "C" fn run_continual_mode(
//     provider: *const location_provider_t,
//     location_state: *mut location_state_t,
//     scheme: *const transition_scheme_t,
//     method: *const gamma_method_t,
//     method_state: *mut gamma_state_t,
//     use_fade: c_int,
//     preserve_gamma: c_int,
//     verbose: c_int,
// ) -> c_int {
//     let mut r: c_int = 0;

//     // Short fade parameters
//     let mut fade_length: c_int = 0 as c_int;
//     let mut fade_time: c_int = 0 as c_int;
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
//     let mut prev_period: period_t = PERIOD_NONE;

//     // Previous target color setting and current actual color setting.
//     // Actual color setting takes into account the current color fade.
//     let mut prev_target_interp: ColorSetting = ColorSetting {
//         temperature: 0,
//         gamma: [0.; 3],
//         brightness: 0.,
//     };
//     color_setting_reset(&mut prev_target_interp);
//     let mut interp: ColorSetting = ColorSetting {
//         temperature: 0,
//         gamma: [0.; 3],
//         brightness: 0.,
//     };
//     color_setting_reset(&mut interp);
//     let mut loc: location_t = {
//         location_t {
//             lat: ::core::f32::NAN,
//             lon: ::core::f32::NAN,
//         }
//     };

//     let need_location: c_int = ((*scheme).use_time == 0) as c_int;
//     if need_location != 0 {
//         eprintln!("Waiting for initial location to become available...");

//         // Get initial location from provider
//         r = provider_get_location(provider, location_state, -(1 as c_int), &mut loc);
//         if r < 0 as c_int {
//             eprintln!("Unable to get location from provider.");
//             return -(1 as c_int);
//         }
//         print_location(&mut loc);
//     }

//     if verbose != 0 {
//         printf(
//             // gettext(
//             b"Color temperature: %uK\n\0" as *const u8 as *const c_char,
//             interp.temperature,
//         );
//         printf(
//             // gettext(
//             b"Brightness: %.2f\n\0" as *const u8 as *const c_char,
//             interp.brightness as c_double,
//         );
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
//         if verbose != 0 && disabled != prev_disabled {
//             printf(
//                 // gettext(
//                 b"Status: %s\n\0" as *const u8 as *const c_char,
//                 if disabled != 0 {
//                     // gettext(
//                     b"Disabled\0" as *const u8 as *const c_char
//                 } else {
//                     // gettext(
//                     b"Enabled\0" as *const u8 as *const c_char
//                 },
//             );
//         }
//         prev_disabled = disabled;

//         // Read timestamp
//         let mut now: c_double = 0.;
//         r = systemtime_get_time(&mut now);
//         if r < 0 as c_int {
//             eprintln!("Unable to read system time.");
//             return -(1 as c_int);
//         }

//         let mut period: period_t = PERIOD_NONE;
//         let mut transition_prog: c_double = 0.;
//         if (*scheme).use_time != 0 {
//             let time_offset: c_int = get_seconds_since_midnight(now);
//             period = get_period_from_time(scheme, time_offset);
//             transition_prog = get_transition_progress_from_time(scheme, time_offset);
//         } else {
//             // Current angular elevation of the sun
//             let elevation: c_double =
//                 solar_elevation(now, loc.lat as c_double, loc.lon as c_double);
//             period = get_period_from_elevation(scheme, elevation);
//             transition_prog = get_transition_progress_from_elevation(scheme, elevation);
//         }

//         // Use transition progress to get target color
//         //    temperature.
//         let mut target_interp: ColorSetting = ColorSetting {
//             temperature: 0,
//             gamma: [0.; 3],
//             brightness: 0.,
//         };
//         interpolate_transition_scheme(scheme, transition_prog, &mut target_interp);
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
//         if verbose != 0
//             && (period as c_uint != prev_period as c_uint
//                 || period as c_uint == PERIOD_TRANSITION as c_int as c_uint)
//         {
//             print_period(period, transition_prog);
//         }

//         /* Activate hooks if period changed */
//         if period as c_uint != prev_period as c_uint {
//             hooks_signal_period_change(prev_period, period);
//         }

//         // Start fade if the parameter differences are too big to apply
//         // instantly.
//         if use_fade != 0
//             && (fade_length == 0 as c_int
//                 && color_setting_diff_is_major(&mut interp, &mut target_interp) != 0
//                 || fade_length != 0 as c_int
//                     && color_setting_diff_is_major(&mut target_interp, &mut prev_target_interp)
//                         != 0)
//         {
//             fade_length = 40 as c_int;
//             fade_time = 0 as c_int;
//             fade_start_interp = interp;
//         }

//         // Handle ongoing fade
//         if fade_length != 0 as c_int {
//             fade_time += 1 as c_int;
//             let frac: c_double = fade_time as c_double / fade_length as c_double;
//             let alpha: c_double = if 0.0f64 > ease_fade(frac) {
//                 0.0f64
//             } else if ease_fade(frac) < 1.0f64 {
//                 ease_fade(frac)
//             } else {
//                 1.0f64
//             };
//             interpolate_color_settings(
//                 &mut fade_start_interp,
//                 &mut target_interp,
//                 alpha,
//                 &mut interp,
//             );
//             if fade_time > fade_length {
//                 fade_time = 0 as c_int;
//                 fade_length = 0 as c_int;
//             }
//         } else {
//             interp = target_interp;
//         }

//         // Break loop when done and final fade is over
//         if done != 0 && fade_length == 0 as c_int {
//             break;
//         }

//         if verbose != 0 {
//             if prev_target_interp.temperature != target_interp.temperature {
//                 printf(
//                     // gettext(
//                     b"Color temperature: %uK\n\0" as *const u8 as *const c_char,
//                     target_interp.temperature,
//                 );
//             }
//             if prev_target_interp.brightness != target_interp.brightness {
//                 printf(
//                     // gettext(
//                     b"Brightness: %.2f\n\0" as *const u8 as *const c_char,
//                     target_interp.brightness as c_double,
//                 );
//             }
//         }

//         // Adjust temperature
//         r = ((*method).set_temperature).expect("non-null function pointer")(
//             method_state,
//             &mut interp,
//             preserve_gamma,
//         );
//         if r < 0 as c_int {
//             eprintln!("Temperature adjustment failed.");
//             return -(1 as c_int);
//         }

//         // Save period and target color setting as previous
//         prev_period = period;
//         prev_target_interp = target_interp;

//         // Sleep length depends on whether a fade is ongoing.
//         let mut delay: c_int = 5000 as c_int;
//         if fade_length != 0 as c_int {
//             delay = 100 as c_int;
//         }

//         // Update location.
//         let mut loc_fd: c_int = -(1 as c_int);
//         if need_location != 0 {
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

fn main() -> Result<()> {
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

    // Initialize location provider if needed. If provider is NULL
    // try all providers until one that works is found.
    // let mut location_state: *mut location_state_t = std::ptr::null_mut::<location_state_t>();

    let config = Config::new()?;

    let need_location = match (config.mode, config.transition_scheme.select) {
        (Mode::Reset | Mode::Manual, _) => false,
        (Mode::Continual | Mode::OneShot | Mode::Print, TransitionSchemeKind::Elevation) => true,
        (Mode::Continual | Mode::OneShot | Mode::Print, TransitionSchemeKind::TimeRanges) => false,
    };

    if need_location {
        if !(options.provider).is_null() {
            // Use provider specified on command line.
        } else {
            // Try all providers, use the first that works.
            // b"Trying location provider `%s'...\n\0" as *const u8 as *const c_char,
            //     b"Trying next provider...\n\0" as *const u8 as *const c_char,
            //     b"Using provider `%s'.\n\0" as *const u8 as *const c_char,
            // b"No more location providers to try.\n\0" as *const u8 as *const c_char,
            todo!()
        }

        if config.verbose {
            // TRANSLATORS: Append degree symbols if possible.
            // b"Solar elevations: day above %.1f, night below %.1f\n\0" as *const u8
            todo!()
        }
    }

    if config.mode as c_uint != PROGRAM_MODE_RESET as c_int as c_uint
        && config.mode as c_uint != PROGRAM_MODE_MANUAL as c_int as c_uint
    {
        if config.verbose {
            todo!()
            // b"Temperatures: %dK at day, %dK at night\n\0" as *const u8 as *const c_char,
        }
    }

    if options.verbose != 0 {
        printf(
            // gettext(
            b"Brightness: %.2f:%.2f\n\0" as *const u8 as *const c_char,
            options.scheme.day.brightness as c_double,
            options.scheme.night.brightness as c_double,
        );
    }

    // Gamma
    if gamma_is_valid((options.scheme.day.gamma).as_mut_ptr() as *const c_float) == 0
        || gamma_is_valid((options.scheme.night.gamma).as_mut_ptr() as *const c_float) == 0
    {
        // TRANSLATORS: The string in parenthesis is either
        // Daytime or Night (translated).
        fprintf(
            stderr,
            // gettext(
            b"Gamma value must be between %.1f and %.1f.\n\0" as *const u8 as *const c_char,
            // ),
            0.1f64,
            10.0f64,
        );
        exit(1 as c_int);
    }

    if options.verbose != 0 {
        printf(
            // gettext(
            b"Gamma (%s): %.3f, %.3f, %.3f\n\0" as *const u8 as *const c_char,
            // gettext(
            b"Daytime\0" as *const u8 as *const c_char,
            options.scheme.day.gamma[0 as c_int as usize] as c_double,
            options.scheme.day.gamma[1 as c_int as usize] as c_double,
            options.scheme.day.gamma[2 as c_int as usize] as c_double,
        );
        printf(
            // gettext(
            b"Gamma (%s): %.3f, %.3f, %.3f\n\0" as *const u8 as *const c_char,
            // gettext(
            b"Night\0" as *const u8 as *const c_char,
            options.scheme.night.gamma[0 as c_int as usize] as c_double,
            options.scheme.night.gamma[1 as c_int as usize] as c_double,
            options.scheme.night.gamma[2 as c_int as usize] as c_double,
        );
    }

    let scheme: *mut transition_scheme_t = &mut options.scheme;

    // Initialize gamma adjustment method. If method is NULL
    // try all methods until one that works is found.
    let mut method_state: *mut gamma_state_t = std::ptr::null_mut::<gamma_state_t>();

    // Gamma adjustment not needed for print mode
    if options.mode as c_uint != PROGRAM_MODE_PRINT as c_int as c_uint {
        if !(options.method).is_null() {
            // Use method specified on command line.
            r = method_try_start(
                options.method,
                &mut method_state,
                &mut config_state,
                options.method_args,
            );
            if r < 0 as c_int {
                exit(1 as c_int);
            }
        } else {
            // Try all methods, use the first that works.
            let mut i_0: c_int = 0 as c_int;
            while !(gamma_methods[i_0 as usize].name).is_null() {
                let m: *const gamma_method_t =
                    &*gamma_methods.as_ptr().offset(i_0 as isize) as *const gamma_method_t;
                if (*m).autostart != 0 {
                    r = method_try_start(
                        m,
                        &mut method_state,
                        &mut config_state,
                        std::ptr::null_mut::<c_char>(),
                    );
                    if r < 0 as c_int {
                        fputs(
                            // gettext(
                            b"Trying next method...\n\0" as *const u8 as *const c_char,
                            stderr,
                        );
                    } else {
                        // Found method that works.
                        printf(
                            // gettext(
                            b"Using method `%s'.\n\0" as *const u8 as *const c_char,
                            (*m).name,
                        );
                        options.method = m;
                        break;
                    }
                }
                i_0 += 1;
                i_0;
            }

            // Failure if no methods were successful at this point.
            if (options.method).is_null() {
                fputs(
                    // gettext(
                    b"No more methods to try.\n\0" as *const u8 as *const c_char,
                    stderr,
                );
                exit(1 as c_int);
            }
        }
    }

    match options.mode as c_uint {
        1 | 2 => {
            let mut loc: location_t = {
                location_t {
                    lat: ::core::f32::NAN,
                    lon: ::core::f32::NAN,
                }
            };
            if need_location != 0 {
                fputs(
                    // gettext(
                    b"Waiting for current location to become available...\n\0" as *const u8
                        as *const c_char,
                    // ),
                    stderr,
                );

                // Wait for location provider.
                let r_0: c_int = provider_get_location(
                    options.provider,
                    location_state,
                    -(1 as c_int),
                    &mut loc,
                );
                if r_0 < 0 as c_int {
                    fputs(
                        // gettext(
                        b"Unable to get location from provider.\n\0" as *const u8 as *const c_char,
                        // ),
                        stderr,
                    );
                    exit(1 as c_int);
                }
                print_location(&mut loc);
            }
            let mut now: c_double = 0.;
            r = systemtime_get_time(&mut now);
            if r < 0 as c_int {
                fputs(
                    // gettext(
                    b"Unable to read system time.\n\0" as *const u8 as *const c_char,
                    stderr,
                );
                ((*options.method).free).expect("non-null function pointer")(method_state);
                exit(1 as c_int);
            }
            let mut period: period_t = PERIOD_NONE;
            let mut transition_prog: c_double = 0.;
            if options.scheme.use_time != 0 {
                let time_offset: c_int = get_seconds_since_midnight(now);
                period = get_period_from_time(scheme, time_offset);
                transition_prog = get_transition_progress_from_time(scheme, time_offset);
            } else {
                // Current angular elevation of the sun
                let elevation: c_double =
                    solar_elevation(now, loc.lat as c_double, loc.lon as c_double);
                if options.verbose != 0 {
                    // TRANSLATORS: Append degree symbol if
                    // possible.
                    printf(
                        // gettext(
                        b"Solar elevation: %f\n\0" as *const u8 as *const c_char,
                        elevation,
                    );
                }
                period = get_period_from_elevation(scheme, elevation);
                transition_prog = get_transition_progress_from_elevation(scheme, elevation);
            }

            // Use transition progress to set color temperature
            let mut interp: ColorSetting = ColorSetting {
                temperature: 0,
                gamma: [0.; 3],
                brightness: 0.,
            };
            interpolate_transition_scheme(scheme, transition_prog, &mut interp);
            if options.verbose != 0
                || options.mode as c_uint == PROGRAM_MODE_PRINT as c_int as c_uint
            {
                print_period(period, transition_prog);
                printf(
                    // gettext(
                    b"Color temperature: %uK\n\0" as *const u8 as *const c_char,
                    interp.temperature,
                );
                printf(
                    // gettext(
                    b"Brightness: %.2f\n\0" as *const u8 as *const c_char,
                    interp.brightness as c_double,
                );
            }
            if options.mode as c_uint != PROGRAM_MODE_PRINT as c_int as c_uint {
                // Adjust temperature
                r = ((*options.method).set_temperature).expect("non-null function pointer")(
                    method_state,
                    &mut interp,
                    options.preserve_gamma,
                );
                if r < 0 as c_int {
                    fputs(
                        // gettext(
                        b"Temperature adjustment failed.\n\0" as *const u8 as *const c_char,
                        // ),
                        stderr,
                    );
                    ((*options.method).free).expect("non-null function pointer")(method_state);
                    exit(1 as c_int);
                }

                // In Quartz (macOS) the gamma adjustments will
                // automatically revert when the process exits.
                // Therefore, we have to loop until CTRL-C is received.
                if strcmp(
                    (*options.method).name,
                    b"quartz\0" as *const u8 as *const c_char,
                ) == 0 as c_int
                {
                    fputs(
                        // gettext(
                        b"Press ctrl-c to stop...\n\0" as *const u8 as *const c_char,
                        stderr,
                    );
                    pause();
                }
            }
        }

        4 => {
            if options.verbose != 0 {
                printf(
                    // gettext(
                    b"Color temperature: %uK\n\0" as *const u8 as *const c_char,
                    options.temp_set,
                );
            }

            // Adjust temperature
            let mut manual: ColorSetting = (*scheme).day;
            manual.temperature = options.temp_set;
            r = ((*options.method).set_temperature).expect("non-null function pointer")(
                method_state,
                &mut manual,
                options.preserve_gamma,
            );
            if r < 0 as c_int {
                fputs(
                    // gettext(
                    b"Temperature adjustment failed.\n\0" as *const u8 as *const c_char,
                    stderr,
                );
                ((*options.method).free).expect("non-null function pointer")(method_state);
                exit(1 as c_int);
            }

            // In Quartz (OSX) the gamma adjustments will automatically
            //    revert when the process exits. Therefore, we have to loop
            //    until CTRL-C is received.
            if strcmp(
                (*options.method).name,
                b"quartz\0" as *const u8 as *const c_char,
            ) == 0 as c_int
            {
                fputs(
                    // gettext(
                    b"Press ctrl-c to stop...\n\0" as *const u8 as *const c_char,
                    stderr,
                );
                pause();
            }
        }

        3 => {
            // Reset screen
            let mut reset: ColorSetting = ColorSetting {
                temperature: 0,
                gamma: [0.; 3],
                brightness: 0.,
            };
            color_setting_reset(&mut reset);
            r = ((*options.method).set_temperature).expect("non-null function pointer")(
                method_state,
                &mut reset,
                0 as c_int,
            );
            if r < 0 as c_int {
                fputs(
                    // gettext(
                    b"Temperature adjustment failed.\n\0" as *const u8 as *const c_char,
                    stderr,
                );
                ((*options.method).free).expect("non-null function pointer")(method_state);
                exit(1 as c_int);
            }

            // In Quartz (OSX) the gamma adjustments will automatically
            // revert when the process exits. Therefore, we have to loop
            // until CTRL-C is received.
            if strcmp(
                (*options.method).name,
                b"quartz\0" as *const u8 as *const c_char,
            ) == 0 as c_int
            {
                fputs(
                    // gettext(
                    b"Press ctrl-c to stop...\n\0" as *const u8 as *const c_char,
                    stderr,
                );
                pause();
            }
        }
        0 => {
            r = run_continual_mode(
                options.provider,
                location_state,
                scheme,
                options.method,
                method_state,
                options.use_fade,
                options.preserve_gamma,
                options.verbose,
            );
            if r < 0 as c_int {
                exit(1 as c_int);
            }
        }
        _ => {}
    }

    // Clean up gamma adjustment state
    if options.mode as c_uint != PROGRAM_MODE_PRINT as c_int as c_uint {
        ((*options.method).free).expect("non-null function pointer")(method_state);
    }

    // Clean up location provider state
    if need_location != 0 {
        ((*options.provider).free).expect("non-null function pointer")(location_state);
    }
}
