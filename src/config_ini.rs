/*  config-ini.rs -- INI config file parser
    This file is part of <https://github.com/mahor1221/reddish-shift>.
    Copyright (C) 2024 Mahor Foruzesh <mahor1221@gmail.com>
    Ported from Redshift <https://github.com/jonls/redshift>.
    Copyright (c) 2013-2014  Jon Lund Steffensen <jonlst@gmail.com>

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
    fclose, fgets, fopen, free, getenv, getpwuid, getuid, malloc, memcpy, perror, snprintf,
    strcasecmp, strchr, strcspn, strlen, strspn, FILE,
};
use std::ffi::{c_char, c_int, c_long, c_ulong, c_void};

#[derive(Copy, Clone)]
#[repr(C)]
pub struct config_ini_section_t {
    pub next: *mut config_ini_section_t,
    pub name: *mut c_char,
    pub settings: *mut config_ini_setting_t,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct config_ini_setting_t {
    pub next: *mut config_ini_setting_t,
    pub name: *mut c_char,
    pub value: *mut c_char,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct config_ini_state_t {
    pub sections: *mut config_ini_section_t,
}

unsafe extern "C" fn open_config_file(filepath: *const c_char) -> *mut FILE {
    // If a path is not specified (filepath is NULL) then
    // the configuration file is searched for in the directories
    // specified by the XDG Base Directory Specification
    // <http://standards.freedesktop.org/basedir-spec/basedir-spec-latest.html>.

    // If HOME is not set, getpwuid() is consulted for the home directory. On
    // windows platforms the %localappdata% is used in place of XDG_CONFIG_HOME.
    let mut f: *mut FILE = std::ptr::null_mut::<FILE>();
    if filepath.is_null() {
        let mut f_0: *mut FILE = std::ptr::null_mut::<FILE>();
        let mut cp: [c_char; 4096] = [0; 4096];
        let mut env: *mut c_char = std::ptr::null_mut::<c_char>();
        if f_0.is_null()
            && {
                env = getenv(b"XDG_CONFIG_HOME\0" as *const u8 as *const c_char);
                !env.is_null()
            }
            && *env.offset(0 as c_int as isize) as c_int != '\0' as i32
        {
            snprintf(
                cp.as_mut_ptr(),
                ::core::mem::size_of::<[c_char; 4096]>(),
                b"%s/redshift/redshift.conf\0" as *const u8 as *const c_char,
                env,
            );
            f_0 = fopen(cp.as_mut_ptr(), b"r\0" as *const u8 as *const c_char);
            if f_0.is_null() {
                // Fall back to formerly used path.
                snprintf(
                    cp.as_mut_ptr(),
                    ::core::mem::size_of::<[c_char; 4096]>(),
                    b"%s/redshift.conf\0" as *const u8 as *const c_char,
                    env,
                );
                f_0 = fopen(cp.as_mut_ptr(), b"r\0" as *const u8 as *const c_char);
            }
        }
        if f_0.is_null()
            && {
                env = getenv(b"HOME\0" as *const u8 as *const c_char);
                !env.is_null()
            }
            && *env.offset(0 as c_int as isize) as c_int != '\0' as i32
        {
            snprintf(
                cp.as_mut_ptr(),
                ::core::mem::size_of::<[c_char; 4096]>(),
                b"%s/.config/redshift/redshift.conf\0" as *const u8 as *const c_char,
                env,
            );
            f_0 = fopen(cp.as_mut_ptr(), b"r\0" as *const u8 as *const c_char);
            if f_0.is_null() {
                // Fall back to formerly used path.
                snprintf(
                    cp.as_mut_ptr(),
                    ::core::mem::size_of::<[c_char; 4096]>(),
                    b"%s/.config/redshift.conf\0" as *const u8 as *const c_char,
                    env,
                );
                f_0 = fopen(cp.as_mut_ptr(), b"r\0" as *const u8 as *const c_char);
            }
        }
        if f_0.is_null() {
            let pwd: *mut libc::passwd = getpwuid(getuid());
            let home: *mut c_char = (*pwd).pw_dir;
            snprintf(
                cp.as_mut_ptr(),
                ::core::mem::size_of::<[c_char; 4096]>(),
                b"%s/.config/redshift/redshift.conf\0" as *const u8 as *const c_char,
                home,
            );
            f_0 = fopen(cp.as_mut_ptr(), b"r\0" as *const u8 as *const c_char);
            if f_0.is_null() {
                // Fall back to formerly used path.
                snprintf(
                    cp.as_mut_ptr(),
                    ::core::mem::size_of::<[c_char; 4096]>(),
                    b"%s/.config/redshift.conf\0" as *const u8 as *const c_char,
                    home,
                );
                f_0 = fopen(cp.as_mut_ptr(), b"r\0" as *const u8 as *const c_char);
            }
        }
        if f_0.is_null()
            && {
                env = getenv(b"XDG_CONFIG_DIRS\0" as *const u8 as *const c_char);
                !env.is_null()
            }
            && *env.offset(0 as c_int as isize) as c_int != '\0' as i32
        {
            let mut begin: *mut c_char = env;
            loop {
                let mut end: *mut c_char = strchr(begin, ':' as i32);
                if end.is_null() {
                    end = strchr(begin, '\0' as i32);
                }
                let len: c_int = end.offset_from(begin) as c_long as c_int;
                if len > 0 as c_int {
                    snprintf(
                        cp.as_mut_ptr(),
                        ::core::mem::size_of::<[c_char; 4096]>(),
                        b"%.*s/redshift/redshift.conf\0" as *const u8 as *const c_char,
                        len,
                        begin,
                    );
                    f_0 = fopen(cp.as_mut_ptr(), b"r\0" as *const u8 as *const c_char);
                    if !f_0.is_null() {
                        // Fall back to formerly used path.
                        snprintf(
                            cp.as_mut_ptr(),
                            ::core::mem::size_of::<[c_char; 4096]>(),
                            b"%.*s/redshift.conf\0" as *const u8 as *const c_char,
                            len,
                            begin,
                        );
                        f_0 = fopen(cp.as_mut_ptr(), b"r\0" as *const u8 as *const c_char);
                    }
                    if !f_0.is_null() {
                        break;
                    }
                }
                if *end.offset(0 as c_int as isize) as c_int == '\0' as i32 {
                    break;
                }
                begin = end.offset(1 as c_int as isize);
            }
        }
        if f_0.is_null() {
            snprintf(
                cp.as_mut_ptr(),
                ::core::mem::size_of::<[c_char; 4096]>(),
                b"%s/redshift.conf\0" as *const u8 as *const c_char,
                b"/etc\0" as *const u8 as *const c_char,
            );
            f_0 = fopen(cp.as_mut_ptr(), b"r\0" as *const u8 as *const c_char);
        }
        return f_0;
    } else {
        f = fopen(filepath, b"r\0" as *const u8 as *const c_char);
        if f.is_null() {
            perror(b"fopen\0" as *const u8 as *const c_char);
            return std::ptr::null_mut::<FILE>();
        }
    }
    f
}

#[no_mangle]
pub unsafe extern "C" fn config_ini_init(
    state: *mut config_ini_state_t,
    filepath: *const c_char,
) -> c_int {
    let mut section: *mut config_ini_section_t = std::ptr::null_mut::<config_ini_section_t>();
    (*state).sections = std::ptr::null_mut::<config_ini_section_t>();
    let f: *mut FILE = open_config_file(filepath);
    if f.is_null() {
        // Only a serious error if a file was explicitly requested.
        if !filepath.is_null() {
            return -(1 as c_int);
        }
        return 0 as c_int;
    }
    let mut line: [c_char; 512] = [0; 512];
    let mut s: *mut c_char = std::ptr::null_mut::<c_char>();
    loop {
        // Handle the file input linewise.
        let r: *mut c_char = fgets(
            line.as_mut_ptr(),
            ::core::mem::size_of::<[c_char; 512]>() as c_ulong as c_int,
            f,
        );
        if r.is_null() {
            break;
        }
        // Strip leading blanks and trailing newline.
        s = line.as_mut_ptr().add(strspn(
            line.as_mut_ptr(),
            b" \t\0" as *const u8 as *const c_char,
        ));
        *s.add(strcspn(s, b"\r\n\0" as *const u8 as *const c_char)) = '\0' as i32 as c_char;
        // Skip comments and empty lines.
        if *s.offset(0 as c_int as isize) as c_int == ';' as i32
            || *s.offset(0 as c_int as isize) as c_int == '#' as i32
            || *s.offset(0 as c_int as isize) as c_int == '\0' as i32
        {
            continue;
        }
        if *s.offset(0 as c_int as isize) as c_int == '[' as i32 {
            // Read name of section.
            let name: *const c_char = s.offset(1 as c_int as isize);
            let end: *mut c_char = strchr(s, ']' as i32);
            if end.is_null()
                || *end.offset(1 as c_int as isize) as c_int != '\0' as i32
                || end == name as *mut c_char
            {
                // gettext(
                eprintln!("Malformed section header in config file.");
                // ),
                fclose(f);
                config_ini_free(state);
                return -(1 as c_int);
            }
            *end = '\0' as i32 as c_char;
            // Create section.
            section =
                malloc(::core::mem::size_of::<config_ini_section_t>()) as *mut config_ini_section_t;
            if section.is_null() {
                fclose(f);
                config_ini_free(state);
                return -(1 as c_int);
            }
            // Insert into section list.
            (*section).name = std::ptr::null_mut::<c_char>();
            (*section).settings = std::ptr::null_mut::<config_ini_setting_t>();
            (*section).next = (*state).sections;
            (*state).sections = section;
            // Copy section name.
            (*section).name = malloc((end.offset_from(name) + 1) as usize) as *mut c_char;
            if ((*section).name).is_null() {
                fclose(f);
                config_ini_free(state);
                return -(1 as c_int);
            }
            memcpy(
                (*section).name as *mut c_void,
                name as *const c_void,
                (end.offset_from(name) + 1) as usize,
            );
        } else {
            // Split assignment at equals character.
            let end_0: *mut c_char = strchr(s, '=' as i32);
            if end_0.is_null() || end_0 == s {
                // gettext(
                println!("Malformed assignment in config file.");
                // )
                fclose(f);
                config_ini_free(state);
                return -(1 as c_int);
            }
            *end_0 = '\0' as i32 as c_char;
            let value: *mut c_char = end_0.offset(1 as c_int as isize);
            if section.is_null() {
                // gettext(
                eprintln!("Assignment outside section in config file.");
                // ),
                fclose(f);
                config_ini_free(state);
                return -(1 as c_int);
            }
            // Create section.
            let setting: *mut config_ini_setting_t =
                malloc(::core::mem::size_of::<config_ini_setting_t>()) as *mut config_ini_setting_t;
            if setting.is_null() {
                fclose(f);
                config_ini_free(state);
                return -(1 as c_int);
            }
            // Insert into section list.
            (*setting).name = std::ptr::null_mut::<c_char>();
            (*setting).value = std::ptr::null_mut::<c_char>();
            (*setting).next = (*section).settings;
            (*section).settings = setting;
            // Copy name of setting.
            (*setting).name = malloc((end_0.offset_from(s) + 1) as usize) as *mut c_char;
            if ((*setting).name).is_null() {
                fclose(f);
                config_ini_free(state);
                return -(1 as c_int);
            }
            memcpy(
                (*setting).name as *mut c_void,
                s as *const c_void,
                (end_0.offset_from(s) + 1) as usize,
            );
            // Copy setting value.
            let value_len = (strlen(value)).wrapping_add(1);
            (*setting).value = malloc(value_len) as *mut c_char;
            if ((*setting).value).is_null() {
                fclose(f);
                config_ini_free(state);
                return -(1 as c_int);
            }
            memcpy(
                (*setting).value as *mut c_void,
                value as *const c_void,
                value_len,
            );
        }
    }
    fclose(f);
    0 as c_int
}

#[no_mangle]
pub unsafe extern "C" fn config_ini_free(state: *mut config_ini_state_t) {
    let mut section: *mut config_ini_section_t = (*state).sections;
    while !section.is_null() {
        let mut setting: *mut config_ini_setting_t = (*section).settings;
        let section_prev: *mut config_ini_section_t = section;
        while !setting.is_null() {
            let setting_prev: *mut config_ini_setting_t = setting;
            free((*setting).name as *mut c_void);
            free((*setting).value as *mut c_void);
            setting = (*setting).next;
            free(setting_prev as *mut c_void);
        }
        free((*section).name as *mut c_void);
        section = (*section).next;
        free(section_prev as *mut c_void);
    }
}

#[no_mangle]
pub unsafe extern "C" fn config_ini_get_section(
    state: *mut config_ini_state_t,
    name: *const c_char,
) -> *mut config_ini_section_t {
    let mut section: *mut config_ini_section_t = (*state).sections;
    while !section.is_null() {
        if strcasecmp((*section).name, name) == 0 as c_int {
            return section;
        }
        section = (*section).next;
    }
    std::ptr::null_mut::<config_ini_section_t>()
}
