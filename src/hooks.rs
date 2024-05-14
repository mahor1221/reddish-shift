use crate::{period_names, period_t};
use libc::{
    __errno_location, _exit, close, dirent, execl, fork, getenv, getpwuid, getuid, gid_t, opendir,
    perror, pid_t, readdir, snprintf, uid_t, DIR,
};
use std::ffi::{c_char, c_int, c_long, c_uchar, c_uint, c_ulong, c_ushort, c_void};

unsafe extern "C" fn open_hooks_dir(mut hp: *mut c_char) -> *mut DIR {
    let mut env: *mut c_char = 0 as *mut c_char;
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
    let mut pwd: *mut libc::passwd = getpwuid(getuid());
    snprintf(
        hp,
        4096,
        b"%s/.config/redshift/hooks\0" as *const u8 as *const c_char,
        (*pwd).pw_dir,
    );
    return opendir(hp);
}

#[no_mangle]
pub unsafe extern "C" fn hooks_signal_period_change(
    mut prev_period: period_t,
    mut period: period_t,
) {
    let mut hooksdir_path: [c_char; 4096] = [0; 4096];
    let mut hooks_dir: *mut DIR = open_hooks_dir(hooksdir_path.as_mut_ptr());
    if hooks_dir.is_null() {
        return;
    }
    let mut ent: *mut dirent = 0 as *mut dirent;
    loop {
        ent = readdir(hooks_dir);
        if ent.is_null() {
            break;
        }
        if (*ent).d_name[0 as c_int as usize] as c_int == '\0' as i32
            || (*ent).d_name[0 as c_int as usize] as c_int == '.' as i32
        {
            continue;
        }
        let mut hook_name: *mut c_char = ((*ent).d_name).as_mut_ptr();
        let mut hook_path: [c_char; 4096] = [0; 4096];
        snprintf(
            hook_path.as_mut_ptr(),
            ::core::mem::size_of::<[c_char; 4096]>(),
            b"%s/%s\0" as *const u8 as *const c_char,
            hooksdir_path.as_mut_ptr(),
            hook_name,
        );
        let mut pid: pid_t = fork();
        if pid == -(1 as c_int) {
            perror(b"fork\0" as *const u8 as *const c_char);
        } else if pid == 0 as c_int {
            close(1 as c_int);
            let mut r: c_int = execl(
                hook_path.as_mut_ptr(),
                hook_name,
                b"period-changed\0" as *const u8 as *const c_char,
                period_names[prev_period as usize],
                period_names[period as usize],
                0 as *mut c_void,
            );
            if r < 0 as c_int && *__errno_location() != 13 as c_int {
                perror(b"execl\0" as *const u8 as *const c_char);
            }
            _exit(1 as c_int);
        }
    }
}
