/*  hooks.rs -- Hooks triggered by events
    This file is part of <https://github.com/mahor1221/reddish-shift>.
    Copyright (C) 2024 Mahor Foruzesh <mahor1221@gmail.com>
    Ported from Redshift <https://github.com/jonls/redshift>.
    Copyright (c) 2014  Jon Lund Steffensen <jonlst@gmail.com>

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

use libc::{
    __errno_location, _exit, close, dirent, execl, fork, getenv, getpwuid, getuid, opendir, perror,
    pid_t, readdir, snprintf, DIR,
};
use std::ffi::{c_char, c_int, c_void};

use crate::{period_names, Period};

// Try to open the directory containing hooks. HP is a string
// of MAX_HOOK_PATH length that will be filled with the path
// of the returned directory.
unsafe extern "C" fn open_hooks_dir(hp: *mut c_char) -> *mut DIR {
    let mut env: *mut c_char = std::ptr::null_mut::<c_char>();
    env = getenv(b"XDG_CONFIG_HOME\0" as *const u8 as *const c_char);
    if !env.is_null() && *env.offset(0 as c_int as isize) as c_int != '\0' as i32 {
        snprintf(
            hp,
            4096,
            b"%s/redshift/hooks\0" as *const u8 as *const c_char,
            env,
        );
        return opendir(hp);
    }
    env = getenv(b"HOME\0" as *const u8 as *const c_char);
    if !env.is_null() && *env.offset(0 as c_int as isize) as c_int != '\0' as i32 {
        snprintf(
            hp,
            4096,
            b"%s/.config/redshift/hooks\0" as *const u8 as *const c_char,
            env,
        );
        return opendir(hp);
    }
    let pwd: *mut libc::passwd = getpwuid(getuid());
    snprintf(
        hp,
        4096,
        b"%s/.config/redshift/hooks\0" as *const u8 as *const c_char,
        (*pwd).pw_dir,
    );
    opendir(hp)
}

// Run hooks with a signal that the period changed.
#[no_mangle]
pub unsafe extern "C" fn hooks_signal_period_change(prev_period: Period, period: Period) {
    let mut hooksdir_path: [c_char; 4096] = [0; 4096];
    let hooks_dir: *mut DIR = open_hooks_dir(hooksdir_path.as_mut_ptr());
    if hooks_dir.is_null() {
        return;
    }
    let mut ent: *mut dirent = std::ptr::null_mut::<dirent>();
    loop {
        ent = readdir(hooks_dir);
        // Skip hidden and special files (., ..)
        if ent.is_null() {
            break;
        }
        if (*ent).d_name[0 as c_int as usize] as c_int == '\0' as i32
            || (*ent).d_name[0 as c_int as usize] as c_int == '.' as i32
        {
            continue;
        }
        let hook_name: *mut c_char = ((*ent).d_name).as_mut_ptr();
        let mut hook_path: [c_char; 4096] = [0; 4096];
        snprintf(
            hook_path.as_mut_ptr(),
            ::core::mem::size_of::<[c_char; 4096]>(),
            b"%s/%s\0" as *const u8 as *const c_char,
            hooksdir_path.as_mut_ptr(),
            hook_name,
        );

        // #ifndef _WIN32
        // Fork and exec the hook. We close stdout
        // so the hook cannot interfere with the normal
        // output.
        let pid: pid_t = fork();
        if pid == -(1 as c_int) {
            perror(b"fork\0" as *const u8 as *const c_char);
        } else if pid == 0 as c_int {
            close(1 as c_int);
            let r: c_int = execl(
                hook_path.as_mut_ptr(),
                hook_name,
                b"period-changed\0" as *const u8 as *const c_char,
                period_names[prev_period as usize],
                period_names[period as usize],
                std::ptr::null_mut::<c_void>(),
            );
            if r < 0 as c_int && *__errno_location() != 13 as c_int {
                perror(b"execl\0" as *const u8 as *const c_char);
            }
            _exit(1 as c_int);
        }
        // #endif
    }
}
