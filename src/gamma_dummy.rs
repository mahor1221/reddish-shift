/*  gamma-dummy.rs -- No-op gamma adjustment
    This file is part of <https://github.com/mahor1221/reddish-shift>.
    Copyright (C) 2024 Mahor Foruzesh <mahor1221@gmail.com>
    Ported from Redshift <https://github.com/jonls/redshift>.
    Copyright (c) 2013-2017  Jon Lund Steffensen <jonlst@gmail.com>

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
    gamma_method_free_func, gamma_method_init_func, gamma_method_print_help_func,
    gamma_method_restore_func, gamma_method_set_option_func, gamma_method_set_temperature_func,
    gamma_method_start_func, gamma_method_t, ColorSetting,
};
use libc::{fputs, FILE};
use std::ffi::{c_char, c_int, c_void};

unsafe extern "C" fn gamma_dummy_init(state: *mut *mut c_void) -> c_int {
    *state = std::ptr::null_mut::<c_void>();
    0 as c_int
}

unsafe extern "C" fn gamma_dummy_start(state: *mut c_void) -> c_int {
    // gettext(
    eprintln!(
        "WARNING: Using dummy gamma method! Display will not be affected by this gamma method."
    );
    0 as c_int
}

unsafe extern "C" fn gamma_dummy_restore(state: *mut c_void) {}

unsafe extern "C" fn gamma_dummy_free(state: *mut c_void) {}

unsafe extern "C" fn gamma_dummy_print_help(f: *mut FILE) {
    fputs(
        // gettext(
        b"Does not affect the display but prints the color temperature to the terminal.\n\0"
            as *const u8 as *const c_char,
        // ),
        f,
    );
    fputs(b"\n\0" as *const u8 as *const c_char, f);
}

unsafe extern "C" fn gamma_dummy_set_option(
    state: *mut c_void,
    key: *const c_char,
    value: *const c_char,
) -> c_int {
    // gettext(
    eprintln!("Unknown method parameter: `{}`", *key);
    -(1 as c_int)
}

unsafe extern "C" fn gamma_dummy_set_temperature(
    state: *mut c_void,
    setting: *const ColorSetting,
    preserve: c_int,
) -> c_int {
    // gettext(
    println!("Temperature: {}", (*setting).temperature);
    0 as c_int
}

#[no_mangle]
pub static mut dummy_gamma_method: gamma_method_t = unsafe {
    {
        gamma_method_t {
            name: b"dummy\0" as *const u8 as *const c_char as *mut c_char,
            autostart: 0 as c_int,
            init: ::core::mem::transmute::<
                Option<unsafe extern "C" fn(*mut *mut c_void) -> c_int>,
                Option<gamma_method_init_func>,
            >(Some(
                gamma_dummy_init as unsafe extern "C" fn(*mut *mut c_void) -> c_int,
            )),
            start: ::core::mem::transmute::<
                Option<unsafe extern "C" fn(*mut c_void) -> c_int>,
                Option<gamma_method_start_func>,
            >(Some(
                gamma_dummy_start as unsafe extern "C" fn(*mut c_void) -> c_int,
            )),
            free: ::core::mem::transmute::<
                Option<unsafe extern "C" fn(*mut c_void) -> ()>,
                Option<gamma_method_free_func>,
            >(Some(
                gamma_dummy_free as unsafe extern "C" fn(*mut c_void) -> (),
            )),
            print_help: ::core::mem::transmute::<
                Option<unsafe extern "C" fn(*mut FILE) -> ()>,
                Option<gamma_method_print_help_func>,
            >(Some(
                gamma_dummy_print_help as unsafe extern "C" fn(*mut FILE) -> (),
            )),
            set_option: ::core::mem::transmute::<
                Option<unsafe extern "C" fn(*mut c_void, *const c_char, *const c_char) -> c_int>,
                Option<gamma_method_set_option_func>,
            >(Some(
                gamma_dummy_set_option
                    as unsafe extern "C" fn(*mut c_void, *const c_char, *const c_char) -> c_int,
            )),
            restore: ::core::mem::transmute::<
                Option<unsafe extern "C" fn(*mut c_void) -> ()>,
                Option<gamma_method_restore_func>,
            >(Some(
                gamma_dummy_restore as unsafe extern "C" fn(*mut c_void) -> (),
            )),
            set_temperature: ::core::mem::transmute::<
                Option<unsafe extern "C" fn(*mut c_void, *const ColorSetting, c_int) -> c_int>,
                Option<gamma_method_set_temperature_func>,
            >(Some(
                gamma_dummy_set_temperature
                    as unsafe extern "C" fn(*mut c_void, *const ColorSetting, c_int) -> c_int,
            )),
        }
    }
};
