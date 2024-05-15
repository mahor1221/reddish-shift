/*  gamma-vidmode.rs -- X VidMode gamma adjustment source
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
    colorramp::colorramp_fill, gamma_method_free_func, gamma_method_init_func,
    gamma_method_print_help_func, gamma_method_restore_func, gamma_method_set_option_func,
    gamma_method_set_temperature_func, gamma_method_start_func, gamma_method_t, ColorSetting,
};
use libc::{atoi, fprintf, fputs, free, malloc, memcpy, perror, size_t, strcasecmp, FILE};
use std::{
    cell::OnceCell,
    ffi::{c_char, c_double, c_int, c_void},
    sync::OnceLock,
};
use x11_dl::{
    xf86vmode::Xf86vmode,
    xlib::{Display, Xlib},
};

#[derive(Copy, Clone)]
#[repr(C)]
pub struct vidmode_state_t {
    pub display: *mut Display,
    pub screen_num: c_int,
    pub ramp_size: c_int,
    pub saved_ramps: *mut u16,
}

static XLIB: OnceLock<Xlib> = OnceLock::new();
static XF86VMODE: OnceLock<Xf86vmode> = OnceLock::new();

fn xlib() -> &'static Xlib {
    XLIB.get_or_init(|| Xlib::open().unwrap())
}

fn xf86vmode() -> &'static Xf86vmode {
    XF86VMODE.get_or_init(|| Xf86vmode::open().unwrap())
}

unsafe extern "C" fn vidmode_init(mut state: *mut *mut vidmode_state_t) -> c_int {
    *state = malloc(::core::mem::size_of::<vidmode_state_t>() as size_t) as *mut vidmode_state_t;
    if (*state).is_null() {
        return -(1 as c_int);
    }
    let mut s: *mut vidmode_state_t = *state;
    (*s).screen_num = -(1 as c_int);
    (*s).saved_ramps = 0 as *mut u16;
    // Open display
    (*s).display = (xlib().XOpenDisplay)(0 as *const c_char);
    if ((*s).display).is_null() {
        // fprintf(
        //     stderr,
        //     gettext(b"X request failed: %s\n\0" as *const u8 as *const c_char),
        //     b"XOpenDisplay\0" as *const u8 as *const c_char,
        // );
        eprintln!("X request failed: {}", "XOpenDisplay");
        return -(1 as c_int);
    }
    return 0 as c_int;
}

unsafe extern "C" fn vidmode_start(mut state: *mut vidmode_state_t) -> c_int {
    let mut r: c_int = 0;
    let mut screen_num: c_int = (*state).screen_num;
    if screen_num < 0 as c_int {
        screen_num = (xlib().XDefaultScreen)((*state).display);
    }
    (*state).screen_num = screen_num;
    // Query extension version
    let mut major: c_int = 0;
    let mut minor: c_int = 0;
    r = (xf86vmode().XF86VidModeQueryVersion)((*state).display, &mut major, &mut minor);
    if r == 0 {
        eprintln!("X request failed: {}", "XF86VidModeQueryVersion");
        return -(1 as c_int);
    }
    // Request size of gamma ramps
    r = (xf86vmode().XF86VidModeGetGammaRampSize)(
        (*state).display,
        (*state).screen_num,
        &mut (*state).ramp_size,
    );
    if r == 0 {
        eprintln!("X request failed: {}", "XF86VidModeGetGammaRampSize");
        return -(1 as c_int);
    }
    if (*state).ramp_size == 0 as c_int {
        eprintln!("Gamma ramp size too small: {}", (*state).ramp_size);
        return -(1 as c_int);
    }
    // Allocate space for saved gamma ramps
    (*state).saved_ramps = malloc(
        ((3 * (*state).ramp_size) as usize).wrapping_mul(::core::mem::size_of::<u16>() as usize),
    ) as *mut u16;
    if ((*state).saved_ramps).is_null() {
        perror(b"malloc\0" as *const u8 as *const c_char);
        return -(1 as c_int);
    }
    let mut gamma_r: *mut u16 =
        &mut *((*state).saved_ramps).offset((0 as c_int * (*state).ramp_size) as isize) as *mut u16;
    let mut gamma_g: *mut u16 =
        &mut *((*state).saved_ramps).offset((1 as c_int * (*state).ramp_size) as isize) as *mut u16;
    let mut gamma_b: *mut u16 =
        &mut *((*state).saved_ramps).offset((2 as c_int * (*state).ramp_size) as isize) as *mut u16;
    // Save current gamma ramps so we can restore them at program exit.
    r = (xf86vmode().XF86VidModeGetGammaRamp)(
        (*state).display,
        (*state).screen_num,
        (*state).ramp_size,
        gamma_r,
        gamma_g,
        gamma_b,
    );
    if r == 0 {
        eprintln!("X request failed: {}", "XF86VidModeGetGammaRamp");
        return -(1 as c_int);
    }
    return 0 as c_int;
}

unsafe extern "C" fn vidmode_free(mut state: *mut vidmode_state_t) {
    // Free saved ramps
    free((*state).saved_ramps as *mut c_void);
    // Close display connection
    (xlib().XCloseDisplay)((*state).display);
    free(state as *mut c_void);
}

unsafe extern "C" fn vidmode_print_help(mut f: *mut FILE) {
    fputs(
        // gettext(
        b"Adjust gamma ramps with the X VidMode extension.\n\0" as *const u8 as *const c_char,
        // ),
        f,
    );
    // TRANSLATORS: VidMode help output
    // left column must not be translated
    fputs(b"\n\0" as *const u8 as *const c_char, f);
    fputs(
        // gettext(
        b"  screen=N\t\tX screen to apply adjustments to\n\0" as *const u8 as *const c_char,
        // ),
        f,
    );
    fputs(b"\n\0" as *const u8 as *const c_char, f);
}

unsafe extern "C" fn vidmode_set_option(
    mut state: *mut vidmode_state_t,
    mut key: *const c_char,
    mut value: *const c_char,
) -> c_int {
    if strcasecmp(key, b"screen\0" as *const u8 as *const c_char) == 0 as c_int {
        (*state).screen_num = atoi(value);
    } else if strcasecmp(key, b"preserve\0" as *const u8 as *const c_char) == 0 as c_int {
        eprintln!(
            "Parameter `{}` is now always on;  Use the `{}` command-line option to disable.",
            *key, "-P"
        );
    } else {
        eprintln!("Unknown method parameter: `{}`", *key);
        return -(1 as c_int);
    }
    return 0 as c_int;
}

unsafe extern "C" fn vidmode_restore(mut state: *mut vidmode_state_t) {
    let mut gamma_r: *mut u16 =
        &mut *((*state).saved_ramps).offset((0 as c_int * (*state).ramp_size) as isize) as *mut u16;
    let mut gamma_g: *mut u16 =
        &mut *((*state).saved_ramps).offset((1 as c_int * (*state).ramp_size) as isize) as *mut u16;
    let mut gamma_b: *mut u16 =
        &mut *((*state).saved_ramps).offset((2 as c_int * (*state).ramp_size) as isize) as *mut u16;
    // Restore gamma ramps
    let mut r: c_int = (xf86vmode().XF86VidModeSetGammaRamp)(
        (*state).display,
        (*state).screen_num,
        (*state).ramp_size,
        gamma_r,
        gamma_g,
        gamma_b,
    );
    if r == 0 {
        eprintln!("X request failed: {}", "XF86VidModeSetGammaRamp")
    }
}

unsafe extern "C" fn vidmode_set_temperature(
    mut state: *mut vidmode_state_t,
    mut setting: *const ColorSetting,
    mut preserve: c_int,
) -> c_int {
    let mut r: c_int = 0;
    // Create new gamma ramps
    let mut gamma_ramps: *mut u16 = malloc(
        ((3 as c_int * (*state).ramp_size) as usize).wrapping_mul(::core::mem::size_of::<u16>()),
    ) as *mut u16;
    if gamma_ramps.is_null() {
        perror(b"malloc\0" as *const u8 as *const c_char);
        return -(1 as c_int);
    }
    let mut gamma_r: *mut u16 =
        &mut *gamma_ramps.offset((0 as c_int * (*state).ramp_size) as isize) as *mut u16;
    let mut gamma_g: *mut u16 =
        &mut *gamma_ramps.offset((1 as c_int * (*state).ramp_size) as isize) as *mut u16;
    let mut gamma_b: *mut u16 =
        &mut *gamma_ramps.offset((2 as c_int * (*state).ramp_size) as isize) as *mut u16;
    if preserve != 0 {
        // Initialize gamma ramps from saved state
        memcpy(
            gamma_ramps as *mut c_void,
            (*state).saved_ramps as *const c_void,
            ((3 as c_int * (*state).ramp_size) as usize)
                .wrapping_mul(::core::mem::size_of::<u16>()),
        );
    } else {
        // Initialize gamma ramps to pure state
        let mut i: c_int = 0 as c_int;
        while i < (*state).ramp_size {
            let mut value: u16 = (i as c_double / (*state).ramp_size as c_double
                * (65535 as c_int + 1 as c_int) as c_double)
                as u16;
            *gamma_r.offset(i as isize) = value;
            *gamma_g.offset(i as isize) = value;
            *gamma_b.offset(i as isize) = value;
            i += 1;
            i;
        }
    }

    let r = std::slice::from_raw_parts_mut(gamma_r, (*state).ramp_size as usize);
    let g = std::slice::from_raw_parts_mut(gamma_g, (*state).ramp_size as usize);
    let b = std::slice::from_raw_parts_mut(gamma_b, (*state).ramp_size as usize);
    colorramp_fill(r, g, b, &*setting);

    // Set new gamma ramps
    let res = (xf86vmode().XF86VidModeSetGammaRamp)(
        (*state).display,
        (*state).screen_num,
        (*state).ramp_size,
        gamma_r,
        gamma_g,
        gamma_b,
    );

    if res == 0 {
        eprintln!("X request failed: {}", "XF86VidModeSetGammaRamp");
        free(gamma_ramps as *mut c_void);
        return -(1 as c_int);
    }
    free(gamma_ramps as *mut c_void);
    return 0 as c_int;
}

#[no_mangle]
pub static mut vidmode_gamma_method: gamma_method_t = unsafe {
    {
        let mut init = gamma_method_t {
            name: b"vidmode\0" as *const u8 as *const c_char as *mut c_char,
            autostart: 1 as c_int,
            init: ::core::mem::transmute::<
                Option<unsafe extern "C" fn(*mut *mut vidmode_state_t) -> c_int>,
                Option<gamma_method_init_func>,
            >(Some(
                vidmode_init as unsafe extern "C" fn(*mut *mut vidmode_state_t) -> c_int,
            )),
            start: ::core::mem::transmute::<
                Option<unsafe extern "C" fn(*mut vidmode_state_t) -> c_int>,
                Option<gamma_method_start_func>,
            >(Some(
                vidmode_start as unsafe extern "C" fn(*mut vidmode_state_t) -> c_int,
            )),
            free: ::core::mem::transmute::<
                Option<unsafe extern "C" fn(*mut vidmode_state_t) -> ()>,
                Option<gamma_method_free_func>,
            >(Some(
                vidmode_free as unsafe extern "C" fn(*mut vidmode_state_t) -> (),
            )),
            print_help: ::core::mem::transmute::<
                Option<unsafe extern "C" fn(*mut FILE) -> ()>,
                Option<gamma_method_print_help_func>,
            >(Some(
                vidmode_print_help as unsafe extern "C" fn(*mut FILE) -> (),
            )),
            set_option: ::core::mem::transmute::<
                Option<
                    unsafe extern "C" fn(
                        *mut vidmode_state_t,
                        *const c_char,
                        *const c_char,
                    ) -> c_int,
                >,
                Option<gamma_method_set_option_func>,
            >(Some(
                vidmode_set_option
                    as unsafe extern "C" fn(
                        *mut vidmode_state_t,
                        *const c_char,
                        *const c_char,
                    ) -> c_int,
            )),
            restore: ::core::mem::transmute::<
                Option<unsafe extern "C" fn(*mut vidmode_state_t) -> ()>,
                Option<gamma_method_restore_func>,
            >(Some(
                vidmode_restore as unsafe extern "C" fn(*mut vidmode_state_t) -> (),
            )),
            set_temperature: ::core::mem::transmute::<
                Option<
                    unsafe extern "C" fn(*mut vidmode_state_t, *const ColorSetting, c_int) -> c_int,
                >,
                Option<gamma_method_set_temperature_func>,
            >(Some(
                vidmode_set_temperature
                    as unsafe extern "C" fn(
                        *mut vidmode_state_t,
                        *const ColorSetting,
                        c_int,
                    ) -> c_int,
            )),
        };
        init
    }
};
