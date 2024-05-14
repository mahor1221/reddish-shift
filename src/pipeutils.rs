use libc::{close, fcntl, perror, pipe, read, size_t, write};
use std::ffi::{c_char, c_int, c_void};

#[no_mangle]
pub unsafe extern "C" fn pipeutils_create_nonblocking(mut pipefds: *mut c_int) -> c_int {
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
    return 0 as c_int;
}

#[no_mangle]
pub unsafe extern "C" fn pipeutils_signal(mut write_fd: c_int) {
    write(
        write_fd,
        b"\0" as *const u8 as *const c_char as *const c_void,
        1 as c_int as size_t,
    );
}

#[no_mangle]
pub unsafe extern "C" fn pipeutils_handle_signal(mut read_fd: c_int) {
    let mut data: c_char = 0;
    read(
        read_fd,
        &mut data as *mut c_char as *mut c_void,
        1 as c_int as size_t,
    );
}
