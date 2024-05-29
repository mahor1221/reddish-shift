/*  options.rs -- Program options
    This file is part of <https://github.com/mahor1221/reddish-shift>.
    Copyright (C) 2024 Mahor Foruzesh <mahor1221@gmail.com>
    Ported from Redshift <https://github.com/jonls/redshift>.
    Copyright (c) 2017  Jon Lund Steffensen <jonlst@gmail.com>

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

use super::config_ini::{
    config_ini_get_section, config_ini_section_t, config_ini_setting_t, config_ini_state_t,
};
use crate::{
    gamma_method_t, location_provider_t, program_mode_t, solar::SOLAR_CIVIL_TWILIGHT_ELEV, stdout,
    time_range_t, transition_scheme_t, PROGRAM_MODE_CONTINUAL, PROGRAM_MODE_MANUAL,
    PROGRAM_MODE_ONE_SHOT, PROGRAM_MODE_PRINT, PROGRAM_MODE_RESET,
};
use libc::{
    __errno_location, atof, atoi, exit, free, getopt, memcpy, printf, strcasecmp, strchr, strdup,
    strtof, strtol,
};
use std::ffi::{c_char, c_float, c_int, c_long, c_void, CStr};

extern "C" {
    static optarg: *mut libc::c_char;
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct options_t {
    // Path to config file
    pub config_filepath: *mut c_char,
    pub scheme: transition_scheme_t,
    pub mode: program_mode_t,
    pub verbose: c_int,

    // Whether to preserve gamma ramps if supported by gamma method.
    pub preserve_gamma: c_int,

    // Temperature to set in manual mode.
    pub temp_set: c_int,
    // Whether to fade between large skips in color temperature.
    pub use_fade: c_int,
    // Selected gamma method.
    pub method: *const gamma_method_t,
    // Arguments for gamma method.
    pub method_args: *mut c_char,
    // Selected location provider.
    pub provider: *const location_provider_t,
    // Arguments for location provider.
    pub provider_args: *mut c_char,
}

unsafe extern "C" fn parse_brightness_string(
    str: *const c_char,
    bright_day: *mut c_float,
    bright_night: *mut c_float,
) {
    let mut s: *mut c_char = strchr(str, ':' as i32);
    if s.is_null() {
        *bright_night = atof(str) as c_float;
        *bright_day = *bright_night;
    } else {
        let fresh0 = s;
        s = s.offset(1);
        *fresh0 = '\0' as i32 as c_char;
        *bright_day = atof(str) as c_float;
        *bright_night = atof(s) as c_float;
    };
}

unsafe extern "C" fn parse_gamma_string(str: *const c_char, gamma: *mut c_float) -> c_int {
    let mut s: *mut c_char = strchr(str, ':' as i32);
    if s.is_null() {
        let g: c_float = atof(str) as c_float;
        let fresh1 = &mut (*gamma.offset(2 as c_int as isize));
        *fresh1 = g;
        let fresh2 = &mut (*gamma.offset(1 as c_int as isize));
        *fresh2 = *fresh1;
        *gamma.offset(0 as c_int as isize) = *fresh2;
    } else {
        let fresh3 = s;
        s = s.offset(1);
        *fresh3 = '\0' as i32 as c_char;
        let g_s: *mut c_char = s;
        s = strchr(s, ':' as i32);
        if s.is_null() {
            return -(1 as c_int);
        }
        let fresh4 = s;
        s = s.offset(1);
        *fresh4 = '\0' as i32 as c_char;
        *gamma.offset(0 as c_int as isize) = atof(str) as c_float;
        *gamma.offset(1 as c_int as isize) = atof(g_s) as c_float;
        *gamma.offset(2 as c_int as isize) = atof(s) as c_float;
    }
    0 as c_int
}

unsafe extern "C" fn parse_transition_time(str: *const c_char, end: *mut *const c_char) -> c_int {
    let mut min: *const c_char = std::ptr::null::<c_char>();
    *__errno_location() = 0 as c_int;
    let hours: c_long = strtol(
        str,
        &mut min as *mut *const c_char as *mut *mut c_char,
        10 as c_int,
    );
    if *__errno_location() != 0 as c_int
        || min == str
        || *min.offset(0 as c_int as isize) as c_int != ':' as i32
        || hours < 0 as c_int as c_long
        || hours >= 24 as c_int as c_long
    {
        return -(1 as c_int);
    }
    min = min.offset(1 as c_int as isize);
    *__errno_location() = 0 as c_int;
    let minutes: c_long = strtol(min, end as *mut *mut c_char, 10 as c_int);
    if *__errno_location() != 0 as c_int
        || *end == min
        || minutes < 0 as c_int as c_long
        || minutes >= 60 as c_int as c_long
    {
        return -(1 as c_int);
    }
    (minutes * 60 as c_int as c_long + hours * 3600 as c_int as c_long) as c_int
}

unsafe extern "C" fn parse_transition_range(str: *const c_char, range: *mut time_range_t) -> c_int {
    let mut next: *const c_char = std::ptr::null::<c_char>();
    let start_time: c_int = parse_transition_time(str, &mut next);
    if start_time < 0 as c_int {
        return -(1 as c_int);
    }
    let mut end_time: c_int = 0;
    if *next.offset(0 as c_int as isize) as c_int == '\0' as i32 {
        end_time = start_time;
    } else if *next.offset(0 as c_int as isize) as c_int == '-' as i32 {
        next = next.offset(1 as c_int as isize);
        let mut end: *const c_char = std::ptr::null::<c_char>();
        end_time = parse_transition_time(next, &mut end);
        if end_time < 0 as c_int || *end.offset(0 as c_int as isize) as c_int != '\0' as i32 {
            return -(1 as c_int);
        }
    } else {
        return -(1 as c_int);
    }
    (*range).start = start_time;
    (*range).end = end_time;
    0 as c_int
}

unsafe extern "C" fn print_method_list(gamma_methods: *const gamma_method_t) {
    // gettext(
    println!("Available adjustment methods:");

    let mut i: c_int = 0 as c_int;
    while !((*gamma_methods.offset(i as isize)).name).is_null() {
        let name = (*gamma_methods.offset(i as isize)).name;
        let name = CStr::from_ptr(name).to_str().unwrap();
        println!("  {name}");
        i += 1;
    }

    // TRANSLATORS: `help' must not be translated.
    println!(
        "
Specify colon-separated options with `-m METHOD:OPTIONS`
Try `-m METHOD:help' for help."
    );
}

// Print list of location providers.
unsafe extern "C" fn print_provider_list(location_providers: *const location_provider_t) {
    // gettext(
    println!("Available location providers:");

    let mut i: c_int = 0 as c_int;
    while !((*location_providers.offset(i as isize)).name).is_null() {
        let name = (*location_providers.offset(i as isize)).name;
        let name = CStr::from_ptr(name).to_str().unwrap();
        println!("  {name}");
        i += 1;
    }

    // TRANSLATORS: `help' must not be translated.
    println!(
        "
Specify colon-separated options with`-l PROVIDER:OPTIONS'.
Try `-l PROVIDER:help' for help.
"
    );
}

// Return the gamma method with the given name.
unsafe extern "C" fn find_gamma_method(
    gamma_methods: *const gamma_method_t,
    name: *const c_char,
) -> *const gamma_method_t {
    let mut method: *const gamma_method_t = std::ptr::null::<gamma_method_t>();
    let mut i: c_int = 0 as c_int;
    while !((*gamma_methods.offset(i as isize)).name).is_null() {
        let m: *const gamma_method_t = &*gamma_methods.offset(i as isize) as *const gamma_method_t;
        if strcasecmp(name, (*m).name) == 0 as c_int {
            method = m;
            break;
        } else {
            i += 1;
            i;
        }
    }
    method
}

// Return location provider with the given name.
unsafe extern "C" fn find_location_provider(
    location_providers: *const location_provider_t,
    name: *const c_char,
) -> *const location_provider_t {
    let mut provider: *const location_provider_t = std::ptr::null::<location_provider_t>();
    let mut i: c_int = 0 as c_int;
    while !((*location_providers.offset(i as isize)).name).is_null() {
        let p: *const location_provider_t =
            &*location_providers.offset(i as isize) as *const location_provider_t;
        if strcasecmp(name, (*p).name) == 0 as c_int {
            provider = p;
            break;
        } else {
            i += 1;
            i;
        }
    }
    provider
}

// Initialize options struct.
#[no_mangle]
pub unsafe extern "C" fn options_init(options: *mut options_t) {
    (*options).config_filepath = std::ptr::null_mut::<c_char>();

    // Default elevation values.
    (*options).scheme.high = TRANSITION_HIGH;
    (*options).scheme.low = TRANSITION_LOW;

    // Settings for day, night and transition period.
    //    Initialized to indicate that the values are not set yet.
    (*options).scheme.use_time = 0 as c_int;
    (*options).scheme.dawn.start = -(1 as c_int);
    (*options).scheme.dawn.end = -(1 as c_int);
    (*options).scheme.dusk.start = -(1 as c_int);
    (*options).scheme.dusk.end = -(1 as c_int);

    (*options).scheme.day.temperature = -(1 as c_int);
    (*options).scheme.day.gamma[0 as c_int as usize] = ::core::f32::NAN;
    (*options).scheme.day.brightness = ::core::f32::NAN;

    (*options).scheme.night.temperature = -(1 as c_int);
    (*options).scheme.night.gamma[0 as c_int as usize] = ::core::f32::NAN;
    (*options).scheme.night.brightness = ::core::f32::NAN;

    // Temperature for manual mode
    (*options).temp_set = -(1 as c_int);

    (*options).method = std::ptr::null::<gamma_method_t>();
    (*options).method_args = std::ptr::null_mut::<c_char>();

    (*options).provider = std::ptr::null::<location_provider_t>();
    (*options).provider_args = std::ptr::null_mut::<c_char>();

    (*options).use_fade = -(1 as c_int);
    (*options).preserve_gamma = 1 as c_int;
    (*options).mode = PROGRAM_MODE_CONTINUAL;
    (*options).verbose = 0 as c_int;
}

// Parse a single option from the command-line.
unsafe extern "C" fn parse_command_line_option(
    option: c_char,
    value: *mut c_char,
    options: *mut options_t,
    program_name: *const c_char,
    gamma_methods: *const gamma_method_t,
    location_providers: *const location_provider_t,
) -> c_int {
    let mut r: c_int = 0;
    let mut s: *mut c_char = std::ptr::null_mut::<c_char>();
    let mut provider_name: *mut c_char = std::ptr::null_mut::<c_char>();
    let mut end: *mut c_char = std::ptr::null_mut::<c_char>();
    match option as c_int {
        98 => {
            parse_brightness_string(
                value,
                &mut (*options).scheme.day.brightness,
                &mut (*options).scheme.night.brightness,
            );
        }
        99 => {
            free((*options).config_filepath as *mut c_void);
            (*options).config_filepath = strdup(value);
        }
        103 => {
            r = parse_gamma_string(value, ((*options).scheme.day.gamma).as_mut_ptr());
            if r < 0 as c_int {
                eprintln!("Malformed gamma argument.\nTry `-h' for more information.");
                return -(1 as c_int);
            }

            // Set night gamma to the same value as day gamma.
            // To set these to distinct values use the config
            // file.
            memcpy(
                ((*options).scheme.night.gamma).as_mut_ptr() as *mut c_void,
                ((*options).scheme.day.gamma).as_mut_ptr() as *const c_void,
                ::core::mem::size_of::<[c_float; 3]>(),
            );
        }
        104 => {
            print_help(program_name);
            exit(0 as c_int);
        }
        108 => {
            // Print list of providers if argument is `list'
            if strcasecmp(value, b"list\0" as *const u8 as *const c_char) == 0 as c_int {
                print_provider_list(location_providers);
                exit(0 as c_int);
            }
            provider_name = std::ptr::null_mut::<c_char>();

            // Don't save the result of strtof(); we simply want
            // to know if value can be parsed as a float.
            *__errno_location() = 0 as c_int;
            end = std::ptr::null_mut::<c_char>();
            strtof(value, &mut end);
            if *__errno_location() == 0 as c_int && *end as c_int == ':' as i32 {
                // Use instead as arguments to `manual'.
                provider_name = b"manual\0" as *const u8 as *const c_char as *mut c_char;
                (*options).provider_args = value;
            } else {
                // Split off provider arguments.
                s = strchr(value, ':' as i32);
                if !s.is_null() {
                    let fresh5 = s;
                    s = s.offset(1);
                    *fresh5 = '\0' as i32 as c_char;
                    (*options).provider_args = s;
                }
                provider_name = value;
            }
            // Lookup provider from name.
            (*options).provider = find_location_provider(location_providers, provider_name);
            if ((*options).provider).is_null() {
                eprintln!(
                    "Unknown location provider `{}`.",
                    CStr::from_ptr(provider_name).to_str().unwrap()
                );
                return -(1 as c_int);
            }
            // Print provider help if arg is `help'.
            if !((*options).provider_args).is_null()
                && strcasecmp(
                    (*options).provider_args,
                    b"help\0" as *const u8 as *const c_char,
                ) == 0 as c_int
            {
                ((*(*options).provider).print_help).expect("non-null function pointer")(stdout);
                exit(0 as c_int);
            }
        }
        109 => {
            // Print list of methods if argument is `list'
            if strcasecmp(value, b"list\0" as *const u8 as *const c_char) == 0 as c_int {
                print_method_list(gamma_methods);
                exit(0 as c_int);
            }
            // Split off method arguments.
            s = strchr(value, ':' as i32);
            if !s.is_null() {
                let fresh6 = s;
                s = s.offset(1);
                *fresh6 = '\0' as i32 as c_char;
                (*options).method_args = s;
            }
            // Find adjustment method by name.
            (*options).method = find_gamma_method(gamma_methods, value);
            if ((*options).method).is_null() {
                // TRANSLATORS: This refers to the method
                //    used to adjust colors e.g VidMode
                eprintln!(
                    "Unknown adjustment method `{}`",
                    CStr::from_ptr(value).to_str().unwrap()
                );
                return -(1 as c_int);
            }
            // Print method help if arg is `help'.
            if !((*options).method_args).is_null()
                && strcasecmp(
                    (*options).method_args,
                    b"help\0" as *const u8 as *const c_char,
                ) == 0 as c_int
            {
                ((*(*options).method).print_help).expect("non-null function pointer")(stdout);
                exit(0 as c_int);
            }
        }
        111 => {
            (*options).mode = PROGRAM_MODE_ONE_SHOT;
        }
        79 => {
            (*options).mode = PROGRAM_MODE_MANUAL;
            (*options).temp_set = atoi(value);
        }
        112 => {
            (*options).mode = PROGRAM_MODE_PRINT;
        }
        80 => {
            (*options).preserve_gamma = 0 as c_int;
        }
        114 => {
            (*options).use_fade = 0 as c_int;
        }
        116 => {
            s = strchr(value, ':' as i32);
            if s.is_null() {
                // gettext(
                eprintln!("Malformed temperature argument.\nTry `-h' for more information.");
                return -(1 as c_int);
            }
            let fresh7 = s;
            s = s.offset(1);
            *fresh7 = '\0' as i32 as c_char;
            (*options).scheme.day.temperature = atoi(value);
            (*options).scheme.night.temperature = atoi(s);
        }
        118 => {
            (*options).verbose = 1 as c_int;
        }
        86 => {
            printf(
                b"%s\n\0" as *const u8 as *const c_char,
                b"redshift 1.12\0" as *const u8 as *const c_char,
            );
            exit(0 as c_int);
        }
        120 => {
            (*options).mode = PROGRAM_MODE_RESET;
        }
        63 => {
            eprintln!("Try `-h' for more information.");
            return -(1 as c_int);
        }
        _ => {}
    }
    0 as c_int
}

// Parse command line arguments.
#[no_mangle]
pub unsafe extern "C" fn options_parse_args(
    options: *mut options_t,
    argc: c_int,
    argv: *mut *mut c_char,
    gamma_methods: *const gamma_method_t,
    location_providers: *const location_provider_t,
) {
    let program_name: *const c_char = *argv.offset(0 as c_int as isize);
    let mut opt: c_int = 0;
    loop {
        opt = getopt(
            argc,
            argv as *const *mut c_char,
            b"b:c:g:hl:m:oO:pPrt:vVx\0" as *const u8 as *const c_char,
        );
        if opt == -(1 as c_int) {
            break;
        }
        let option: c_char = opt as c_char;
        let r: c_int = parse_command_line_option(
            option,
            optarg,
            options,
            program_name,
            gamma_methods,
            location_providers,
        );
        if r < 0 as c_int {
            exit(1 as c_int);
        }
    }
}

// Parse a single key-value pair from the configuration file.
unsafe extern "C" fn parse_config_file_option(
    key: *const c_char,
    value: *const c_char,
    options: *mut options_t,
    gamma_methods: *const gamma_method_t,
    location_providers: *const location_provider_t,
) -> c_int {
    if strcasecmp(key, b"temp-day\0" as *const u8 as *const c_char) == 0 as c_int {
        if (*options).scheme.day.temperature < 0 as c_int {
            (*options).scheme.day.temperature = atoi(value);
        }
    } else if strcasecmp(key, b"temp-night\0" as *const u8 as *const c_char) == 0 as c_int {
        if (*options).scheme.night.temperature < 0 as c_int {
            (*options).scheme.night.temperature = atoi(value);
        }
    } else if strcasecmp(key, b"transition\0" as *const u8 as *const c_char) == 0 as c_int
        || strcasecmp(key, b"fade\0" as *const u8 as *const c_char) == 0 as c_int
    {
        // "fade" is preferred, "transition" is
        //    deprecated as the setting key.
        if (*options).use_fade < 0 as c_int {
            (*options).use_fade = (atoi(value) != 0) as c_int;
        }
    } else if strcasecmp(key, b"brightness\0" as *const u8 as *const c_char) == 0 as c_int {
        if ((*options).scheme.day.brightness).is_nan() as i32 != 0 {
            (*options).scheme.day.brightness = atof(value) as c_float;
        }
        if ((*options).scheme.night.brightness).is_nan() as i32 != 0 {
            (*options).scheme.night.brightness = atof(value) as c_float;
        }
    } else if strcasecmp(key, b"brightness-day\0" as *const u8 as *const c_char) == 0 as c_int {
        if ((*options).scheme.day.brightness).is_nan() as i32 != 0 {
            (*options).scheme.day.brightness = atof(value) as c_float;
        }
    } else if strcasecmp(key, b"brightness-night\0" as *const u8 as *const c_char) == 0 as c_int {
        if ((*options).scheme.night.brightness).is_nan() as i32 != 0 {
            (*options).scheme.night.brightness = atof(value) as c_float;
        }
    } else if strcasecmp(key, b"elevation-high\0" as *const u8 as *const c_char) == 0 as c_int {
        (*options).scheme.high = atof(value);
    } else if strcasecmp(key, b"elevation-low\0" as *const u8 as *const c_char) == 0 as c_int {
        (*options).scheme.low = atof(value);
    } else if strcasecmp(key, b"gamma\0" as *const u8 as *const c_char) == 0 as c_int {
        if ((*options).scheme.day.gamma[0 as c_int as usize]).is_nan() as i32 != 0 {
            let r: c_int = parse_gamma_string(value, ((*options).scheme.day.gamma).as_mut_ptr());
            if r < 0 as c_int {
                eprintln!("Malformed gamma setting.");
                return -(1 as c_int);
            }
            memcpy(
                ((*options).scheme.night.gamma).as_mut_ptr() as *mut c_void,
                ((*options).scheme.day.gamma).as_mut_ptr() as *const c_void,
                ::core::mem::size_of::<[c_float; 3]>(),
            );
        }
    } else if strcasecmp(key, b"gamma-day\0" as *const u8 as *const c_char) == 0 as c_int {
        if ((*options).scheme.day.gamma[0 as c_int as usize]).is_nan() as i32 != 0 {
            let r_0: c_int = parse_gamma_string(value, ((*options).scheme.day.gamma).as_mut_ptr());
            if r_0 < 0 as c_int {
                eprintln!("Malformed gamma setting.");
                return -(1 as c_int);
            }
        }
    } else if strcasecmp(key, b"gamma-night\0" as *const u8 as *const c_char) == 0 as c_int {
        if ((*options).scheme.night.gamma[0 as c_int as usize]).is_nan() as i32 != 0 {
            let r_1: c_int =
                parse_gamma_string(value, ((*options).scheme.night.gamma).as_mut_ptr());
            if r_1 < 0 as c_int {
                eprintln!("Malformed gamma setting.");
                return -(1 as c_int);
            }
        }
    } else if strcasecmp(key, b"adjustment-method\0" as *const u8 as *const c_char) == 0 as c_int {
        if ((*options).method).is_null() {
            (*options).method = find_gamma_method(gamma_methods, value);
            if ((*options).method).is_null() {
                eprintln!(
                    "Unknown adjustment method `{}`.",
                    CStr::from_ptr(value).to_str().unwrap(),
                );
                return -(1 as c_int);
            }
        }
    } else if strcasecmp(key, b"location-provider\0" as *const u8 as *const c_char) == 0 as c_int {
        if ((*options).provider).is_null() {
            (*options).provider = find_location_provider(location_providers, value);
            if ((*options).provider).is_null() {
                eprintln!(
                    "Unknown location provider `{}`.",
                    CStr::from_ptr(value).to_str().unwrap(),
                );
                return -(1 as c_int);
            }
        }
    } else if strcasecmp(key, b"dawn-time\0" as *const u8 as *const c_char) == 0 as c_int {
        if (*options).scheme.dawn.start < 0 as c_int {
            let r_2: c_int = parse_transition_range(value, &mut (*options).scheme.dawn);
            if r_2 < 0 as c_int {
                eprintln!(
                    "Malformed dawn-time setting `{}`.",
                    CStr::from_ptr(value).to_str().unwrap(),
                );
                return -(1 as c_int);
            }
        }
    } else if strcasecmp(key, b"dusk-time\0" as *const u8 as *const c_char) == 0 as c_int {
        if (*options).scheme.dusk.start < 0 as c_int {
            let r_3: c_int = parse_transition_range(value, &mut (*options).scheme.dusk);
            if r_3 < 0 as c_int {
                eprintln!(
                    "Malformed dusk-time setting `{}`.",
                    CStr::from_ptr(value).to_str().unwrap(),
                );
                return -(1 as c_int);
            }
        }
    } else {
        eprintln!(
            "Unknown configuration setting `{}`.",
            CStr::from_ptr(key).to_str().unwrap(),
        );
    }
    0 as c_int
}

// Parse options defined in the config file.
#[no_mangle]
pub unsafe extern "C" fn options_parse_config_file(
    options: *mut options_t,
    config_state: *mut config_ini_state_t,
    gamma_methods: *const gamma_method_t,
    location_providers: *const location_provider_t,
) {
    // Read global config settings.
    let section: *mut config_ini_section_t =
        config_ini_get_section(config_state, b"redshift\0" as *const u8 as *const c_char);
    if section.is_null() {
        return;
    }
    let mut setting: *mut config_ini_setting_t = (*section).settings;
    while !setting.is_null() {
        let r: c_int = parse_config_file_option(
            (*setting).name,
            (*setting).value,
            options,
            gamma_methods,
            location_providers,
        );
        if r < 0 as c_int {
            exit(1 as c_int);
        }
        setting = (*setting).next;
    }
}

// Replace unspecified options with default values.
#[no_mangle]
pub unsafe extern "C" fn options_set_defaults(options: *mut options_t) {
    if (*options).scheme.day.temperature < 0 as c_int {
        (*options).scheme.day.temperature = 6500 as c_int;
    }
    if (*options).scheme.night.temperature < 0 as c_int {
        (*options).scheme.night.temperature = 4500 as c_int;
    }
    if ((*options).scheme.day.brightness).is_nan() as i32 != 0 {
        (*options).scheme.day.brightness = 1.0f64 as c_float;
    }
    if ((*options).scheme.night.brightness).is_nan() as i32 != 0 {
        (*options).scheme.night.brightness = 1.0f64 as c_float;
    }
    if ((*options).scheme.day.gamma[0 as c_int as usize]).is_nan() as i32 != 0 {
        (*options).scheme.day.gamma[0 as c_int as usize] = 1.0f64 as c_float;
        (*options).scheme.day.gamma[1 as c_int as usize] = 1.0f64 as c_float;
        (*options).scheme.day.gamma[2 as c_int as usize] = 1.0f64 as c_float;
    }
    if ((*options).scheme.night.gamma[0 as c_int as usize]).is_nan() as i32 != 0 {
        (*options).scheme.night.gamma[0 as c_int as usize] = 1.0f64 as c_float;
        (*options).scheme.night.gamma[1 as c_int as usize] = 1.0f64 as c_float;
        (*options).scheme.night.gamma[2 as c_int as usize] = 1.0f64 as c_float;
    }
    if (*options).use_fade < 0 as c_int {
        (*options).use_fade = 1 as c_int;
    }
}
