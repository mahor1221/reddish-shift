/*  gamma-randr.rs -- X RANDR gamma adjustment
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

use crate::{colorramp::colorramp_fill, config::ColorSettings, Adjuster};
use libc::{
    __errno_location, atoi, calloc, fputs, free, malloc, memcpy, perror,
    strcasecmp, strtol, FILE,
};
use std::ffi::{c_char, c_double, c_int, c_uint, c_void};
use xcb::{
    ffi::xcb_generic_error_t,
    randr::{
        Crtc, GetCrtcGamma, GetCrtcGammaSize, GetScreenResourcesCurrent,
        QueryVersion, SetCrtcGamma,
    },
    x::Screen,
    Connection,
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Randr {
    screen: usize,
}

impl Randr {
    pub fn new(screen: usize) -> Self {
        Self { screen }
    }
}

impl Adjuster for Randr {}

// #[derive(Copy, Clone)]
#[repr(C)]
pub struct randr_state_t<'a> {
    pub conn: Connection,
    pub screen: Option<&'a Screen>,
    pub preferred_screen: c_int,
    pub screen_num: c_int,
    pub crtc_num_count: c_int,
    pub crtc_num: *mut c_int,
    pub crtc_count: c_uint,
    pub crtcs: *mut randr_crtc_state_t,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct randr_crtc_state_t {
    pub crtc: Crtc,
    pub ramp_size: c_uint,
    pub saved_ramps: *mut u16,
}

unsafe extern "C" fn randr_init(state: *mut *mut randr_state_t) -> c_int {
    // Initialize state.
    *state =
        malloc(::core::mem::size_of::<randr_state_t>()) as *mut randr_state_t;
    if (*state).is_null() {
        return -(1 as c_int);
    }
    let s: *mut randr_state_t = *state;
    (*s).screen_num = -(1 as c_int);
    (*s).crtc_num = std::ptr::null_mut::<c_int>();
    (*s).crtc_num_count = 0 as c_int;
    (*s).crtc_count = 0 as c_int as c_uint;
    (*s).crtcs = std::ptr::null_mut::<randr_crtc_state_t>();
    let error: *mut xcb_generic_error_t =
        std::ptr::null_mut::<xcb_generic_error_t>();

    // (*s).conn = xcb_connect(0 as *const c_char, &mut (*s).preferred_screen);
    // let mut ver_cookie: xcb_randr_query_version_cookie_t =
    //     xcb_randr_query_version((*s).conn, 1 as c_int as u32, 3 as c_int as u32);
    // let mut ver_reply: *mut xcb_randr_query_version_reply_t =
    //     xcb_randr_query_version_reply((*s).conn, ver_cookie, &mut error);

    // TODO: Error handling
    // if !error.is_null() || ver_reply.is_null() {
    //     let mut ec: c_int = if !error.is_null() {
    //         (*error).error_code as c_int
    //     } else {
    //         -(1 as c_int)
    //     };

    //     // gettext(
    //     eprintln!("`{}` returned error {}", "RANDR Query Version", ec);
    //     // xcb_disconnect((*s).conn);
    //     free(s as *mut c_void);
    //     return -(1 as c_int);
    // }

    // TODO: Error handling
    // Open X server connection
    ((*s).conn, (*s).preferred_screen) = Connection::connect(None).unwrap();
    // Query RandR version
    let ver_cookie = (*s).conn.send_request(&QueryVersion {
        major_version: 1,
        minor_version: 3,
    });
    let ver_reply = (*s).conn.wait_for_reply(ver_cookie).unwrap();

    let major_version = ver_reply.major_version();
    let minor_version = ver_reply.minor_version();

    // TODO What does it mean when both error and ver_reply is NULL?
    //      Apparently, we have to check both to avoid seg faults.
    if major_version != 1 as c_int as u32 || minor_version < 3 as c_int as u32 {
        // gettext(
        eprintln!(
            "Unsupported RANDR version ({}.{})",
            major_version, minor_version
        );
        // free(ver_reply as *mut c_void);
        // xcb_disconnect((*s).conn);
        free(s as *mut c_void);
        return -(1 as c_int);
    }
    // free(ver_reply as *mut c_void);
    0 as c_int
}

unsafe extern "C" fn randr_start(state: *mut randr_state_t) -> c_int {
    let error: *mut xcb_generic_error_t =
        std::ptr::null_mut::<xcb_generic_error_t>();
    let mut screen_num: c_int = (*state).screen_num;
    if screen_num < 0 as c_int {
        screen_num = (*state).preferred_screen;
    }

    // let mut setup: *const xcb_setup_t = xcb_get_setup((*state).conn);
    // let mut iter: xcb_screen_iterator_t = xcb_setup_roots_iterator(setup);

    // Get screen
    let screens = (*state).conn.get_setup().roots();
    (*state).screen = None;

    let mut i: c_int = 0 as c_int;
    for screen in screens {
        if i == screen_num {
            (*state).screen = Some(screen);
            break;
        } else {
            i += 1;
        }
    }
    if ((*state).screen).is_none() {
        // gettext(
        eprintln!("Screen {} could not be found.", screen_num);
        return -(1 as c_int);
    }

    // TODO: Error handling
    // let mut res_cookie: xcb_randr_get_screen_resources_current_cookie_t =
    //     xcb_randr_get_screen_resources_current((*state).conn);
    // let mut res_reply: *mut xcb_randr_get_screen_resources_current_reply_t =
    //     xcb_randr_get_screen_resources_current_reply((*state).conn, res_cookie, &mut error);
    // if !error.is_null() {
    //     fprintf(
    //         stderr,
    //         gettext(b"`%s' returned error %d\n\0" as *const u8 as *const c_char),
    //         b"RANDR Get Screen Resources Current\0" as *const u8 as *const c_char,
    //         (*error).error_code as c_int,
    //     );
    //     return -(1 as c_int);
    // }

    // TODO: Error handling
    // Get list of CRTCs for the screen
    let res_cookie = (*state).conn.send_request(&GetScreenResourcesCurrent {
        window: (*state).screen.unwrap().root(),
    });
    let res_reply = (*state).conn.wait_for_reply(res_cookie).unwrap();

    (*state).crtc_count = res_reply.crtcs().len() as c_uint;
    (*state).crtcs = calloc(
        (*state).crtc_count as usize,
        ::core::mem::size_of::<randr_crtc_state_t>(),
    ) as *mut randr_crtc_state_t;
    if ((*state).crtcs).is_null() {
        perror(b"malloc\0" as *const u8 as *const c_char);
        (*state).crtc_count = 0 as c_int as c_uint;
        return -(1 as c_int);
    }

    let crtcs: *const Crtc = res_reply.crtcs().as_ptr();
    // Save CRTC identifier in state
    let mut i_0: c_int = 0 as c_int;
    while (i_0 as c_uint) < (*state).crtc_count {
        (*((*state).crtcs).offset(i_0 as isize)).crtc =
            *crtcs.offset(i_0 as isize);
        i_0 += 1;
        i_0;
    }
    // free(res_reply as *mut c_void);

    // Save size and gamma ramps of all CRTCs.
    // Current gamma ramps are saved so we can restore them
    // at program exit.
    let mut i_1: c_int = 0 as c_int;
    while (i_1 as c_uint) < (*state).crtc_count {
        let crtc = (*((*state).crtcs).offset(i_1 as isize)).crtc;

        // TODO: Error handling
        // let mut gamma_size_cookie: xcb_randr_get_crtc_gamma_size_cookie_t =
        //     xcb_randr_get_crtc_gamma_size((*state).conn, crtc);
        // let mut gamma_size_reply: *mut xcb_randr_get_crtc_gamma_size_reply_t =
        //     xcb_randr_get_crtc_gamma_size_reply((*state).conn, gamma_size_cookie, &mut error);
        // if !error.is_null() {
        //     fprintf(
        //         stderr,
        //         gettext(b"`%s' returned error %d\n\0" as *const u8 as *const c_char),
        //         b"RANDR Get CRTC Gamma Size\0" as *const u8 as *const c_char,
        //         (*error).error_code as c_int,
        //     );
        //     return -(1 as c_int);
        // }

        // TODO: Error handling
        // Request size of gamma ramps
        let gamma_size_cookie =
            (*state).conn.send_request(&GetCrtcGammaSize { crtc });
        let gamma_size_reply =
            (*state).conn.wait_for_reply(gamma_size_cookie).unwrap();

        let ramp_size = gamma_size_reply.size() as u32;
        (*((*state).crtcs).offset(i_1 as isize)).ramp_size = ramp_size;
        // free(gamma_size_reply as *mut c_void);
        if ramp_size == 0 as c_int as c_uint {
            // gettext(
            eprintln!("Gamma ramp size too small: {}", ramp_size,);
            return -(1 as c_int);
        }

        // let mut gamma_get_cookie: xcb_randr_get_crtc_gamma_cookie_t =
        //     xcb_randr_get_crtc_gamma((*state).conn, crtc);
        // let mut gamma_get_reply: *mut xcb_randr_get_crtc_gamma_reply_t =
        //     xcb_randr_get_crtc_gamma_reply((*state).conn, gamma_get_cookie, &mut error);
        // if !error.is_null() {
        //     fprintf(
        //         stderr,
        //         gettext(b"`%s' returned error %d\n\0" as *const u8 as *const c_char),
        //         b"RANDR Get CRTC Gamma\0" as *const u8 as *const c_char,
        //         (*error).error_code as c_int,
        //     );
        //     return -(1 as c_int);
        // }

        // TODO: Error handling
        // Request current gamma ramps
        let gamma_get_cookie =
            (*state).conn.send_request(&GetCrtcGamma { crtc });
        let gamma_get_reply =
            (*state).conn.wait_for_reply(gamma_get_cookie).unwrap();

        let gamma_r: *const u16 = gamma_get_reply.red().as_ptr();
        let gamma_g: *const u16 = gamma_get_reply.green().as_ptr();
        let gamma_b: *const u16 = gamma_get_reply.blue().as_ptr();

        // Allocate space for saved gamma ramps
        let fresh0 = &mut (*((*state).crtcs).offset(i_1 as isize)).saved_ramps;
        *fresh0 = malloc(
            ((3 as c_int as c_uint).wrapping_mul(ramp_size) as usize)
                .wrapping_mul(::core::mem::size_of::<u16>()),
        ) as *mut u16;
        if ((*((*state).crtcs).offset(i_1 as isize)).saved_ramps).is_null() {
            perror(b"malloc\0" as *const u8 as *const c_char);
            // free(gamma_get_reply as *mut c_void);
            return -(1 as c_int);
        }

        // Copy gamma ramps into CRTC state
        memcpy(
            &mut *((*((*state).crtcs).offset(i_1 as isize)).saved_ramps)
                .offset((0 as c_int as c_uint).wrapping_mul(ramp_size) as isize)
                as *mut u16 as *mut c_void,
            gamma_r as *const c_void,
            (ramp_size as usize).wrapping_mul(::core::mem::size_of::<u16>()),
        );
        memcpy(
            &mut *((*((*state).crtcs).offset(i_1 as isize)).saved_ramps)
                .offset((1 as c_int as c_uint).wrapping_mul(ramp_size) as isize)
                as *mut u16 as *mut c_void,
            gamma_g as *const c_void,
            (ramp_size as usize).wrapping_mul(::core::mem::size_of::<u16>()),
        );
        memcpy(
            &mut *((*((*state).crtcs).offset(i_1 as isize)).saved_ramps)
                .offset((2 as c_int as c_uint).wrapping_mul(ramp_size) as isize)
                as *mut u16 as *mut c_void,
            gamma_b as *const c_void,
            (ramp_size as usize).wrapping_mul(::core::mem::size_of::<u16>()),
        );
        // free(gamma_get_reply as *mut c_void);
        i_1 += 1;
        i_1;
    }
    0 as c_int
}

unsafe extern "C" fn randr_restore(state: *mut randr_state_t) {
    // Restore CRTC gamma ramps
    let error: *mut xcb_generic_error_t =
        std::ptr::null_mut::<xcb_generic_error_t>();
    let mut i: c_int = 0 as c_int;
    while (i as c_uint) < (*state).crtc_count {
        // TODO: Error handling
        // let mut gamma_set_cookie: xcb_void_cookie_t = xcb_randr_set_crtc_gamma_checked(
        //     (*state).conn,
        //     crtc,
        //     ramp_size as u16,
        //     gamma_r,
        //     gamma_g,
        //     gamma_b,
        // );
        // error = xcb_request_check((*state).conn, gamma_set_cookie);
        // if !error.is_null() {
        //     fprintf(
        //         stderr,
        //         gettext(b"`%s' returned error %d\n\0" as *const u8 as *const c_char),
        //         b"RANDR Set CRTC Gamma\0" as *const u8 as *const c_char,
        //         (*error).error_code as c_int,
        //     );
        //     fprintf(
        //         stderr,
        //         gettext(b"Unable to restore CRTC %i\n\0" as *const u8 as *const c_char),
        //         i,
        //     );
        // }

        let crtc = (*((*state).crtcs).offset(i as isize)).crtc;
        let ramp_size = (*((*state).crtcs).offset(i as isize)).ramp_size;
        let gamma_r: *mut u16 = &mut *((*((*state).crtcs).offset(i as isize))
            .saved_ramps)
            .offset((0 as c_int as c_uint).wrapping_mul(ramp_size) as isize)
            as *mut u16;
        let gamma_g: *mut u16 = &mut *((*((*state).crtcs).offset(i as isize))
            .saved_ramps)
            .offset((1 as c_int as c_uint).wrapping_mul(ramp_size) as isize)
            as *mut u16;
        let gamma_b: *mut u16 = &mut *((*((*state).crtcs).offset(i as isize))
            .saved_ramps)
            .offset((2 as c_int as c_uint).wrapping_mul(ramp_size) as isize)
            as *mut u16;

        let ramp_size = ramp_size as usize;
        let red = std::slice::from_raw_parts(gamma_r, ramp_size);
        let green = std::slice::from_raw_parts(gamma_g, ramp_size);
        let blue = std::slice::from_raw_parts(gamma_b, ramp_size);

        // Set gamma ramps
        let gamma_set_cookie =
            (*state).conn.send_request_checked(&SetCrtcGamma {
                crtc,
                red,
                green,
                blue,
            });
        // TODO: Error handling
        (*state).conn.check_request(gamma_set_cookie).unwrap();

        i += 1;
    }
}

unsafe extern "C" fn randr_free(state: *mut randr_state_t) {
    /* Free CRTC state */
    let mut i: c_int = 0 as c_int;
    while (i as c_uint) < (*state).crtc_count {
        free((*((*state).crtcs).offset(i as isize)).saved_ramps as *mut c_void);
        i += 1;
        i;
    }
    free((*state).crtcs as *mut c_void);
    free((*state).crtc_num as *mut c_void);
    // Close connection
    // xcb_disconnect((*state).conn);
    free(state as *mut c_void);
}

unsafe extern "C" fn randr_print_help(f: *mut FILE) {
    fputs(
        // gettext(
        b"Adjust gamma ramps with the X RANDR extension.\n\0" as *const u8
            as *const c_char,
        // ),
        f,
    );
    fputs(b"\n\0" as *const u8 as *const c_char, f);
    // TRANSLATORS: RANDR help output
    // left column must not be translated
    fputs(
        // gettext(
            b"  screen=N\t\tX screen to apply adjustments to\n  crtc=N\tList of comma separated CRTCs to apply adjustments to\n\0"
                as *const u8 as *const c_char,
        // ),
        f,
    );
    fputs(b"\n\0" as *const u8 as *const c_char, f);
}

unsafe extern "C" fn randr_set_option(
    state: *mut randr_state_t,
    key: *const c_char,
    value: *const c_char,
) -> c_int {
    if strcasecmp(key, b"screen\0" as *const u8 as *const c_char) == 0 as c_int
    {
        (*state).screen_num = atoi(value);
    } else if strcasecmp(key, b"crtc\0" as *const u8 as *const c_char)
        == 0 as c_int
    {
        let mut tail: *mut c_char = std::ptr::null_mut::<c_char>();
        // Check how many crtcs are configured
        let mut local_value: *const c_char = value;
        loop {
            *__errno_location() = 0 as c_int;
            let parsed: c_int =
                strtol(local_value, &mut tail, 0 as c_int) as c_int;
            if parsed == 0 as c_int
                && (*__errno_location() != 0 as c_int
                    || tail == local_value as *mut c_char)
            {
                // gettext(
                eprintln!("Unable to read screen number: `{}`.", *value);
                return -(1 as c_int);
            } else {
                (*state).crtc_num_count += 1 as c_int;
            }
            local_value = tail;
            if *local_value as c_int == ',' as i32 {
                local_value = local_value.offset(1 as c_int as isize);
            } else if *local_value as c_int == '\0' as i32 {
                break;
            }
        }

        // Configure all given crtcs
        (*state).crtc_num = calloc(
            (*state).crtc_num_count as usize,
            ::core::mem::size_of::<c_int>(),
        ) as *mut c_int;
        local_value = value;
        let mut i: c_int = 0 as c_int;
        while i < (*state).crtc_num_count {
            *__errno_location() = 0 as c_int;
            let parsed_0: c_int =
                strtol(local_value, &mut tail, 0 as c_int) as c_int;
            if parsed_0 == 0 as c_int
                && (*__errno_location() != 0 as c_int
                    || tail == local_value as *mut c_char)
            {
                return -(1 as c_int);
            } else {
                *((*state).crtc_num).offset(i as isize) = parsed_0;
            }
            local_value = tail;
            if *local_value as c_int == ',' as i32 {
                local_value = local_value.offset(1 as c_int as isize);
            } else if *local_value as c_int == '\0' as i32 {
                break;
            }
            i += 1;
            i;
        }
    } else if strcasecmp(key, b"preserve\0" as *const u8 as *const c_char)
        == 0 as c_int
    {
        // gettext(
        eprintln!(
            "Parameter `{}` is now always on;  Use the `-P` command-line option to disable.",
            *key,
        );
    } else {
        // gettext(
        eprintln!("Unknown method parameter: `{}`.", *key);
        return -(1 as c_int);
    }
    0 as c_int
}

unsafe extern "C" fn randr_set_temperature_for_crtc(
    state: *mut randr_state_t,
    crtc_num: c_int,
    setting: *const ColorSettings,
    preserve: c_int,
) -> c_int {
    let error: *mut xcb_generic_error_t =
        std::ptr::null_mut::<xcb_generic_error_t>();
    if crtc_num as c_uint >= (*state).crtc_count || crtc_num < 0 as c_int {
        // gettext(
        eprintln!("CRTC {} does not exist.", crtc_num);
        if (*state).crtc_count > 1 as c_int as c_uint {
            // gettext(
            eprintln!(
                "Valid CRTCs are [0-{}].",
                ((*state).crtc_count).wrapping_sub(1 as c_int as c_uint)
            );
        } else {
            // gettext(
            eprintln!("Only CRTC 0 exists.");
        }
        return -(1 as c_int);
    }
    let crtc = (*((*state).crtcs).offset(crtc_num as isize)).crtc;
    let ramp_size: c_uint =
        (*((*state).crtcs).offset(crtc_num as isize)).ramp_size;
    // Create new gamma ramps
    let gamma_ramps: *mut u16 = malloc(
        ((3 as c_int as c_uint).wrapping_mul(ramp_size) as usize)
            .wrapping_mul(::core::mem::size_of::<u16>()),
    ) as *mut u16;
    if gamma_ramps.is_null() {
        perror(b"malloc\0" as *const u8 as *const c_char);
        return -(1 as c_int);
    }
    let gamma_r: *mut u16 = &mut *gamma_ramps
        .offset((0 as c_int as c_uint).wrapping_mul(ramp_size) as isize)
        as *mut u16;
    let gamma_g: *mut u16 = &mut *gamma_ramps
        .offset((1 as c_int as c_uint).wrapping_mul(ramp_size) as isize)
        as *mut u16;
    let gamma_b: *mut u16 = &mut *gamma_ramps
        .offset((2 as c_int as c_uint).wrapping_mul(ramp_size) as isize)
        as *mut u16;
    // Initialize gamma ramps from saved state
    if preserve != 0 {
        memcpy(
            gamma_ramps as *mut c_void,
            (*((*state).crtcs).offset(crtc_num as isize)).saved_ramps
                as *const c_void,
            ((3 as c_int as c_uint).wrapping_mul(ramp_size) as usize)
                .wrapping_mul(::core::mem::size_of::<u16>()),
        );
    } else {
        // Initialize gamma ramps to pure state
        let mut i: c_int = 0 as c_int;
        while (i as c_uint) < ramp_size {
            let value: u16 = (i as c_double / ramp_size as c_double
                * (65535 as c_int + 1 as c_int) as c_double)
                as u16;
            *gamma_r.offset(i as isize) = value;
            *gamma_g.offset(i as isize) = value;
            *gamma_b.offset(i as isize) = value;
            i += 1;
            i;
        }
    }

    let r = std::slice::from_raw_parts_mut(gamma_r, ramp_size as usize);
    let g = std::slice::from_raw_parts_mut(gamma_g, ramp_size as usize);
    let b = std::slice::from_raw_parts_mut(gamma_b, ramp_size as usize);
    colorramp_fill(r, g, b, &*setting);
    // colorramp_fill(gamma_r, gamma_g, gamma_b, ramp_size as c_int, setting);

    // TODO: Error handling
    // let mut gamma_set_cookie: xcb_void_cookie_t = xcb_randr_set_crtc_gamma_checked(
    //     (*state).conn,
    //     crtc,
    //     ramp_size as u16,
    //     gamma_r,
    //     gamma_g,
    //     gamma_b,
    // );
    // error = xcb_request_check((*state).conn, gamma_set_cookie);
    // if !error.is_null() {
    //     fprintf(
    //         stderr,
    //         gettext(b"`%s' returned error %d\n\0" as *const u8 as *const c_char),
    //         b"RANDR Set CRTC Gamma\0" as *const u8 as *const c_char,
    //         (*error).error_code as c_int,
    //     );
    //     free(gamma_ramps as *mut c_void);
    //     return -(1 as c_int);
    // }

    // Set new gamma ramps
    let ramp_size = ramp_size as usize;
    let red = std::slice::from_raw_parts(gamma_r, ramp_size);
    let green = std::slice::from_raw_parts(gamma_g, ramp_size);
    let blue = std::slice::from_raw_parts(gamma_b, ramp_size);
    let gamma_set_cookie = (*state).conn.send_request_checked(&SetCrtcGamma {
        crtc,
        red,
        green,
        blue,
    });
    // TODO: Error handling
    (*state).conn.check_request(gamma_set_cookie).unwrap();

    free(gamma_ramps as *mut c_void);
    0 as c_int
}

unsafe extern "C" fn randr_set_temperature(
    state: *mut randr_state_t,
    setting: *const ColorSettings,
    preserve: c_int,
) -> c_int {
    let mut r: c_int = 0;
    // If no CRTC numbers have been specified,
    // set temperature on all CRTCs.
    if (*state).crtc_num_count == 0 as c_int {
        let mut i: c_int = 0 as c_int;
        while (i as c_uint) < (*state).crtc_count {
            r = randr_set_temperature_for_crtc(state, i, setting, preserve);
            if r < 0 as c_int {
                return -(1 as c_int);
            }
            i += 1;
            i;
        }
    } else {
        let mut i_0: c_int = 0 as c_int;
        while i_0 < (*state).crtc_num_count {
            r = randr_set_temperature_for_crtc(
                state,
                *((*state).crtc_num).offset(i_0 as isize),
                setting,
                preserve,
            );
            if r < 0 as c_int {
                return -(1 as c_int);
            }
            i_0 += 1;
            i_0;
        }
    }
    0 as c_int
}
