/*  gamma-drm.rs -- DRM gamma adjustment
    This file is part of <https://github.com/mahor1221/reddish-shift>.
    Copyright (C) 2024 Mahor Foruzesh <mahor1221@gmail.com>
    Ported from Redshift <https://github.com/jonls/redshift>.
    Copyright (c) 2014  Mattias Andrée <maandree@member.fsf.org>
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

use crate::{colorramp::colorramp_fill, config::ColorSetting};
use drm::ffi::xf86drm_mode::{
    drmModeCrtc, drmModeCrtcGetGamma, drmModeCrtcSetGamma, drmModeFreeCrtc, drmModeFreeResources,
    drmModeGetCrtc, drmModeGetResources, drmModeRes,
};
use libc::{
    atoi, calloc, close, fputs, free, malloc, open, perror, realloc, sprintf, strcasecmp, strlen,
    FILE,
};
use std::ffi::{c_char, c_double, c_int, c_long, c_void, CStr};

#[derive(Copy, Clone)]
#[repr(C)]
pub struct drm_state_t {
    pub card_num: c_int,
    pub crtc_num: c_int,
    pub fd: c_int,
    pub res: *mut drmModeRes,
    pub crtcs: *mut drm_crtc_state_t,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct drm_crtc_state_t {
    pub crtc_num: c_int,
    pub crtc_id: c_int,
    pub gamma_size: c_int,
    pub r_gamma: *mut u16,
    pub g_gamma: *mut u16,
    pub b_gamma: *mut u16,
}

unsafe extern "C" fn drm_init(state: *mut *mut drm_state_t) -> c_int {
    // Initialize state.
    *state = malloc(::core::mem::size_of::<drm_state_t>()) as *mut drm_state_t;
    if (*state).is_null() {
        return -(1 as c_int);
    }
    let s: *mut drm_state_t = *state;
    (*s).card_num = 0 as c_int;
    (*s).crtc_num = -(1 as c_int);
    (*s).fd = -(1 as c_int);
    (*s).res = std::ptr::null_mut::<drmModeRes>();
    (*s).crtcs = std::ptr::null_mut::<drm_crtc_state_t>();
    0 as c_int
}

unsafe extern "C" fn drm_start(state: *mut drm_state_t) -> c_int {
    // Acquire access to a graphics card.
    let maxlen: c_long = (strlen(b"/dev/dri\0" as *const u8 as *const c_char))
        .wrapping_add(strlen(b"%s/card%d\0" as *const u8 as *const c_char))
        .wrapping_add(10) as c_long;
    let vla = maxlen as usize;
    let mut pathname: Vec<c_char> = ::std::vec::from_elem(0, vla);
    sprintf(
        pathname.as_mut_ptr(),
        b"%s/card%d\0" as *const u8 as *const c_char,
        b"/dev/dri\0" as *const u8 as *const c_char,
        (*state).card_num,
    );
    (*state).fd = open(pathname.as_mut_ptr(), 0o2 as c_int | 0o2000000 as c_int);
    if (*state).fd < 0 as c_int {
        // TODO check if access permissions, normally root or
        //      membership of the video group is required.
        perror(b"open\0" as *const u8 as *const c_char);
        // gettext(
        let pathname = CStr::from_ptr(pathname.as_ptr()).to_str().unwrap();
        eprintln!("Failed to open DRM device: {pathname}");
        return -(1 as c_int);
    }
    // Acquire mode resources.
    (*state).res = drmModeGetResources((*state).fd);
    if ((*state).res).is_null() {
        // gettext(
        eprintln!("Failed to get DRM mode resources");
        close((*state).fd);
        (*state).fd = -(1 as c_int);
        return -(1 as c_int);
    }
    // Create entries for selected CRTCs.
    let crtc_count: c_int = (*(*state).res).count_crtcs;
    if (*state).crtc_num >= 0 as c_int {
        if (*state).crtc_num >= crtc_count {
            // gettext(
            eprintln!("CRTC {} does not exist.", (*state).crtc_num);
            if crtc_count > 1 as c_int {
                // gettext(
                eprintln!("Valid CRTCs are [0-{}]..", crtc_count - 1);
            } else {
                // gettext(
                eprintln!("Only CRTC 0 exists.")
            }
            close((*state).fd);
            (*state).fd = -(1 as c_int);
            drmModeFreeResources((*state).res);
            (*state).res = std::ptr::null_mut::<drmModeRes>();
            return -(1 as c_int);
        }
        (*state).crtcs = malloc(2_usize.wrapping_mul(::core::mem::size_of::<drm_crtc_state_t>()))
            as *mut drm_crtc_state_t;
        (*((*state).crtcs).offset(1 as c_int as isize)).crtc_num = -(1 as c_int);
        (*(*state).crtcs).crtc_num = (*state).crtc_num;
        (*(*state).crtcs).crtc_id = -(1 as c_int);
        (*(*state).crtcs).gamma_size = -(1 as c_int);
        (*(*state).crtcs).r_gamma = std::ptr::null_mut::<u16>();
        (*(*state).crtcs).g_gamma = std::ptr::null_mut::<u16>();
        (*(*state).crtcs).b_gamma = std::ptr::null_mut::<u16>();
    } else {
        let mut crtc_num: c_int = 0;
        (*state).crtcs = malloc(
            ((crtc_count + 1) as usize).wrapping_mul(::core::mem::size_of::<drm_crtc_state_t>()),
        ) as *mut drm_crtc_state_t;
        (*((*state).crtcs).offset(crtc_count as isize)).crtc_num = -(1 as c_int);
        crtc_num = 0 as c_int;
        while crtc_num < crtc_count {
            (*((*state).crtcs).offset(crtc_num as isize)).crtc_num = crtc_num;
            (*((*state).crtcs).offset(crtc_num as isize)).crtc_id = -(1 as c_int);
            (*((*state).crtcs).offset(crtc_num as isize)).gamma_size = -(1 as c_int);
            let fresh0 = &mut (*((*state).crtcs).offset(crtc_num as isize)).r_gamma;
            *fresh0 = std::ptr::null_mut::<u16>();
            let fresh1 = &mut (*((*state).crtcs).offset(crtc_num as isize)).g_gamma;
            *fresh1 = std::ptr::null_mut::<u16>();
            let fresh2 = &mut (*((*state).crtcs).offset(crtc_num as isize)).b_gamma;
            *fresh2 = std::ptr::null_mut::<u16>();
            crtc_num += 1;
            crtc_num;
        }
    }
    // Load CRTC information and gamma ramps.
    let mut crtcs: *mut drm_crtc_state_t = (*state).crtcs;
    while (*crtcs).crtc_num >= 0 as c_int {
        (*crtcs).crtc_id = *((*(*state).res).crtcs).offset((*crtcs).crtc_num as isize) as c_int;
        let crtc_info: *mut drmModeCrtc = drmModeGetCrtc((*state).fd, (*crtcs).crtc_id as u32);
        if crtc_info.is_null() {
            // gettext(
            eprintln!("CRTC {} lost, skipping", (*crtcs).crtc_num)
        } else {
            (*crtcs).gamma_size = (*crtc_info).gamma_size;
            drmModeFreeCrtc(crtc_info);
            if (*crtcs).gamma_size <= 1 as c_int {
                // gettext(
                eprintln!(
          "Could not get gamma ramp size for CRTC {}\non graphics card {}, ignoring device.",
          (*crtcs).crtc_num,
          (*state).card_num,
        );
            } else {
                // Valgrind complains about us reading uninitialize memory if we just use malloc.
                (*crtcs).r_gamma = calloc(
                    (3 * (*crtcs).gamma_size) as usize,
                    ::core::mem::size_of::<u16>(),
                ) as *mut u16;
                (*crtcs).g_gamma = ((*crtcs).r_gamma).offset((*crtcs).gamma_size as isize);
                (*crtcs).b_gamma = ((*crtcs).g_gamma).offset((*crtcs).gamma_size as isize);
                if !((*crtcs).r_gamma).is_null() {
                    let r: c_int = drmModeCrtcGetGamma(
                        (*state).fd,
                        (*crtcs).crtc_id as u32,
                        (*crtcs).gamma_size as u32,
                        (*crtcs).r_gamma,
                        (*crtcs).g_gamma,
                        (*crtcs).b_gamma,
                    );
                    if r < 0 as c_int {
                        eprintln!(
              "DRM could not read gamma ramps on CRTC {} on\ngraphics card {}, ignoring device.",
              (*crtcs).crtc_num,
              (*state).card_num,
            );
                        free((*crtcs).r_gamma as *mut c_void);
                        (*crtcs).r_gamma = std::ptr::null_mut::<u16>();
                    }
                } else {
                    perror(b"malloc\0" as *const u8 as *const c_char);
                    drmModeFreeResources((*state).res);
                    (*state).res = std::ptr::null_mut::<drmModeRes>();
                    close((*state).fd);
                    (*state).fd = -(1 as c_int);
                    loop {
                        let fresh3 = crtcs;
                        crtcs = crtcs.offset(-1);
                        if fresh3 == (*state).crtcs {
                            break;
                        }
                        free((*crtcs).r_gamma as *mut c_void);
                    }
                    free((*state).crtcs as *mut c_void);
                    (*state).crtcs = std::ptr::null_mut::<drm_crtc_state_t>();
                    return -(1 as c_int);
                }
            }
        }
        crtcs = crtcs.offset(1);
        crtcs;
    }
    0 as c_int
}

unsafe extern "C" fn drm_restore(state: *mut drm_state_t) {
    let mut crtcs: *mut drm_crtc_state_t = (*state).crtcs;
    while (*crtcs).crtc_num >= 0 as c_int {
        if !((*crtcs).r_gamma).is_null() {
            drmModeCrtcSetGamma(
                (*state).fd,
                (*crtcs).crtc_id as u32,
                (*crtcs).gamma_size as u32,
                (*crtcs).r_gamma,
                (*crtcs).g_gamma,
                (*crtcs).b_gamma,
            );
        }
        crtcs = crtcs.offset(1);
        crtcs;
    }
}

unsafe extern "C" fn drm_free(state: *mut drm_state_t) {
    if !((*state).crtcs).is_null() {
        let mut crtcs: *mut drm_crtc_state_t = (*state).crtcs;
        while (*crtcs).crtc_num >= 0 as c_int {
            free((*crtcs).r_gamma as *mut c_void);
            (*crtcs).crtc_num = -(1 as c_int);
            crtcs = crtcs.offset(1);
            crtcs;
        }
        free((*state).crtcs as *mut c_void);
        (*state).crtcs = std::ptr::null_mut::<drm_crtc_state_t>();
    }
    if !((*state).res).is_null() {
        drmModeFreeResources((*state).res);
        (*state).res = std::ptr::null_mut::<drmModeRes>();
    }
    if (*state).fd >= 0 as c_int {
        close((*state).fd);
        (*state).fd = -(1 as c_int);
    }
    free(state as *mut c_void);
}

unsafe extern "C" fn drm_print_help(f: *mut FILE) {
    fputs(
        // gettext(
        b"Adjust gamma ramps with Direct Rendering Manager.\n\0" as *const u8 as *const c_char,
        // ),
        f,
    );
    fputs(b"\n\0" as *const u8 as *const c_char, f);
    // TRANSLATORS: DRM help output
    // left column must not be translated
    fputs(
    // gettext(
    b"  card=N\tGraphics card to apply adjustments to\n  crtc=N\tCRTC to apply adjustments to\n\0"
      as *const u8 as *const c_char,
    // ),
    f,
  );
    fputs(b"\n\0" as *const u8 as *const c_char, f);
}

unsafe extern "C" fn drm_set_option(
    state: *mut drm_state_t,
    key: *const c_char,
    value: *const c_char,
) -> c_int {
    if strcasecmp(key, b"card\0" as *const u8 as *const c_char) == 0 as c_int {
        (*state).card_num = atoi(value);
    } else if strcasecmp(key, b"crtc\0" as *const u8 as *const c_char) == 0 as c_int {
        (*state).crtc_num = atoi(value);
        if (*state).crtc_num < 0 as c_int {
            // gettext(
            eprintln!("CRTC must be a non-negative integer");
            return -(1 as c_int);
        }
    } else {
        eprintln!("Unknown method parameter: `{}`.", *key);
        return -(1 as c_int);
    }
    0 as c_int
}

unsafe extern "C" fn drm_set_temperature(
    state: *mut drm_state_t,
    setting: &ColorSetting,
    preserve: c_int,
) -> c_int {
    let mut crtcs: *mut drm_crtc_state_t = (*state).crtcs;
    let mut last_gamma_size: c_int = 0 as c_int;
    let mut r_gamma: *mut u16 = std::ptr::null_mut::<u16>();
    let mut g_gamma: *mut u16 = std::ptr::null_mut::<u16>();
    let mut b_gamma: *mut u16 = std::ptr::null_mut::<u16>();
    while (*crtcs).crtc_num >= 0 as c_int {
        if (*crtcs).gamma_size > 1 as c_int {
            if (*crtcs).gamma_size != last_gamma_size {
                if last_gamma_size == 0 as c_int {
                    r_gamma = malloc(
                        ((3 * (*crtcs).gamma_size) as usize)
                            .wrapping_mul(::core::mem::size_of::<u16>()),
                    ) as *mut u16;
                    g_gamma = r_gamma.offset((*crtcs).gamma_size as isize);
                    b_gamma = g_gamma.offset((*crtcs).gamma_size as isize);
                } else if (*crtcs).gamma_size > last_gamma_size {
                    r_gamma = realloc(
                        r_gamma as *mut c_void,
                        ((3 * (*crtcs).gamma_size) as usize)
                            .wrapping_mul(::core::mem::size_of::<u16>()),
                    ) as *mut u16;
                    g_gamma = r_gamma.offset((*crtcs).gamma_size as isize);
                    b_gamma = g_gamma.offset((*crtcs).gamma_size as isize);
                }
                if r_gamma.is_null() {
                    perror(if last_gamma_size == 0 as c_int {
                        b"malloc\0" as *const u8 as *const c_char
                    } else {
                        b"realloc\0" as *const u8 as *const c_char
                    });
                    return -(1 as c_int);
                }
                last_gamma_size = (*crtcs).gamma_size;
            }
            // Initialize gamma ramps to pure state
            let ramp_size: c_int = (*crtcs).gamma_size;
            let mut i: c_int = 0 as c_int;
            while i < ramp_size {
                let value: u16 = (i as c_double / ramp_size as c_double
                    * (65535 as c_int + 1 as c_int) as c_double)
                    as u16;
                *r_gamma.offset(i as isize) = value;
                *g_gamma.offset(i as isize) = value;
                *b_gamma.offset(i as isize) = value;
                i += 1;
                i;
            }

            let r = std::slice::from_raw_parts_mut(r_gamma, (*crtcs).gamma_size as usize);
            let g = std::slice::from_raw_parts_mut(g_gamma, (*crtcs).gamma_size as usize);
            let b = std::slice::from_raw_parts_mut(b_gamma, (*crtcs).gamma_size as usize);
            colorramp_fill(r, g, b, &*setting);
            drmModeCrtcSetGamma(
                (*state).fd,
                (*crtcs).crtc_id as u32,
                (*crtcs).gamma_size as u32,
                r_gamma,
                g_gamma,
                b_gamma,
            );
        }
        crtcs = crtcs.offset(1);
        crtcs;
    }
    free(r_gamma as *mut c_void);
    0 as c_int
}
