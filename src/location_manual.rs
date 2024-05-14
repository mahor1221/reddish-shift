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

unsafe extern "C" fn location_manual_init(mut state: *mut *mut location_manual_state_t) -> c_int {
    *state =
        malloc(::core::mem::size_of::<location_manual_state_t>()) as *mut location_manual_state_t;
    if (*state).is_null() {
        return -(1 as c_int);
    }
    let mut s: *mut location_manual_state_t = *state;
    (*s).loc.lat = ::core::f32::NAN;
    (*s).loc.lon = ::core::f32::NAN;
    return 0 as c_int;
}

unsafe extern "C" fn location_manual_start(mut state: *mut location_manual_state_t) -> c_int {
    if ((*state).loc.lat).is_nan() as i32 != 0 || ((*state).loc.lon).is_nan() as i32 != 0 {
        // gettext(
        eprintln!("Latitude and longitude must be set.");
        exit(1 as c_int);
    }
    return 0 as c_int;
}

unsafe extern "C" fn location_manual_free(mut state: *mut location_manual_state_t) {
    free(state as *mut c_void);
}

unsafe extern "C" fn location_manual_print_help(mut f: *mut FILE) {
    fputs(
        // gettext(
        b"Specify location manually.\n\0" as *const u8 as *const c_char,
        f,
    );
    fputs(b"\n\0" as *const u8 as *const c_char, f);
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
    mut state: *mut location_manual_state_t,
    mut key: *const c_char,
    mut value: *const c_char,
) -> c_int {
    let mut end: *mut c_char = 0 as *mut c_char;
    *__errno_location() = 0 as c_int;
    let mut v: c_float = strtof(value, &mut end);
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
    return 0 as c_int;
}
unsafe extern "C" fn location_manual_get_fd(mut state: *mut location_manual_state_t) -> c_int {
    return -(1 as c_int);
}
unsafe extern "C" fn location_manual_handle(
    mut state: *mut location_manual_state_t,
    mut location: *mut location_t,
    mut available: *mut c_int,
) -> c_int {
    *location = (*state).loc;
    *available = 1 as c_int;
    return 0 as c_int;
}
#[no_mangle]
pub static mut manual_location_provider: location_provider_t = unsafe {
    {
        let mut init = location_provider_t {
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
        };
        init
    }
};
