use crate::{
    color_setting_t, colorramp::colorramp_fill, gamma_method_free_func, gamma_method_init_func,
    gamma_method_print_help_func, gamma_method_restore_func, gamma_method_set_option_func,
    gamma_method_set_temperature_func, gamma_method_start_func, gamma_method_t,
};
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

unsafe extern "C" fn drm_init(mut state: *mut *mut drm_state_t) -> c_int {
    *state = malloc(::core::mem::size_of::<drm_state_t>()) as *mut drm_state_t;
    if (*state).is_null() {
        return -(1 as c_int);
    }
    let mut s: *mut drm_state_t = *state;
    (*s).card_num = 0 as c_int;
    (*s).crtc_num = -(1 as c_int);
    (*s).fd = -(1 as c_int);
    (*s).res = 0 as *mut drmModeRes;
    (*s).crtcs = 0 as *mut drm_crtc_state_t;
    return 0 as c_int;
}

unsafe extern "C" fn drm_start(mut state: *mut drm_state_t) -> c_int {
    let mut maxlen: c_long = (strlen(b"/dev/dri\0" as *const u8 as *const c_char))
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
        perror(b"open\0" as *const u8 as *const c_char);
        // gettext(
        let pathname = CStr::from_ptr(pathname.as_ptr()).to_str().unwrap();
        eprintln!("Failed to open DRM device: {pathname}");
        return -(1 as c_int);
    }
    (*state).res = drmModeGetResources((*state).fd);
    if ((*state).res).is_null() {
        // gettext(
        eprintln!("Failed to get DRM mode resources");
        close((*state).fd);
        (*state).fd = -(1 as c_int);
        return -(1 as c_int);
    }
    let mut crtc_count: c_int = (*(*state).res).count_crtcs;
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
            (*state).res = 0 as *mut drmModeRes;
            return -(1 as c_int);
        }
        (*state).crtcs = malloc(2_usize.wrapping_mul(::core::mem::size_of::<drm_crtc_state_t>()))
            as *mut drm_crtc_state_t;
        (*((*state).crtcs).offset(1 as c_int as isize)).crtc_num = -(1 as c_int);
        (*(*state).crtcs).crtc_num = (*state).crtc_num;
        (*(*state).crtcs).crtc_id = -(1 as c_int);
        (*(*state).crtcs).gamma_size = -(1 as c_int);
        (*(*state).crtcs).r_gamma = 0 as *mut u16;
        (*(*state).crtcs).g_gamma = 0 as *mut u16;
        (*(*state).crtcs).b_gamma = 0 as *mut u16;
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
            let ref mut fresh0 = (*((*state).crtcs).offset(crtc_num as isize)).r_gamma;
            *fresh0 = 0 as *mut u16;
            let ref mut fresh1 = (*((*state).crtcs).offset(crtc_num as isize)).g_gamma;
            *fresh1 = 0 as *mut u16;
            let ref mut fresh2 = (*((*state).crtcs).offset(crtc_num as isize)).b_gamma;
            *fresh2 = 0 as *mut u16;
            crtc_num += 1;
            crtc_num;
        }
    }
    let mut crtcs: *mut drm_crtc_state_t = (*state).crtcs;
    while (*crtcs).crtc_num >= 0 as c_int {
        (*crtcs).crtc_id = *((*(*state).res).crtcs).offset((*crtcs).crtc_num as isize) as c_int;
        let mut crtc_info: *mut drmModeCrtc = drmModeGetCrtc((*state).fd, (*crtcs).crtc_id as u32);
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
                (*crtcs).r_gamma = calloc(
                    (3 * (*crtcs).gamma_size) as usize,
                    ::core::mem::size_of::<u16>(),
                ) as *mut u16;
                (*crtcs).g_gamma = ((*crtcs).r_gamma).offset((*crtcs).gamma_size as isize);
                (*crtcs).b_gamma = ((*crtcs).g_gamma).offset((*crtcs).gamma_size as isize);
                if !((*crtcs).r_gamma).is_null() {
                    let mut r: c_int = drmModeCrtcGetGamma(
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
                        (*crtcs).r_gamma = 0 as *mut u16;
                    }
                } else {
                    perror(b"malloc\0" as *const u8 as *const c_char);
                    drmModeFreeResources((*state).res);
                    (*state).res = 0 as *mut drmModeRes;
                    close((*state).fd);
                    (*state).fd = -(1 as c_int);
                    loop {
                        let fresh3 = crtcs;
                        crtcs = crtcs.offset(-1);
                        if !(fresh3 != (*state).crtcs) {
                            break;
                        }
                        free((*crtcs).r_gamma as *mut c_void);
                    }
                    free((*state).crtcs as *mut c_void);
                    (*state).crtcs = 0 as *mut drm_crtc_state_t;
                    return -(1 as c_int);
                }
            }
        }
        crtcs = crtcs.offset(1);
        crtcs;
    }
    return 0 as c_int;
}

unsafe extern "C" fn drm_restore(mut state: *mut drm_state_t) {
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

unsafe extern "C" fn drm_free(mut state: *mut drm_state_t) {
    if !((*state).crtcs).is_null() {
        let mut crtcs: *mut drm_crtc_state_t = (*state).crtcs;
        while (*crtcs).crtc_num >= 0 as c_int {
            free((*crtcs).r_gamma as *mut c_void);
            (*crtcs).crtc_num = -(1 as c_int);
            crtcs = crtcs.offset(1);
            crtcs;
        }
        free((*state).crtcs as *mut c_void);
        (*state).crtcs = 0 as *mut drm_crtc_state_t;
    }
    if !((*state).res).is_null() {
        drmModeFreeResources((*state).res);
        (*state).res = 0 as *mut drmModeRes;
    }
    if (*state).fd >= 0 as c_int {
        close((*state).fd);
        (*state).fd = -(1 as c_int);
    }
    free(state as *mut c_void);
}

unsafe extern "C" fn drm_print_help(mut f: *mut FILE) {
    fputs(
        // gettext(
        b"Adjust gamma ramps with Direct Rendering Manager.\n\0" as *const u8 as *const c_char,
        // ),
        f,
    );
    fputs(b"\n\0" as *const u8 as *const c_char, f);
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
    mut state: *mut drm_state_t,
    mut key: *const c_char,
    mut value: *const c_char,
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
    return 0 as c_int;
}

unsafe extern "C" fn drm_set_temperature(
    mut state: *mut drm_state_t,
    mut setting: *const color_setting_t,
    mut preserve: c_int,
) -> c_int {
    let mut crtcs: *mut drm_crtc_state_t = (*state).crtcs;
    let mut last_gamma_size: c_int = 0 as c_int;
    let mut r_gamma: *mut u16 = 0 as *mut u16;
    let mut g_gamma: *mut u16 = 0 as *mut u16;
    let mut b_gamma: *mut u16 = 0 as *mut u16;
    while (*crtcs).crtc_num >= 0 as c_int {
        if !((*crtcs).gamma_size <= 1 as c_int) {
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
            let mut ramp_size: c_int = (*crtcs).gamma_size;
            let mut i: c_int = 0 as c_int;
            while i < ramp_size {
                let mut value: u16 = (i as c_double / ramp_size as c_double
                    * (65535 as c_int + 1 as c_int) as c_double)
                    as u16;
                *r_gamma.offset(i as isize) = value;
                *g_gamma.offset(i as isize) = value;
                *b_gamma.offset(i as isize) = value;
                i += 1;
                i;
            }
            colorramp_fill(r_gamma, g_gamma, b_gamma, (*crtcs).gamma_size, setting);
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
    return 0 as c_int;
}

#[no_mangle]
pub static mut drm_gamma_method: gamma_method_t = unsafe {
    {
        let mut init = gamma_method_t {
            name: b"drm\0" as *const u8 as *const c_char as *mut c_char,
            autostart: 0 as c_int,
            init: ::core::mem::transmute::<
                Option<unsafe extern "C" fn(*mut *mut drm_state_t) -> c_int>,
                Option<gamma_method_init_func>,
            >(Some(
                drm_init as unsafe extern "C" fn(*mut *mut drm_state_t) -> c_int,
            )),
            start: ::core::mem::transmute::<
                Option<unsafe extern "C" fn(*mut drm_state_t) -> c_int>,
                Option<gamma_method_start_func>,
            >(Some(
                drm_start as unsafe extern "C" fn(*mut drm_state_t) -> c_int,
            )),
            free: ::core::mem::transmute::<
                Option<unsafe extern "C" fn(*mut drm_state_t) -> ()>,
                Option<gamma_method_free_func>,
            >(Some(
                drm_free as unsafe extern "C" fn(*mut drm_state_t) -> (),
            )),
            print_help: ::core::mem::transmute::<
                Option<unsafe extern "C" fn(*mut FILE) -> ()>,
                Option<gamma_method_print_help_func>,
            >(Some(
                drm_print_help as unsafe extern "C" fn(*mut FILE) -> (),
            )),
            set_option: ::core::mem::transmute::<
                Option<
                    unsafe extern "C" fn(*mut drm_state_t, *const c_char, *const c_char) -> c_int,
                >,
                Option<gamma_method_set_option_func>,
            >(Some(
                drm_set_option
                    as unsafe extern "C" fn(
                        *mut drm_state_t,
                        *const c_char,
                        *const c_char,
                    ) -> c_int,
            )),
            restore: ::core::mem::transmute::<
                Option<unsafe extern "C" fn(*mut drm_state_t) -> ()>,
                Option<gamma_method_restore_func>,
            >(Some(
                drm_restore as unsafe extern "C" fn(*mut drm_state_t) -> (),
            )),
            set_temperature: ::core::mem::transmute::<
                Option<
                    unsafe extern "C" fn(*mut drm_state_t, *const color_setting_t, c_int) -> c_int,
                >,
                Option<gamma_method_set_temperature_func>,
            >(Some(
                drm_set_temperature
                    as unsafe extern "C" fn(
                        *mut drm_state_t,
                        *const color_setting_t,
                        c_int,
                    ) -> c_int,
            )),
        };
        init
    }
};
