use super::config_ini::{
    config_ini_get_section, config_ini_section_t, config_ini_setting_t, config_ini_state_t,
};
use crate::{
    color_setting_t, gamma_method_t, location_provider_t, options_t, program_mode_t, stdout,
    time_range_t, PROGRAM_MODE_CONTINUAL, PROGRAM_MODE_MANUAL, PROGRAM_MODE_ONE_SHOT,
    PROGRAM_MODE_PRINT, PROGRAM_MODE_RESET,
};
use libc::{
    __errno_location, atof, atoi, exit, free, getopt, memcpy, printf, strcasecmp, strchr, strdup,
    strtof, strtol, FILE,
};
use std::ffi::{c_char, c_double, c_float, c_int, c_long, c_uint, c_ulong, c_void, CStr};

extern "C" {
    static optarg: *mut libc::c_char;
}

unsafe extern "C" fn parse_brightness_string(
    mut str: *const c_char,
    mut bright_day: *mut c_float,
    mut bright_night: *mut c_float,
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

unsafe extern "C" fn parse_gamma_string(mut str: *const c_char, mut gamma: *mut c_float) -> c_int {
    let mut s: *mut c_char = strchr(str, ':' as i32);
    if s.is_null() {
        let mut g: c_float = atof(str) as c_float;
        let ref mut fresh1 = *gamma.offset(2 as c_int as isize);
        *fresh1 = g;
        let ref mut fresh2 = *gamma.offset(1 as c_int as isize);
        *fresh2 = *fresh1;
        *gamma.offset(0 as c_int as isize) = *fresh2;
    } else {
        let fresh3 = s;
        s = s.offset(1);
        *fresh3 = '\0' as i32 as c_char;
        let mut g_s: *mut c_char = s;
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
    return 0 as c_int;
}

unsafe extern "C" fn parse_transition_time(
    mut str: *const c_char,
    mut end: *mut *const c_char,
) -> c_int {
    let mut min: *const c_char = 0 as *const c_char;
    *__errno_location() = 0 as c_int;
    let mut hours: c_long = strtol(
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
    let mut minutes: c_long = strtol(min, end as *mut *mut c_char, 10 as c_int);
    if *__errno_location() != 0 as c_int
        || *end == min
        || minutes < 0 as c_int as c_long
        || minutes >= 60 as c_int as c_long
    {
        return -(1 as c_int);
    }
    return (minutes * 60 as c_int as c_long + hours * 3600 as c_int as c_long) as c_int;
}

unsafe extern "C" fn parse_transition_range(
    mut str: *const c_char,
    mut range: *mut time_range_t,
) -> c_int {
    let mut next: *const c_char = 0 as *const c_char;
    let mut start_time: c_int = parse_transition_time(str, &mut next);
    if start_time < 0 as c_int {
        return -(1 as c_int);
    }
    let mut end_time: c_int = 0;
    if *next.offset(0 as c_int as isize) as c_int == '\0' as i32 {
        end_time = start_time;
    } else if *next.offset(0 as c_int as isize) as c_int == '-' as i32 {
        next = next.offset(1 as c_int as isize);
        let mut end: *const c_char = 0 as *const c_char;
        end_time = parse_transition_time(next, &mut end);
        if end_time < 0 as c_int || *end.offset(0 as c_int as isize) as c_int != '\0' as i32 {
            return -(1 as c_int);
        }
    } else {
        return -(1 as c_int);
    }
    (*range).start = start_time;
    (*range).end = end_time;
    return 0 as c_int;
}

unsafe extern "C" fn print_help(mut program_name: *const c_char) {
    // gettext(
    println!(
        "Usage: {} -l LAT:LON -t DAY:NIGHT [OPTIONS...]

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

The neutral temperature is {}K. Using this value will not change the color\ntemperature of the display. Setting the color temperature to a value higher\nthan this results in more blue light, and setting a lower value will result in\nmore red light.

Default values:
  Daytime temperature: {}K
  Night temperature: {}K
  
Please report bugs to <{}>
",
        CStr::from_ptr(program_name).to_str().unwrap(),
        6500,
        6500,
        4500,
        "https://github.com/jonls/redshift/issues"
    );
}

unsafe extern "C" fn print_method_list(mut gamma_methods: *const gamma_method_t) {
    // gettext(
    println!("Available adjustment methods:");

    let mut i: c_int = 0 as c_int;
    while !((*gamma_methods.offset(i as isize)).name).is_null() {
        let name = (*gamma_methods.offset(i as isize)).name;
        let name = CStr::from_ptr(name).to_str().unwrap();
        println!("  {name}");
        i += 1;
    }

    println!(
        "
Specify colon-separated options with `-m METHOD:OPTIONS`
Try `-m METHOD:help' for help."
    );
}

unsafe extern "C" fn print_provider_list(mut location_providers: *const location_provider_t) {
    // gettext(
    println!("Available location providers:");

    let mut i: c_int = 0 as c_int;
    while !((*location_providers.offset(i as isize)).name).is_null() {
        let name = (*location_providers.offset(i as isize)).name;
        let name = CStr::from_ptr(name).to_str().unwrap();
        println!("  {name}");
        i += 1;
    }

    println!(
        "
Specify colon-separated options with`-l PROVIDER:OPTIONS'.
Try `-l PROVIDER:help' for help.
"
    );
}

unsafe extern "C" fn find_gamma_method(
    mut gamma_methods: *const gamma_method_t,
    mut name: *const c_char,
) -> *const gamma_method_t {
    let mut method: *const gamma_method_t = 0 as *const gamma_method_t;
    let mut i: c_int = 0 as c_int;
    while !((*gamma_methods.offset(i as isize)).name).is_null() {
        let mut m: *const gamma_method_t =
            &*gamma_methods.offset(i as isize) as *const gamma_method_t;
        if strcasecmp(name, (*m).name) == 0 as c_int {
            method = m;
            break;
        } else {
            i += 1;
            i;
        }
    }
    return method;
}

unsafe extern "C" fn find_location_provider(
    mut location_providers: *const location_provider_t,
    mut name: *const c_char,
) -> *const location_provider_t {
    let mut provider: *const location_provider_t = 0 as *const location_provider_t;
    let mut i: c_int = 0 as c_int;
    while !((*location_providers.offset(i as isize)).name).is_null() {
        let mut p: *const location_provider_t =
            &*location_providers.offset(i as isize) as *const location_provider_t;
        if strcasecmp(name, (*p).name) == 0 as c_int {
            provider = p;
            break;
        } else {
            i += 1;
            i;
        }
    }
    return provider;
}

#[no_mangle]
pub unsafe extern "C" fn options_init(mut options: *mut options_t) {
    (*options).config_filepath = 0 as *mut c_char;
    (*options).scheme.high = 3.0f64;
    (*options).scheme.low = -6.0f64;
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
    (*options).temp_set = -(1 as c_int);
    (*options).method = 0 as *const gamma_method_t;
    (*options).method_args = 0 as *mut c_char;
    (*options).provider = 0 as *const location_provider_t;
    (*options).provider_args = 0 as *mut c_char;
    (*options).use_fade = -(1 as c_int);
    (*options).preserve_gamma = 1 as c_int;
    (*options).mode = PROGRAM_MODE_CONTINUAL;
    (*options).verbose = 0 as c_int;
}

unsafe extern "C" fn parse_command_line_option(
    option: c_char,
    mut value: *mut c_char,
    mut options: *mut options_t,
    mut program_name: *const c_char,
    mut gamma_methods: *const gamma_method_t,
    mut location_providers: *const location_provider_t,
) -> c_int {
    let mut r: c_int = 0;
    let mut s: *mut c_char = 0 as *mut c_char;
    let mut provider_name: *mut c_char = 0 as *mut c_char;
    let mut end: *mut c_char = 0 as *mut c_char;
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
            if strcasecmp(value, b"list\0" as *const u8 as *const c_char) == 0 as c_int {
                print_provider_list(location_providers);
                exit(0 as c_int);
            }
            provider_name = 0 as *mut c_char;
            *__errno_location() = 0 as c_int;
            end = 0 as *mut c_char;
            strtof(value, &mut end);
            if *__errno_location() == 0 as c_int && *end as c_int == ':' as i32 {
                provider_name = b"manual\0" as *const u8 as *const c_char as *mut c_char;
                (*options).provider_args = value;
            } else {
                s = strchr(value, ':' as i32);
                if !s.is_null() {
                    let fresh5 = s;
                    s = s.offset(1);
                    *fresh5 = '\0' as i32 as c_char;
                    (*options).provider_args = s;
                }
                provider_name = value;
            }
            (*options).provider = find_location_provider(location_providers, provider_name);
            if ((*options).provider).is_null() {
                eprintln!(
                    "Unknown location provider `{}`.",
                    CStr::from_ptr(provider_name).to_str().unwrap()
                );
                return -(1 as c_int);
            }
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
            if strcasecmp(value, b"list\0" as *const u8 as *const c_char) == 0 as c_int {
                print_method_list(gamma_methods);
                exit(0 as c_int);
            }
            s = strchr(value, ':' as i32);
            if !s.is_null() {
                let fresh6 = s;
                s = s.offset(1);
                *fresh6 = '\0' as i32 as c_char;
                (*options).method_args = s;
            }
            (*options).method = find_gamma_method(gamma_methods, value);
            if ((*options).method).is_null() {
                eprintln!(
                    "Unknown adjustment method `{}`",
                    CStr::from_ptr(value).to_str().unwrap()
                );
                return -(1 as c_int);
            }
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
    return 0 as c_int;
}

#[no_mangle]
pub unsafe extern "C" fn options_parse_args(
    mut options: *mut options_t,
    mut argc: c_int,
    mut argv: *mut *mut c_char,
    mut gamma_methods: *const gamma_method_t,
    mut location_providers: *const location_provider_t,
) {
    let mut program_name: *const c_char = *argv.offset(0 as c_int as isize);
    let mut opt: c_int = 0;
    loop {
        opt = getopt(
            argc,
            argv as *const *mut c_char,
            b"b:c:g:hl:m:oO:pPrt:vVx\0" as *const u8 as *const c_char,
        );
        if !(opt != -(1 as c_int)) {
            break;
        }
        let mut option: c_char = opt as c_char;
        let mut r: c_int = parse_command_line_option(
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

unsafe extern "C" fn parse_config_file_option(
    mut key: *const c_char,
    mut value: *const c_char,
    mut options: *mut options_t,
    mut gamma_methods: *const gamma_method_t,
    mut location_providers: *const location_provider_t,
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
            let mut r: c_int =
                parse_gamma_string(value, ((*options).scheme.day.gamma).as_mut_ptr());
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
            let mut r_0: c_int =
                parse_gamma_string(value, ((*options).scheme.day.gamma).as_mut_ptr());
            if r_0 < 0 as c_int {
                eprintln!("Malformed gamma setting.");
                return -(1 as c_int);
            }
        }
    } else if strcasecmp(key, b"gamma-night\0" as *const u8 as *const c_char) == 0 as c_int {
        if ((*options).scheme.night.gamma[0 as c_int as usize]).is_nan() as i32 != 0 {
            let mut r_1: c_int =
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
            let mut r_2: c_int = parse_transition_range(value, &mut (*options).scheme.dawn);
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
            let mut r_3: c_int = parse_transition_range(value, &mut (*options).scheme.dusk);
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
    return 0 as c_int;
}

#[no_mangle]
pub unsafe extern "C" fn options_parse_config_file(
    mut options: *mut options_t,
    mut config_state: *mut config_ini_state_t,
    mut gamma_methods: *const gamma_method_t,
    mut location_providers: *const location_provider_t,
) {
    let mut section: *mut config_ini_section_t =
        config_ini_get_section(config_state, b"redshift\0" as *const u8 as *const c_char);
    if section.is_null() {
        return;
    }
    let mut setting: *mut config_ini_setting_t = (*section).settings;
    while !setting.is_null() {
        let mut r: c_int = parse_config_file_option(
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

#[no_mangle]
pub unsafe extern "C" fn options_set_defaults(mut options: *mut options_t) {
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
