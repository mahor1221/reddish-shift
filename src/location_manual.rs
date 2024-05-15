/*  location-manual.rs -- Manual location provider source
    This file is part of <https://github.com/mahor1221/reddish-shift>.
    Copyright (C) 2024 Mahor Foruzesh <mahor1221@gmail.com>
    Ported from Redshift <https://github.com/jonls/redshift>.
    Copyright (c) 2010-2017  Jon Lund Steffensen <jonlst@gmail.com>

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
    location_provider_free_func, location_provider_get_fd_func, location_provider_handle_func,
    location_provider_init_func, location_provider_print_help_func,
    location_provider_set_option_func, location_provider_start_func, location_provider_t,
    location_t,
};
use libc::{__errno_location, exit, fputs, free, malloc, strcasecmp, strtof, FILE};
use std::ffi::{c_char, c_float, c_int, c_void};

#[derive(Copy, Clone)]
#[repr(C)]
pub struct location_manual_state_t {
    pub loc: location_t,
}

unsafe extern "C" fn location_manual_init(state: *mut *mut location_manual_state_t) -> c_int {
    *state =
        malloc(::core::mem::size_of::<location_manual_state_t>()) as *mut location_manual_state_t;
    if (*state).is_null() {
        return -(1 as c_int);
    }
    let s: *mut location_manual_state_t = *state;
    (*s).loc.lat = ::core::f32::NAN;
    (*s).loc.lon = ::core::f32::NAN;
    0 as c_int
}

unsafe extern "C" fn location_manual_start(state: *mut location_manual_state_t) -> c_int {
    // Latitude and longitude must be set
    if ((*state).loc.lat).is_nan() as i32 != 0 || ((*state).loc.lon).is_nan() as i32 != 0 {
        // gettext(
        eprintln!("Latitude and longitude must be set.");
        exit(1 as c_int);
    }
    0 as c_int
}

unsafe extern "C" fn location_manual_free(state: *mut location_manual_state_t) {
    free(state as *mut c_void);
}

unsafe extern "C" fn location_manual_print_help(f: *mut FILE) {
    fputs(
        // gettext(
        b"Specify location manually.\n\0" as *const u8 as *const c_char,
        f,
    );
    fputs(b"\n\0" as *const u8 as *const c_char, f);
    // TRANSLATORS: Manual location help output
    // left column must not be translated
    fputs(
        // gettext(
        b"  lat=N\t\tLatitude\n  lon=N\t\tLongitude\n\0" as *const u8 as *const c_char,
        f,
    );
    fputs(b"\n\0" as *const u8 as *const c_char, f);
    fputs(
        // gettext(
            b"Both values are expected to be floating point numbers,\nnegative values representing west / south, respectively.\n\0"
                as *const u8 as *const c_char,
        f,
    );
    fputs(b"\n\0" as *const u8 as *const c_char, f);
}

unsafe extern "C" fn location_manual_set_option(
    state: *mut location_manual_state_t,
    key: *const c_char,
    value: *const c_char,
) -> c_int {
    /* Parse float value */
    let mut end: *mut c_char = std::ptr::null_mut::<c_char>();
    *__errno_location() = 0 as c_int;
    let v: c_float = strtof(value, &mut end);
    if *__errno_location() != 0 as c_int || *end as c_int != '\0' as i32 {
        // gettext(
        eprintln!("Malformed argument.");
        return -(1 as c_int);
    }
    if strcasecmp(key, b"lat\0" as *const u8 as *const c_char) == 0 as c_int {
        (*state).loc.lat = v;
    } else if strcasecmp(key, b"lon\0" as *const u8 as *const c_char) == 0 as c_int {
        (*state).loc.lon = v;
    } else {
        // gettext(
        eprintln!("Unknown method parameter: `{}`.", *key);
        return -(1 as c_int);
    }
    0 as c_int
}

unsafe extern "C" fn location_manual_get_fd(state: *mut location_manual_state_t) -> c_int {
    -(1 as c_int)
}

unsafe extern "C" fn location_manual_handle(
    state: *mut location_manual_state_t,
    location: *mut location_t,
    available: *mut c_int,
) -> c_int {
    *location = (*state).loc;
    *available = 1 as c_int;
    0 as c_int
}

#[no_mangle]
pub static mut manual_location_provider: location_provider_t = unsafe {
    {
        location_provider_t {
            name: b"manual\0" as *const u8 as *const c_char as *mut c_char,
            init: ::core::mem::transmute::<
                Option<unsafe extern "C" fn(*mut *mut location_manual_state_t) -> c_int>,
                Option<location_provider_init_func>,
            >(Some(
                location_manual_init
                    as unsafe extern "C" fn(*mut *mut location_manual_state_t) -> c_int,
            )),
            start: ::core::mem::transmute::<
                Option<unsafe extern "C" fn(*mut location_manual_state_t) -> c_int>,
                Option<location_provider_start_func>,
            >(Some(
                location_manual_start
                    as unsafe extern "C" fn(*mut location_manual_state_t) -> c_int,
            )),
            free: ::core::mem::transmute::<
                Option<unsafe extern "C" fn(*mut location_manual_state_t) -> ()>,
                Option<location_provider_free_func>,
            >(Some(
                location_manual_free as unsafe extern "C" fn(*mut location_manual_state_t) -> (),
            )),
            print_help: ::core::mem::transmute::<
                Option<unsafe extern "C" fn(*mut FILE) -> ()>,
                Option<location_provider_print_help_func>,
            >(Some(
                location_manual_print_help as unsafe extern "C" fn(*mut FILE) -> (),
            )),
            set_option: ::core::mem::transmute::<
                Option<
                    unsafe extern "C" fn(
                        *mut location_manual_state_t,
                        *const c_char,
                        *const c_char,
                    ) -> c_int,
                >,
                Option<location_provider_set_option_func>,
            >(Some(
                location_manual_set_option
                    as unsafe extern "C" fn(
                        *mut location_manual_state_t,
                        *const c_char,
                        *const c_char,
                    ) -> c_int,
            )),
            get_fd: ::core::mem::transmute::<
                Option<unsafe extern "C" fn(*mut location_manual_state_t) -> c_int>,
                Option<location_provider_get_fd_func>,
            >(Some(
                location_manual_get_fd
                    as unsafe extern "C" fn(*mut location_manual_state_t) -> c_int,
            )),
            handle: ::core::mem::transmute::<
                Option<
                    unsafe extern "C" fn(
                        *mut location_manual_state_t,
                        *mut location_t,
                        *mut c_int,
                    ) -> c_int,
                >,
                Option<location_provider_handle_func>,
            >(Some(
                location_manual_handle
                    as unsafe extern "C" fn(
                        *mut location_manual_state_t,
                        *mut location_t,
                        *mut c_int,
                    ) -> c_int,
            )),
        }
    }
};
