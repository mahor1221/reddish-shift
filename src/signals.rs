/*  signals.rs -- Signal processing source
    This file is part of <https://github.com/mahor1221/reddish-shift>.
    Copyright (C) 2024 Mahor Foruzesh <mahor1221@gmail.com>
    Ported from Redshift <https://github.com/jonls/redshift>.
    Copyright (c) 2009-2015  Jon Lund Steffensen <jonlst@gmail.com>
    Copyright (c) 2015  Mattias Andrée <maandree@member.fsf.org>

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

use crate::sig_atomic_t;
use libc::{
    c_char, c_int, c_long, c_short, c_uint, c_ulong, c_void, clock_t, perror, pid_t, sigaction,
    sigemptyset, sigset_t, uid_t,
};
use std::{mem::MaybeUninit, ptr::addr_of_mut};

// #if defined(HAVE_SIGNAL_H) && !defined(__WIN32__)
#[no_mangle]
pub static mut exiting: sig_atomic_t = 0 as c_int;
#[no_mangle]
pub static mut disable: sig_atomic_t = 0 as c_int;
// Signal handler for exit signals
unsafe extern "C" fn sigexit(mut signo: c_int) {
    ::core::ptr::write_volatile(addr_of_mut!(exiting) as *mut sig_atomic_t, 1 as c_int);
}
// Signal handler for disable signal
unsafe extern "C" fn sigdisable(mut signo: c_int) {
    ::core::ptr::write_volatile(addr_of_mut!(disable) as *mut sig_atomic_t, 1 as c_int);
}
// #else /* ! HAVE_SIGNAL_H || __WIN32__ */
// int disable = 0;
// int exiting = 0;
// #endif /* ! HAVE_SIGNAL_H || __WIN32__ */
#[no_mangle]
pub unsafe extern "C" fn signals_install_handlers() -> c_int {
    let mut sigact: sigaction = sigaction {
        sa_sigaction: 0,
        // __sigaction_handler: C2RustUnnamed_9 { sa_handler: None },
        sa_mask: MaybeUninit::zeroed().assume_init(),
        sa_flags: 0,
        sa_restorer: None,
    };

    let mut sigset: sigset_t = MaybeUninit::zeroed().assume_init();
    let mut r: c_int = 0;
    sigemptyset(&mut sigset);

    // Install signal handler for INT and TERM signals
    // sigact.__sigaction_handler.sa_handler = Some(sigexit as unsafe extern "C" fn(c_int) -> ());
    sigact.sa_sigaction = 0;
    sigact.sa_mask = sigset;
    sigact.sa_flags = 0 as c_int;
    r = sigaction(2 as c_int, &mut sigact, 0 as *mut sigaction);
    if r < 0 as c_int {
        perror(b"sigaction\0" as *const u8 as *const c_char);
        return -(1 as c_int);
    }
    r = sigaction(15 as c_int, &mut sigact, 0 as *mut sigaction);
    if r < 0 as c_int {
        perror(b"sigaction\0" as *const u8 as *const c_char);
        return -(1 as c_int);
    }

    // Install signal handler for USR1 signal
    // sigact.__sigaction_handler.sa_handler = Some(sigdisable as unsafe extern "C" fn(c_int) -> ());
    sigact.sa_sigaction = 0;
    sigact.sa_mask = sigset;
    sigact.sa_flags = 0 as c_int;
    r = sigaction(10 as c_int, &mut sigact, 0 as *mut sigaction);
    if r < 0 as c_int {
        perror(b"sigaction\0" as *const u8 as *const c_char);
        return -(1 as c_int);
    }

    // Ignore CHLD signal. This causes child processes
    // (hooks) to be reaped automatically.
    // sigact.__sigaction_handler.sa_handler =
    //     ::core::mem::transmute::<intptr_t, __sighandler_t>(1 as c_int as intptr_t);
    sigact.sa_sigaction = 1;
    sigact.sa_mask = sigset;
    sigact.sa_flags = 0 as c_int;
    r = sigaction(17 as c_int, &mut sigact, 0 as *mut sigaction);
    if r < 0 as c_int {
        perror(b"sigaction\0" as *const u8 as *const c_char);
        return -(1 as c_int);
    }

    return 0 as c_int;
}
