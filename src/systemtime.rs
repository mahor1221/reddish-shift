use libc::{__syscall_ulong_t, clock_gettime, nanosleep, perror, time_t, timespec};
use std::ffi::{c_char, c_double, c_int, c_long, c_uint};

#[no_mangle]
pub unsafe extern "C" fn systemtime_get_time(mut t: *mut c_double) -> c_int {
    let mut now: timespec = timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    let mut r: c_int = clock_gettime(0 as c_int, &mut now);
    if r < 0 as c_int {
        perror(b"clock_gettime\0" as *const u8 as *const c_char);
        return -(1 as c_int);
    }
    *t = now.tv_sec as c_double + now.tv_nsec as c_double / 1000000000.0f64;
    return 0 as c_int;
}

#[no_mangle]
pub unsafe extern "C" fn systemtime_msleep(mut msecs: c_uint) {
    let mut sleep: timespec = timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    sleep.tv_sec = msecs.wrapping_div(1000 as c_int as c_uint) as time_t;
    sleep.tv_nsec = msecs
        .wrapping_rem(1000 as c_int as c_uint)
        .wrapping_mul(1000000 as c_int as c_uint) as c_long; //__syscall_slong_t;
    nanosleep(&mut sleep, 0 as *mut timespec);
}
