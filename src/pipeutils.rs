/*  pipeutils.rs -- Utilities for using pipes as signals
    This file is part of <https://github.com/mahor1221/reddish-shift>.
    Copyright (C) 2024 Mahor Foruzesh <mahor1221@gmail.com>
    Ported from Redshift <https://github.com/jonls/redshift>.
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

use libc::{close, fcntl, perror, pipe, read, size_t, write};
use std::ffi::{c_char, c_int, c_void};

// Create non-blocking set of pipe fds.
#[no_mangle]
pub unsafe extern "C" fn pipeutils_create_nonblocking(pipefds: *mut c_int) -> c_int {
    let mut r: c_int = pipe(pipefds);
    if r == -(1 as c_int) {
        perror(b"pipe\0" as *const u8 as *const c_char);
        return -(1 as c_int);
    }
    let mut flags: c_int = fcntl(*pipefds.offset(0 as c_int as isize), 3 as c_int);
    if flags == -(1 as c_int) {
        perror(b"fcntl\0" as *const u8 as *const c_char);
        close(*pipefds.offset(0 as c_int as isize));
        close(*pipefds.offset(1 as c_int as isize));
        return -(1 as c_int);
    }
    r = fcntl(
        *pipefds.offset(0 as c_int as isize),
        4 as c_int,
        flags | 0o4000 as c_int,
    );
    if r == -(1 as c_int) {
        perror(b"fcntl\0" as *const u8 as *const c_char);
        close(*pipefds.offset(0 as c_int as isize));
        close(*pipefds.offset(1 as c_int as isize));
        return -(1 as c_int);
    }
    flags = fcntl(*pipefds.offset(1 as c_int as isize), 3 as c_int);
    if flags == -(1 as c_int) {
        perror(b"fcntl\0" as *const u8 as *const c_char);
        close(*pipefds.offset(0 as c_int as isize));
        close(*pipefds.offset(1 as c_int as isize));
        return -(1 as c_int);
    }
    r = fcntl(
        *pipefds.offset(1 as c_int as isize),
        4 as c_int,
        flags | 0o4000 as c_int,
    );
    if r == -(1 as c_int) {
        perror(b"fcntl\0" as *const u8 as *const c_char);
        close(*pipefds.offset(0 as c_int as isize));
        close(*pipefds.offset(1 as c_int as isize));
        return -(1 as c_int);
    }
    0 as c_int
}

// /* Create non-blocking set of pipe fds.
//    Not supported on Windows! Always fails. */
// int
// pipeutils_create_nonblocking(int pipefds[2])
// {
// 	return -1;
// }

// Signal on write-end of pipe.
#[no_mangle]
pub unsafe extern "C" fn pipeutils_signal(write_fd: c_int) {
    write(
        write_fd,
        b"\0" as *const u8 as *const c_char as *const c_void,
        1 as c_int as size_t,
    );
}

// Mark signal as handled on read-end of pipe.
#[no_mangle]
pub unsafe extern "C" fn pipeutils_handle_signal(read_fd: c_int) {
    let mut data: c_char = 0;
    read(
        read_fd,
        &mut data as *mut c_char as *mut c_void,
        1 as c_int as size_t,
    );
}
