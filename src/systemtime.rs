/*  systemtime.rs -- Portable system time source
    This file is part of <https://github.com/mahor1221/reddish-shift>.
    Copyright (C) 2024 Mahor Foruzesh <mahor1221@gmail.com>
    Ported from Redshift <https://github.com/jonls/redshift>.
    Copyright (c) 2010-2014  Jon Lund Steffensen <jonlst@gmail.com>

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

use libc::{clock_gettime, nanosleep, perror, time_t, timespec};
use std::ffi::{c_char, c_double, c_int, c_long, c_uint};

// Return current time in T as the number of seconds since the epoch.
#[no_mangle]
pub unsafe extern "C" fn systemtime_get_time(t: *mut c_double) -> c_int {
    let mut now: timespec = timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    let r: c_int = clock_gettime(0 as c_int, &mut now);
    if r < 0 as c_int {
        perror(b"clock_gettime\0" as *const u8 as *const c_char);
        return -(1 as c_int);
    }
    *t = now.tv_sec as c_double + now.tv_nsec as c_double / 1000000000.0f64;
    0 as c_int
}

// Sleep for a number of milliseconds.
#[no_mangle]
pub unsafe extern "C" fn systemtime_msleep(msecs: c_uint) {
    let mut sleep: timespec = timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    sleep.tv_sec = msecs.wrapping_div(1000 as c_int as c_uint) as time_t;
    sleep.tv_nsec = msecs
        .wrapping_rem(1000 as c_int as c_uint)
        .wrapping_mul(1000000 as c_int as c_uint) as c_long; //__syscall_slong_t;
    nanosleep(&mut sleep, std::ptr::null_mut::<timespec>());
}
