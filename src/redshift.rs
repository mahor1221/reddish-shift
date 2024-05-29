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
pub mod options;
pub mod pipeutils;
pub mod signals;
pub mod solar;
pub mod systemtime;

use config::Config;
use gamma_drm::drm_gamma_method;
use gamma_randr::randr_gamma_method;
use gamma_vidmode::dummy_gamma_method;
use gamma_vidmode::vidmode_gamma_method;
use hooks::hooks_signal_period_change;
use libc::{
    __errno_location, exit, fprintf, fputs, free, localtime_r, pause, perror, poll, pollfd, printf,
    setlocale, setvbuf, strchr, strcmp, time_t, tm, FILE,
};
use location_geoclue2::geoclue2_location_provider;
use location_manual::manual_location_provider;
use options::{
    options_init, options_parse_args, options_parse_config_file, options_set_defaults, options_t,
};
use signals::{disable, exiting, signals_install_handlers};
use solar::solar_elevation;
use std::{
    ffi::{c_char, c_double, c_float, c_int, c_long, c_short, c_uint, c_ulong, c_void, CStr},
    ptr::addr_of_mut,
};
use systemtime::{systemtime_get_time, systemtime_msleep};

pub type size_t = c_ulong;
pub type __off_t = c_long;
pub type __off64_t = c_long;
pub type __time_t = c_long;
pub type __sig_atomic_t = c_int;

pub type nfds_t = c_ulong;
pub type sig_atomic_t = __sig_atomic_t;

extern "C" {
    pub static stdout: *mut libc::FILE;
    pub static stderr: *mut libc::FILE;
}

// TODO: replace magic numbers with the const values below:

// Bounds for parameters.
pub const MIN_LAT: f64 = -90.0;
pub const MAX_LAT: f64 = 90.0;
pub const MIN_LON: f64 = -180.0;
pub const MAX_LON: f64 = 180.0;
pub const MIN_TEMP: u32 = 1000;
pub const MAX_TEMP: u32 = 25000;
pub const MIN_BRIGHTNESS: f64 = 0.1;
pub const MAX_BRIGHTNESS: f64 = 1.0;
pub const MIN_GAMMA: f64 = 0.1;
pub const MAX_GAMMA: f64 = 10.0;

// Duration of sleep between screen updates (milliseconds).
const SLEEP_DURATION: u32 = 5000;
const SLEEP_DURATION_SHORT: u32 = 100;

// Length of fade in numbers of short sleep durations.
const FADE_LENGTH: u32 = 40;

// Location
#[derive(Copy, Clone)]
#[repr(C)]
pub struct location_t {
    pub lat: c_float,
    pub lon: c_float,
}

// Names of periods supplied to scripts.
pub static mut period_names: [*const c_char; 4] = [
    // TRANSLATORS: Name printed when period of day is unknown
    b"None\0" as *const u8 as *const c_char,
    b"Daytime\0" as *const u8 as *const c_char,
    b"Night\0" as *const u8 as *const c_char,
    b"Transition\0" as *const u8 as *const c_char,
];

// Periods of day.
pub type period_t = c_uint;
pub const PERIOD_TRANSITION: period_t = 3;
pub const PERIOD_NIGHT: period_t = 2;
pub const PERIOD_DAYTIME: period_t = 1;
pub const PERIOD_NONE: period_t = 0;

// Color setting
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ColorSetting {
    // TODO: u32
    pub temperature: i32,
    pub gamma: [f32; 3],
    pub brightness: f32,
}

// Program modes.
pub type program_mode_t = c_uint;
pub const PROGRAM_MODE_MANUAL: program_mode_t = 4;
pub const PROGRAM_MODE_RESET: program_mode_t = 3;
pub const PROGRAM_MODE_PRINT: program_mode_t = 2;
pub const PROGRAM_MODE_ONE_SHOT: program_mode_t = 1;
pub const PROGRAM_MODE_CONTINUAL: program_mode_t = 0;

// Time range.
// Fields are offsets from midnight in seconds.
#[derive(Copy, Clone)]
#[repr(C)]
pub struct time_range_t {
    pub start: c_int,
    pub end: c_int,
}

// Transition scheme.
// The solar elevations at which the transition begins/ends,
// and the association color settings.
#[derive(Copy, Clone)]
#[repr(C)]
pub struct transition_scheme_t {
    pub high: c_double,
    pub low: c_double,
    pub use_time: c_int, // When enabled, ignore elevation and use time ranges.
    pub dawn: time_range_t,
    pub dusk: time_range_t,
    pub day: ColorSetting,
    pub night: ColorSetting,
}

// Gamma adjustment method
pub struct gamma_state_t;
pub type gamma_method_init_func = unsafe extern "C" fn(*mut *mut gamma_state_t) -> c_int;
pub type gamma_method_start_func = unsafe extern "C" fn(*mut gamma_state_t) -> c_int;
pub type gamma_method_free_func = unsafe extern "C" fn(*mut gamma_state_t) -> ();
pub type gamma_method_print_help_func = unsafe extern "C" fn(*mut FILE) -> ();
pub type gamma_method_set_option_func =
    unsafe extern "C" fn(*mut gamma_state_t, *const c_char, *const c_char) -> c_int;
pub type gamma_method_restore_func = unsafe extern "C" fn(*mut gamma_state_t) -> ();
pub type gamma_method_set_temperature_func =
    unsafe extern "C" fn(*mut gamma_state_t, *const ColorSetting, c_int) -> c_int;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct gamma_method_t {
    pub name: *mut c_char,

    // If true, this method will be tried if none is explicitly chosen.
    pub autostart: c_int,

    // Initialize state. Options can be set between init and start.
    pub init: Option<gamma_method_init_func>,
    // Allocate storage and make connections that depend on options.
    pub start: Option<gamma_method_start_func>,
    // Free all allocated storage and close connections.
    pub free: Option<gamma_method_free_func>,

    // Print help on options for this adjustment method.
    pub print_help: Option<gamma_method_print_help_func>,
    // Set an option key, value-pair
    pub set_option: Option<gamma_method_set_option_func>,

    // Restore the adjustment to the state before start was called.
    pub restore: Option<gamma_method_restore_func>,
    // Set a specific color temperature.
    pub set_temperature: Option<gamma_method_set_temperature_func>,
}

// Location provider
pub struct location_state_t;
pub type location_provider_init_func = unsafe extern "C" fn(*mut *mut location_state_t) -> c_int;
pub type location_provider_start_func = unsafe extern "C" fn(*mut location_state_t) -> c_int;
pub type location_provider_free_func = unsafe extern "C" fn(*mut location_state_t) -> ();
pub type location_provider_print_help_func = unsafe extern "C" fn(*mut FILE) -> ();
pub type location_provider_set_option_func =
    unsafe extern "C" fn(*mut location_state_t, *const c_char, *const c_char) -> c_int;
pub type location_provider_get_fd_func = unsafe extern "C" fn(*mut location_state_t) -> c_int;
pub type location_provider_handle_func =
    unsafe extern "C" fn(*mut location_state_t, *mut location_t, *mut c_int) -> c_int;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct location_provider_t {
    pub name: *mut c_char,

    // Initialize state. Options can be set between init and start.
    pub init: Option<location_provider_init_func>,
    // Allocate storage and make connections that depend on options.
    pub start: Option<location_provider_start_func>,
    // Free all allocated storage and close connections.
    pub free: Option<location_provider_free_func>,

    // Print help on options for this location provider.
    pub print_help: Option<location_provider_print_help_func>,
    // Set an option key, value-pair.
    pub set_option: Option<location_provider_set_option_func>,

    // Listen and handle location updates.
    pub get_fd: Option<location_provider_get_fd_func>,
    pub handle: Option<location_provider_handle_func>,
}

// Determine which period we are currently in based on time offset.
unsafe extern "C" fn get_period_from_time(
    transition: *const transition_scheme_t,
    time_offset: c_int,
) -> period_t {
    if time_offset < (*transition).dawn.start || time_offset >= (*transition).dusk.end {
        PERIOD_NIGHT
    } else if time_offset >= (*transition).dawn.end && time_offset < (*transition).dusk.start {
        return PERIOD_DAYTIME;
    } else {
        return PERIOD_TRANSITION;
    }
}

// Determine which period we are currently in based on solar elevation.
unsafe extern "C" fn get_period_from_elevation(
    transition: *const transition_scheme_t,
    elevation: c_double,
) -> period_t {
    if elevation < (*transition).low {
        PERIOD_NIGHT
    } else if elevation < (*transition).high {
        return PERIOD_TRANSITION;
    } else {
        return PERIOD_DAYTIME;
    }
}

// Determine how far through the transition we are based on time offset.
unsafe extern "C" fn get_transition_progress_from_time(
    transition: *const transition_scheme_t,
    time_offset: c_int,
) -> c_double {
    if time_offset < (*transition).dawn.start || time_offset >= (*transition).dusk.end {
        0.0f64
    } else if time_offset < (*transition).dawn.end {
        return ((*transition).dawn.start - time_offset) as c_double
            / ((*transition).dawn.start - (*transition).dawn.end) as c_double;
    } else if time_offset > (*transition).dusk.start {
        return ((*transition).dusk.end - time_offset) as c_double
            / ((*transition).dusk.end - (*transition).dusk.start) as c_double;
    } else {
        return 1.0f64;
    }
}

// Determine how far through the transition we are based on elevation.
unsafe extern "C" fn get_transition_progress_from_elevation(
    transition: *const transition_scheme_t,
    elevation: c_double,
) -> c_double {
    if elevation < (*transition).low {
        0.0f64
    } else if elevation < (*transition).high {
        return ((*transition).low - elevation) / ((*transition).low - (*transition).high);
    } else {
        return 1.0f64;
    }
}

// Return number of seconds since midnight from timestamp.
unsafe extern "C" fn get_seconds_since_midnight(timestamp: c_double) -> c_int {
    let mut t: time_t = timestamp as time_t;
    let mut tm: tm = tm {
        tm_sec: 0,
        tm_min: 0,
        tm_hour: 0,
        tm_mday: 0,
        tm_mon: 0,
        tm_year: 0,
        tm_wday: 0,
        tm_yday: 0,
        tm_isdst: 0,
        tm_gmtoff: 0,
        tm_zone: std::ptr::null::<c_char>(),
    };
    localtime_r(&mut t, &mut tm);
    tm.tm_sec + tm.tm_min * 60 as c_int + tm.tm_hour * 3600 as c_int
}

// Print verbose description of the given period.
unsafe extern "C" fn print_period(period: period_t, transition: c_double) {
    match period as c_uint {
        0..=2 => {
            printf(
                // gettext(
                b"Period: %s\n\0" as *const u8 as *const c_char,
                // ),
                // gettext(
                period_names[period as usize],
                // ),
            );
        }
        3 => {
            printf(
                // gettext(
                b"Period: %s (%.2f%% day)\n\0" as *const u8 as *const c_char,
                // ),
                // gettext(
                period_names[period as usize],
                // ),
                transition * 100 as c_int as c_double,
            );
        }
        _ => {}
    };
}

// Print location
unsafe extern "C" fn print_location(location: *const location_t) {
    // gettext(
    // TRANSLATORS: Abbreviation for `north'
    let north: *const c_char = b"N\0" as *const u8 as *const c_char;
    // TRANSLATORS: Abbreviation for `south'
    let south: *const c_char = b"S\0" as *const u8 as *const c_char;
    // TRANSLATORS: Abbreviation for `east'
    let east: *const c_char = b"E\0" as *const u8 as *const c_char;
    // TRANSLATORS: Abbreviation for `west'
    let west: *const c_char = b"W\0" as *const u8 as *const c_char;

    // TRANSLATORS: Append degree symbols after %f if possible.
    // The string following each number is an abbreviation for
    // north, source, east or west (N, S, E, W).
    printf(
        // gettext(
        b"Location: %.2f %s, %.2f %s\n\0" as *const u8 as *const c_char,
        (*location).lat.abs() as c_double,
        if (*location).lat >= 0.0f32 {
            north
        } else {
            south
        },
        (*location).lon.abs() as c_double,
        if (*location).lon >= 0.0f32 {
            east
        } else {
            west
        },
    );
}

// Interpolate color setting structs given alpha.
unsafe extern "C" fn interpolate_color_settings(
    first: *const ColorSetting,
    second: *const ColorSetting,
    mut alpha: c_double,
    result: *mut ColorSetting,
) {
    alpha = if 0.0f64 > alpha {
        0.0f64
    } else if alpha < 1.0f64 {
        alpha
    } else {
        1.0f64
    };
    (*result).temperature = ((1.0f64 - alpha) * (*first).temperature as c_double
        + alpha * (*second).temperature as c_double) as c_int;
    (*result).brightness = ((1.0f64 - alpha) * (*first).brightness as c_double
        + alpha * (*second).brightness as c_double) as c_float;
    let mut i: c_int = 0 as c_int;
    while i < 3 as c_int {
        (*result).gamma[i as usize] = ((1.0f64 - alpha) * (*first).gamma[i as usize] as c_double
            + alpha * (*second).gamma[i as usize] as c_double)
            as c_float;
        i += 1;
        i;
    }
}

// Interpolate color setting structs transition scheme.
unsafe extern "C" fn interpolate_transition_scheme(
    transition: *const transition_scheme_t,
    mut alpha: c_double,
    result: *mut ColorSetting,
) {
    let day: *const ColorSetting = &(*transition).day;
    let night: *const ColorSetting = &(*transition).night;
    alpha = if 0.0f64 > alpha {
        0.0f64
    } else if alpha < 1.0f64 {
        alpha
    } else {
        1.0f64
    };
    interpolate_color_settings(night, day, alpha, result);
}

// Return 1 if color settings have major differences, otherwise 0.
// Used to determine if a fade should be applied in continual mode.
unsafe extern "C" fn color_setting_diff_is_major(
    first: *const ColorSetting,
    second: *const ColorSetting,
) -> c_int {
    (((*first).temperature - (*second).temperature).abs() > 25 as c_int
        || ((*first).brightness - (*second).brightness).abs() as c_double > 0.1f64
        || ((*first).gamma[0 as c_int as usize] - (*second).gamma[0 as c_int as usize]).abs()
            as c_double
            > 0.1f64
        || ((*first).gamma[1 as c_int as usize] - (*second).gamma[1 as c_int as usize]).abs()
            as c_double
            > 0.1f64
        || ((*first).gamma[2 as c_int as usize] - (*second).gamma[2 as c_int as usize]).abs()
            as c_double
            > 0.1f64) as c_int
}

// Reset color setting to default values.
unsafe extern "C" fn color_setting_reset(color: *mut ColorSetting) {
    (*color).temperature = 6500 as c_int;
    (*color).gamma[0 as c_int as usize] = 1.0f64 as c_float;
    (*color).gamma[1 as c_int as usize] = 1.0f64 as c_float;
    (*color).gamma[2 as c_int as usize] = 1.0f64 as c_float;
    (*color).brightness = 1.0f64 as c_float;
}

unsafe extern "C" fn provider_try_start(
    provider: *const location_provider_t,
    state: *mut *mut location_state_t,
    config: &Config,
    mut args: *mut c_char,
) -> c_int {
    let mut r: c_int = 0;
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

    // Set provider options from command line.
    let manual_keys: [*const c_char; 2] = [
        b"lat\0" as *const u8 as *const c_char,
        b"lon\0" as *const u8 as *const c_char,
    ];
    let mut i: c_int = 0 as c_int;
    while !args.is_null() {
        let mut next_arg: *mut c_char = strchr(args, ':' as i32);
        if !next_arg.is_null() {
            let fresh0 = next_arg;
            next_arg = next_arg.offset(1);
            *fresh0 = '\0' as i32 as c_char;
        }
        let mut key: *const c_char = args;
        let mut value: *mut c_char = strchr(args, '=' as i32);
        if value.is_null() {
            // The options for the "manual" method can be set
            //    without keys on the command line for convencience
            //    and for backwards compatibility. We add the proper
            //    keys here before calling set_option().
            if strcmp((*provider).name, b"manual\0" as *const u8 as *const c_char) == 0 as c_int
                && (i as c_ulong)
                    < (::core::mem::size_of::<[*const c_char; 2]>() as c_ulong)
                        .wrapping_div(::core::mem::size_of::<*const c_char>() as c_ulong)
            {
                key = manual_keys[i as usize];
                value = args;
            } else {
                eprintln!(
                    "Failed to parse option `{}`.",
                    CStr::from_ptr(args).to_str().unwrap()
                );
                return -(1 as c_int);
            }
        } else {
            let fresh1 = value;
            value = value.offset(1);
            *fresh1 = '\0' as i32 as c_char;
        }

        r = ((*provider).set_option).expect("non-null function pointer")(*state, key, value);
        if r < 0 as c_int {
            ((*provider).free).expect("non-null function pointer")(*state);
            eprintln!(
                "Failed to set {} option.",
                CStr::from_ptr((*provider).name).to_str().unwrap()
            );
            // TRANSLATORS: `help' must not be translated.
            eprintln!(
                "Try `-l {}:help' for more information.",
                CStr::from_ptr((*provider).name).to_str().unwrap()
            );
            return -(1 as c_int);
        }
        args = next_arg;
        i += 1 as c_int;
    }

    // Start provider.
    r = ((*provider).start).expect("non-null function pointer")(*state);
    if r < 0 as c_int {
        ((*provider).free).expect("non-null function pointer")(*state);
        eprintln!(
            "Failed to start provider {}.",
            CStr::from_ptr((*provider).name).to_str().unwrap()
        );
        return -(1 as c_int);
    }
    0 as c_int
}

unsafe extern "C" fn method_try_start(
    method: *const gamma_method_t,
    state: *mut *mut gamma_state_t,
    config: &Config,
    mut args: *mut c_char,
) -> c_int {
    let mut r: c_int = 0;

    // useless code

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
    while !args.is_null() {
        let mut next_arg: *mut c_char = strchr(args, ':' as i32);
        if !next_arg.is_null() {
            let fresh2 = next_arg;
            next_arg = next_arg.offset(1);
            *fresh2 = '\0' as i32 as c_char;
        }
        let key: *const c_char = args;
        let mut value: *mut c_char = strchr(args, '=' as i32);
        if value.is_null() {
            eprintln!(
                "Failed to parse option `{}`.",
                CStr::from_ptr(args).to_str().unwrap()
            );
            return -(1 as c_int);
        } else {
            let fresh3 = value;
            value = value.offset(1);
            *fresh3 = '\0' as i32 as c_char;
        }
        r = ((*method).set_option).expect("non-null function pointer")(*state, key, value);
        if r < 0 as c_int {
            ((*method).free).expect("non-null function pointer")(*state);
            eprintln!(
                "Failed to set {} option.",
                CStr::from_ptr((*method).name).to_str().unwrap()
            );
            eprintln!(
                "Try `-m {}:help' for more information.",
                CStr::from_ptr((*method).name).to_str().unwrap()
            );
            return -(1 as c_int);
        }
        args = next_arg;
    }

    // Start method.
    r = ((*method).start).expect("non-null function pointer")(*state);
    if r < 0 as c_int {
        ((*method).free).expect("non-null function pointer")(*state);
        eprintln!(
            "Failed to start adjustment method {}.",
            CStr::from_ptr((*method).name).to_str().unwrap()
        );
        return -(1 as c_int);
    }
    0 as c_int
}

// Check whether gamma is within allowed levels.
unsafe extern "C" fn gamma_is_valid(gamma: *const c_float) -> c_int {
    !((*gamma.offset(0 as c_int as isize) as c_double) < 0.1f64
        || *gamma.offset(0 as c_int as isize) as c_double > 10.0f64
        || (*gamma.offset(1 as c_int as isize) as c_double) < 0.1f64
        || *gamma.offset(1 as c_int as isize) as c_double > 10.0f64
        || (*gamma.offset(2 as c_int as isize) as c_double) < 0.1f64
        || *gamma.offset(2 as c_int as isize) as c_double > 10.0f64) as c_int
}

// Check whether location is valid.
// Prints error message on stderr and returns 0 if invalid, otherwise
// returns 1.
unsafe extern "C" fn location_is_valid(location: *const location_t) -> c_int {
    // Latitude
    if ((*location).lat as c_double) < MIN_LAT || (*location).lat as c_double > MAX_LAT {
        // TRANSLATORS: Append degree symbols if possible.
        eprintln!(
            "Latitude must be between {:.1} and {:.1}.",
            MIN_LAT, MAX_LAT,
        );
        return 0 as c_int;
    }

    // Longitude
    if ((*location).lon as c_double) < MIN_LON || (*location).lon as c_double > MAX_LON {
        // TRANSLATORS: Append degree symbols if possible.
        eprintln!(
            "Longitude must be between {:.1} and {:.1}.",
            MIN_LON, MAX_LON,
        );
        return 0 as c_int;
    }
    1 as c_int
}

// Wait for location to become available from provider.
// Waits until timeout (milliseconds) has elapsed or forever if timeout
// is -1. Writes location to loc. Returns -1 on error,
// 0 if timeout was reached, 1 if location became available.
unsafe extern "C" fn provider_get_location(
    provider: *const location_provider_t,
    state: *mut location_state_t,
    mut timeout: c_int,
    loc: *mut location_t,
) -> c_int {
    let mut available: c_int = 0 as c_int;
    let mut pollfds: [pollfd; 1] = [pollfd {
        fd: 0,
        events: 0,
        revents: 0,
    }; 1];
    while available == 0 {
        let loc_fd: c_int = ((*provider).get_fd).expect("non-null function pointer")(state);
        if loc_fd >= 0 as c_int {
            // Provider is dynamic.
            // TODO: This should use a monotonic time source.
            let mut now: c_double = 0.;
            let mut r: c_int = systemtime_get_time(&mut now);
            if r < 0 as c_int {
                eprintln!("Unable to read system time.");
                return -(1 as c_int);
            }

            // Poll on file descriptor until ready.
            pollfds[0 as c_int as usize].fd = loc_fd;
            pollfds[0 as c_int as usize].events = 0x1 as c_int as c_short;
            r = poll(pollfds.as_mut_ptr(), 1 as c_int as nfds_t, timeout);
            if r < 0 as c_int {
                perror(b"poll\0" as *const u8 as *const c_char);
                return -(1 as c_int);
            } else if r == 0 as c_int {
                return 0 as c_int;
            }
            let mut later: c_double = 0.;
            r = systemtime_get_time(&mut later);
            if r < 0 as c_int {
                eprintln!("Unable to read system time.");
                return -(1 as c_int);
            }

            // Adjust timeout by elapsed time
            if timeout >= 0 as c_int {
                timeout =
                    (timeout as c_double - (later - now) * 1000 as c_int as c_double) as c_int;
                timeout = if timeout < 0 as c_int {
                    0 as c_int
                } else {
                    timeout
                };
            }
        }
        let r_0: c_int =
            ((*provider).handle).expect("non-null function pointer")(state, loc, &mut available);
        if r_0 < 0 as c_int {
            return -(1 as c_int);
        }
    }
    1 as c_int
}

// Easing function for fade.
// See https://github.com/mietek/ease-tween
unsafe extern "C" fn ease_fade(t: c_double) -> c_double {
    if t <= 0 as c_int as c_double {
        return 0 as c_int as c_double;
    }
    if t >= 1 as c_int as c_double {
        return 1 as c_int as c_double;
    }
    1.0042954579734844f64
        * (-6.404_173_895_841_566_f64 * (-7.290_824_133_098_134_f64 * t).exp()).exp()
}

// Run continual mode loop
// This is the main loop of the continual mode which keeps track of the
// current time and continuously updates the screen to the appropriate
// color temperature.
unsafe extern "C" fn run_continual_mode(
    provider: *const location_provider_t,
    location_state: *mut location_state_t,
    scheme: *const transition_scheme_t,
    method: *const gamma_method_t,
    method_state: *mut gamma_state_t,
    use_fade: c_int,
    preserve_gamma: c_int,
    verbose: c_int,
) -> c_int {
    let mut r: c_int = 0;

    // Short fade parameters
    let mut fade_length: c_int = 0 as c_int;
    let mut fade_time: c_int = 0 as c_int;
    let mut fade_start_interp: ColorSetting = ColorSetting {
        temperature: 0,
        gamma: [0.; 3],
        brightness: 0.,
    };
    r = signals_install_handlers();
    if r < 0 as c_int {
        return r;
    }

    // Save previous parameters so we can avoid printing status updates if
    // the values did not change.
    let mut prev_period: period_t = PERIOD_NONE;

    // Previous target color setting and current actual color setting.
    // Actual color setting takes into account the current color fade.
    let mut prev_target_interp: ColorSetting = ColorSetting {
        temperature: 0,
        gamma: [0.; 3],
        brightness: 0.,
    };
    color_setting_reset(&mut prev_target_interp);
    let mut interp: ColorSetting = ColorSetting {
        temperature: 0,
        gamma: [0.; 3],
        brightness: 0.,
    };
    color_setting_reset(&mut interp);
    let mut loc: location_t = {
        location_t {
            lat: ::core::f32::NAN,
            lon: ::core::f32::NAN,
        }
    };

    let need_location: c_int = ((*scheme).use_time == 0) as c_int;
    if need_location != 0 {
        eprintln!("Waiting for initial location to become available...");

        // Get initial location from provider
        r = provider_get_location(provider, location_state, -(1 as c_int), &mut loc);
        if r < 0 as c_int {
            eprintln!("Unable to get location from provider.");
            return -(1 as c_int);
        }
        if location_is_valid(&mut loc) == 0 {
            eprintln!("Invalid location returned from provider.");
            return -(1 as c_int);
        }
        print_location(&mut loc);
    }

    if verbose != 0 {
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

    // Continuously adjust color temperature
    let mut done: c_int = 0 as c_int;
    let mut prev_disabled: c_int = 1 as c_int;
    let mut disabled: c_int = 0 as c_int;
    let mut location_available: c_int = 1 as c_int;
    loop {
        // Check to see if disable signal was caught
        if disable != 0 && done == 0 {
            disabled = (disabled == 0) as c_int;
            ::core::ptr::write_volatile(addr_of_mut!(disable) as *mut sig_atomic_t, 0 as c_int);
        }

        // Check to see if exit signal was caught
        if exiting != 0 {
            if done != 0 {
                // On second signal stop the ongoing fade.
                break;
            }
            done = 1 as c_int;
            disabled = 1 as c_int;
            ::core::ptr::write_volatile(addr_of_mut!(exiting) as *mut sig_atomic_t, 0 as c_int);
        }

        // Print status change
        if verbose != 0 && disabled != prev_disabled {
            printf(
                // gettext(
                b"Status: %s\n\0" as *const u8 as *const c_char,
                if disabled != 0 {
                    // gettext(
                    b"Disabled\0" as *const u8 as *const c_char
                } else {
                    // gettext(
                    b"Enabled\0" as *const u8 as *const c_char
                },
            );
        }
        prev_disabled = disabled;

        // Read timestamp
        let mut now: c_double = 0.;
        r = systemtime_get_time(&mut now);
        if r < 0 as c_int {
            eprintln!("Unable to read system time.");
            return -(1 as c_int);
        }

        let mut period: period_t = PERIOD_NONE;
        let mut transition_prog: c_double = 0.;
        if (*scheme).use_time != 0 {
            let time_offset: c_int = get_seconds_since_midnight(now);
            period = get_period_from_time(scheme, time_offset);
            transition_prog = get_transition_progress_from_time(scheme, time_offset);
        } else {
            // Current angular elevation of the sun
            let elevation: c_double =
                solar_elevation(now, loc.lat as c_double, loc.lon as c_double);
            period = get_period_from_elevation(scheme, elevation);
            transition_prog = get_transition_progress_from_elevation(scheme, elevation);
        }

        // Use transition progress to get target color
        //    temperature.
        let mut target_interp: ColorSetting = ColorSetting {
            temperature: 0,
            gamma: [0.; 3],
            brightness: 0.,
        };
        interpolate_transition_scheme(scheme, transition_prog, &mut target_interp);
        if disabled != 0 {
            period = PERIOD_NONE;
            color_setting_reset(&mut target_interp);
        }
        if done != 0 {
            period = PERIOD_NONE;
        }

        // Print period if it changed during this update,
        // or if we are in the transition period. In transition we
        // print the progress, so we always print it in
        // that case.
        if verbose != 0
            && (period as c_uint != prev_period as c_uint
                || period as c_uint == PERIOD_TRANSITION as c_int as c_uint)
        {
            print_period(period, transition_prog);
        }

        /* Activate hooks if period changed */
        if period as c_uint != prev_period as c_uint {
            hooks_signal_period_change(prev_period, period);
        }

        // Start fade if the parameter differences are too big to apply
        // instantly.
        if use_fade != 0
            && (fade_length == 0 as c_int
                && color_setting_diff_is_major(&mut interp, &mut target_interp) != 0
                || fade_length != 0 as c_int
                    && color_setting_diff_is_major(&mut target_interp, &mut prev_target_interp)
                        != 0)
        {
            fade_length = 40 as c_int;
            fade_time = 0 as c_int;
            fade_start_interp = interp;
        }

        // Handle ongoing fade
        if fade_length != 0 as c_int {
            fade_time += 1 as c_int;
            let frac: c_double = fade_time as c_double / fade_length as c_double;
            let alpha: c_double = if 0.0f64 > ease_fade(frac) {
                0.0f64
            } else if ease_fade(frac) < 1.0f64 {
                ease_fade(frac)
            } else {
                1.0f64
            };
            interpolate_color_settings(
                &mut fade_start_interp,
                &mut target_interp,
                alpha,
                &mut interp,
            );
            if fade_time > fade_length {
                fade_time = 0 as c_int;
                fade_length = 0 as c_int;
            }
        } else {
            interp = target_interp;
        }

        // Break loop when done and final fade is over
        if done != 0 && fade_length == 0 as c_int {
            break;
        }

        if verbose != 0 {
            if prev_target_interp.temperature != target_interp.temperature {
                printf(
                    // gettext(
                    b"Color temperature: %uK\n\0" as *const u8 as *const c_char,
                    target_interp.temperature,
                );
            }
            if prev_target_interp.brightness != target_interp.brightness {
                printf(
                    // gettext(
                    b"Brightness: %.2f\n\0" as *const u8 as *const c_char,
                    target_interp.brightness as c_double,
                );
            }
        }

        // Adjust temperature
        r = ((*method).set_temperature).expect("non-null function pointer")(
            method_state,
            &mut interp,
            preserve_gamma,
        );
        if r < 0 as c_int {
            eprintln!("Temperature adjustment failed.");
            return -(1 as c_int);
        }

        // Save period and target color setting as previous
        prev_period = period;
        prev_target_interp = target_interp;

        // Sleep length depends on whether a fade is ongoing.
        let mut delay: c_int = 5000 as c_int;
        if fade_length != 0 as c_int {
            delay = 100 as c_int;
        }

        // Update location.
        let mut loc_fd: c_int = -(1 as c_int);
        if need_location != 0 {
            loc_fd = ((*provider).get_fd).expect("non-null function pointer")(location_state);
        }

        if loc_fd >= 0 as c_int {
            // Provider is dynamic.
            let mut pollfds: [pollfd; 1] = [pollfd {
                fd: 0,
                events: 0,
                revents: 0,
            }; 1];

            pollfds[0 as c_int as usize].fd = loc_fd;
            pollfds[0 as c_int as usize].events = 0x1 as c_int as c_short;
            let mut r_0: c_int = poll(pollfds.as_mut_ptr(), 1 as c_int as nfds_t, delay);
            if r_0 < 0 as c_int {
                if *__errno_location() == 4 as c_int {
                    continue;
                }
                perror(b"poll\0" as *const u8 as *const c_char);
                eprintln!("Unable to get location from provider.");
                return -(1 as c_int);
            } else {
                if r_0 == 0 as c_int {
                    continue;
                }

                // Get new location and availability
                // information.
                let mut new_loc: location_t = location_t { lat: 0., lon: 0. };
                let mut new_available: c_int = 0;
                r_0 = ((*provider).handle).expect("non-null function pointer")(
                    location_state,
                    &mut new_loc,
                    &mut new_available,
                );
                if r_0 < 0 as c_int {
                    eprintln!("Unable to get location from provider.");
                    return -(1 as c_int);
                }
                if new_available == 0 && new_available != location_available {
                    eprintln!("Location is temporarily unavailable; Using previous location until it becomes available...");
                }

                if new_available != 0
                    && (new_loc.lat != loc.lat
                        || new_loc.lon != loc.lon
                        || new_available != location_available)
                {
                    loc = new_loc;
                    print_location(&mut loc);
                }
                location_available = new_available;
                if location_is_valid(&mut loc) == 0 {
                    eprintln!("Invalid location returned from provider.");
                    return -(1 as c_int);
                }
            }
        } else {
            systemtime_msleep(delay as c_uint);
        }
    }

    // Restore saved gamma ramps
    ((*method).restore).expect("non-null function pointer")(method_state);
    0 as c_int
}

unsafe fn main_0(argc: c_int, argv: *mut *mut c_char) -> c_int {
    let mut r: c_int = 0;
    // Init locale
    setlocale(0 as c_int, b"\0" as *const u8 as *const c_char);
    setlocale(5 as c_int, b"\0" as *const u8 as *const c_char);

    // Internationalisation
    // bindtextdomain(
    //     b"redshift\0" as *const u8 as *const c_char,
    //     b"/home/mahor/redshift/root/share/locale\0" as *const u8 as *const c_char,
    // );
    // textdomain(b"redshift\0" as *const u8 as *const c_char);

    // List of gamma methods.
    let gamma_methods: [gamma_method_t; 5] = [
        drm_gamma_method,
        randr_gamma_method,
        vidmode_gamma_method,
        dummy_gamma_method,
        {
            gamma_method_t {
                name: std::ptr::null_mut::<c_char>(),
                autostart: 0,
                init: None,
                start: None,
                free: None,
                print_help: None,
                set_option: None,
                restore: None,
                set_temperature: None,
            }
        },
    ];

    // List of location providers.
    let location_providers: [location_provider_t; 3] =
        [geoclue2_location_provider, manual_location_provider, {
            location_provider_t {
                name: std::ptr::null_mut::<c_char>(),
                init: None,
                start: None,
                free: None,
                print_help: None,
                set_option: None,
                get_fd: None,
                handle: None,
            }
        }];

    // Flush messages consistently even if redirected to a pipe or
    // file.  Change the flush behaviour to line-buffered, without
    // changing the actual buffers being used.
    setvbuf(stdout, std::ptr::null_mut::<c_char>(), 1 as c_int, 0);
    setvbuf(stderr, std::ptr::null_mut::<c_char>(), 1 as c_int, 0);

    let mut options: options_t = options_t {
        config_filepath: std::ptr::null_mut::<c_char>(),
        scheme: transition_scheme_t {
            high: 0.,
            low: 0.,
            use_time: 0,
            dawn: time_range_t { start: 0, end: 0 },
            dusk: time_range_t { start: 0, end: 0 },
            day: ColorSetting {
                temperature: 0,
                gamma: [0.; 3],
                brightness: 0.,
            },
            night: ColorSetting {
                temperature: 0,
                gamma: [0.; 3],
                brightness: 0.,
            },
        },
        mode: PROGRAM_MODE_CONTINUAL,
        verbose: 0,
        temp_set: 0,
        use_fade: 0,
        preserve_gamma: 0,
        method: std::ptr::null::<gamma_method_t>(),
        method_args: std::ptr::null_mut::<c_char>(),
        provider: std::ptr::null::<location_provider_t>(),
        provider_args: std::ptr::null_mut::<c_char>(),
    };
    options_init(&mut options);
    options_parse_args(
        &mut options,
        argc,
        argv,
        gamma_methods.as_ptr(),
        location_providers.as_ptr(),
    );

    // Load settings from config file.
    let config = Config::new();

    options_parse_config_file(
        &mut options,
        &mut config_state,
        gamma_methods.as_ptr(),
        location_providers.as_ptr(),
    );
    options_set_defaults(&mut options);
    if options.scheme.dawn.start >= 0 as c_int
        || options.scheme.dawn.end >= 0 as c_int
        || options.scheme.dusk.start >= 0 as c_int
        || options.scheme.dusk.end >= 0 as c_int
    {
        if options.scheme.dawn.start < 0 as c_int
            || options.scheme.dawn.end < 0 as c_int
            || options.scheme.dusk.start < 0 as c_int
            || options.scheme.dusk.end < 0 as c_int
        {
            eprintln!("Partial time-configuration not supported!");
            exit(1 as c_int);
        }
        if options.scheme.dawn.start > options.scheme.dawn.end
            || options.scheme.dawn.end > options.scheme.dusk.start
            || options.scheme.dusk.start > options.scheme.dusk.end
        {
            eprintln!("Invalid dawn/dusk time configuration!");
            exit(1 as c_int);
        }
        options.scheme.use_time = 1 as c_int;
    }

    // Initialize location provider if needed. If provider is NULL
    // try all providers until one that works is found.
    let mut location_state: *mut location_state_t = std::ptr::null_mut::<location_state_t>();

    // Location is not needed for reset mode and manual mode.
    let need_location: c_int = (options.mode as c_uint != PROGRAM_MODE_RESET as c_int as c_uint
        && options.mode as c_uint != PROGRAM_MODE_MANUAL as c_int as c_uint
        && options.scheme.use_time == 0) as c_int;
    if need_location != 0 {
        if !(options.provider).is_null() {
            // Use provider specified on command line.
            r = provider_try_start(
                options.provider,
                &mut location_state,
                &mut config_state,
                options.provider_args,
            );
            if r < 0 as c_int {
                exit(1 as c_int);
            }
        } else {
            // Try all providers, use the first that works.
            let mut i: c_int = 0 as c_int;
            while !(location_providers[i as usize].name).is_null() {
                let p: *const location_provider_t =
                    &*location_providers.as_ptr().offset(i as isize) as *const location_provider_t;
                fprintf(
                    stderr,
                    // gettext(
                    b"Trying location provider `%s'...\n\0" as *const u8 as *const c_char,
                    (*p).name,
                );
                r = provider_try_start(
                    p,
                    &mut location_state,
                    &mut config_state,
                    std::ptr::null_mut::<c_char>(),
                );
                if r < 0 as c_int {
                    fputs(
                        // gettext(
                        b"Trying next provider...\n\0" as *const u8 as *const c_char,
                        stderr,
                    );
                    i += 1;
                    i;
                } else {
                    // Found provider that works.
                    printf(
                        // gettext(
                        b"Using provider `%s'.\n\0" as *const u8 as *const c_char,
                        (*p).name,
                    );
                    options.provider = p;
                    break;
                }
            }
            if (options.provider).is_null() {
                // Failure if no providers were successful at this
                //    point.
                fputs(
                    // gettext(
                    b"No more location providers to try.\n\0" as *const u8 as *const c_char,
                    stderr,
                );
                exit(1 as c_int);
            }
        }
        if options.scheme.high < options.scheme.low {
            // Solar elevations
            fprintf(
                stderr,
                // gettext(
                b"High transition elevation cannot be lower than the low transition elevation.\n\0"
                    as *const u8 as *const c_char,
                // ),
            );
            exit(1 as c_int);
        }
        if options.verbose != 0 {
            // TRANSLATORS: Append degree symbols if possible.
            printf(
                // gettext(
                b"Solar elevations: day above %.1f, night below %.1f\n\0" as *const u8
                    as *const c_char,
                // ),
                options.scheme.high,
                options.scheme.low,
            );
        }
    }

    if options.mode as c_uint != PROGRAM_MODE_RESET as c_int as c_uint
        && options.mode as c_uint != PROGRAM_MODE_MANUAL as c_int as c_uint
    {
        if options.verbose != 0 {
            printf(
                // gettext(
                b"Temperatures: %dK at day, %dK at night\n\0" as *const u8 as *const c_char,
                // ),
                options.scheme.day.temperature,
                options.scheme.night.temperature,
            );
        }

        // Color temperature
        if options.scheme.day.temperature < 1000 as c_int
            || options.scheme.day.temperature > 25000 as c_int
            || options.scheme.night.temperature < 1000 as c_int
            || options.scheme.night.temperature > 25000 as c_int
        {
            fprintf(
                stderr,
                // gettext(
                b"Temperature must be between %uK and %uK.\n\0" as *const u8 as *const c_char,
                // ),
                1000 as c_int,
                25000 as c_int,
            );
            exit(1 as c_int);
        }
    }

    /* Check color temperature to be set */
    if options.mode as c_uint == PROGRAM_MODE_MANUAL as c_int as c_uint
        && (options.temp_set < 1000 as c_int || options.temp_set > 25000 as c_int)
    {
        fprintf(
            stderr,
            // gettext(
            b"Temperature must be between %uK and %uK.\n\0" as *const u8 as *const c_char,
            // ),
            1000 as c_int,
            25000 as c_int,
        );
        exit(1 as c_int);
    }

    // Brightness
    if (options.scheme.day.brightness as c_double) < 0.1f64
        || options.scheme.day.brightness as c_double > 1.0f64
        || (options.scheme.night.brightness as c_double) < 0.1f64
        || options.scheme.night.brightness as c_double > 1.0f64
    {
        fprintf(
            stderr,
            // gettext(
            b"Brightness values must be between %.1f and %.1f.\n\0" as *const u8 as *const c_char,
            // ),
            0.1f64,
            1.0f64,
        );
        exit(1 as c_int);
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
                if location_is_valid(&mut loc) == 0 {
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
    0 as c_int
}

pub fn main() {
    let mut args: Vec<*mut c_char> = Vec::new();
    for arg in ::std::env::args() {
        args.push(
            (::std::ffi::CString::new(arg))
                .expect("Failed to convert argument into CString.")
                .into_raw(),
        );
    }
    args.push(::core::ptr::null_mut());
    unsafe {
        ::std::process::exit(main_0(
            (args.len() - 1) as c_int,
            args.as_mut_ptr() as *mut *mut c_char,
        ) as i32)
    }
}
