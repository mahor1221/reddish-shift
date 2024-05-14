use libc::{
    fclose, fgets, fopen, fputs, free, getenv, getpwuid, getuid, malloc, memcpy, perror, snprintf,
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

unsafe extern "C" fn open_config_file(mut filepath: *const c_char) -> *mut FILE {
    let mut f: *mut FILE = 0 as *mut FILE;
    if filepath.is_null() {
        let mut f_0: *mut FILE = 0 as *mut FILE;
        let mut cp: [c_char; 4096] = [0; 4096];
        let mut env: *mut c_char = 0 as *mut c_char;
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
            let mut pwd: *mut libc::passwd = getpwuid(getuid());
            let mut home: *mut c_char = (*pwd).pw_dir;
            snprintf(
                cp.as_mut_ptr(),
                ::core::mem::size_of::<[c_char; 4096]>(),
                b"%s/.config/redshift/redshift.conf\0" as *const u8 as *const c_char,
                home,
            );
            f_0 = fopen(cp.as_mut_ptr(), b"r\0" as *const u8 as *const c_char);
            if f_0.is_null() {
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
                let mut len: c_int = end.offset_from(begin) as c_long as c_int;
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
            return 0 as *mut FILE;
        }
    }
    return f;
}

#[no_mangle]
pub unsafe extern "C" fn config_ini_init(
    mut state: *mut config_ini_state_t,
    mut filepath: *const c_char,
) -> c_int {
    let mut section: *mut config_ini_section_t = 0 as *mut config_ini_section_t;
    (*state).sections = 0 as *mut config_ini_section_t;
    let mut f: *mut FILE = open_config_file(filepath);
    if f.is_null() {
        if !filepath.is_null() {
            return -(1 as c_int);
        }
        return 0 as c_int;
    }
    let mut line: [c_char; 512] = [0; 512];
    let mut s: *mut c_char = 0 as *mut c_char;
    loop {
        let mut r: *mut c_char = fgets(
            line.as_mut_ptr(),
            ::core::mem::size_of::<[c_char; 512]>() as c_ulong as c_int,
            f,
        );
        if r.is_null() {
            break;
        }
        s = line
            .as_mut_ptr()
            .offset(strspn(line.as_mut_ptr(), b" \t\0" as *const u8 as *const c_char) as isize);
        *s.offset(strcspn(s, b"\r\n\0" as *const u8 as *const c_char) as isize) =
            '\0' as i32 as c_char;
        if *s.offset(0 as c_int as isize) as c_int == ';' as i32
            || *s.offset(0 as c_int as isize) as c_int == '#' as i32
            || *s.offset(0 as c_int as isize) as c_int == '\0' as i32
        {
            continue;
        }
        if *s.offset(0 as c_int as isize) as c_int == '[' as i32 {
            let mut name: *const c_char = s.offset(1 as c_int as isize);
            let mut end: *mut c_char = strchr(s, ']' as i32);
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
            section =
                malloc(::core::mem::size_of::<config_ini_section_t>()) as *mut config_ini_section_t;
            if section.is_null() {
                fclose(f);
                config_ini_free(state);
                return -(1 as c_int);
            }
            (*section).name = 0 as *mut c_char;
            (*section).settings = 0 as *mut config_ini_setting_t;
            (*section).next = (*state).sections;
            (*state).sections = section;
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
            let mut end_0: *mut c_char = strchr(s, '=' as i32);
            if end_0.is_null() || end_0 == s {
                // gettext(
                println!("Malformed assignment in config file.");
                // )
                fclose(f);
                config_ini_free(state);
                return -(1 as c_int);
            }
            *end_0 = '\0' as i32 as c_char;
            let mut value: *mut c_char = end_0.offset(1 as c_int as isize);
            if section.is_null() {
                // gettext(
                eprintln!("Assignment outside section in config file.");
                // ),
                fclose(f);
                config_ini_free(state);
                return -(1 as c_int);
            }
            let mut setting: *mut config_ini_setting_t =
                malloc(::core::mem::size_of::<config_ini_setting_t>()) as *mut config_ini_setting_t;
            if setting.is_null() {
                fclose(f);
                config_ini_free(state);
                return -(1 as c_int);
            }
            (*setting).name = 0 as *mut c_char;
            (*setting).value = 0 as *mut c_char;
            (*setting).next = (*section).settings;
            (*section).settings = setting;
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
            let mut value_len = (strlen(value)).wrapping_add(1);
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
    return 0 as c_int;
}

#[no_mangle]
pub unsafe extern "C" fn config_ini_free(mut state: *mut config_ini_state_t) {
    let mut section: *mut config_ini_section_t = (*state).sections;
    while !section.is_null() {
        let mut setting: *mut config_ini_setting_t = (*section).settings;
        let mut section_prev: *mut config_ini_section_t = section;
        while !setting.is_null() {
            let mut setting_prev: *mut config_ini_setting_t = setting;
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
    mut state: *mut config_ini_state_t,
    mut name: *const c_char,
) -> *mut config_ini_section_t {
    let mut section: *mut config_ini_section_t = (*state).sections;
    while !section.is_null() {
        if strcasecmp((*section).name, name) == 0 as c_int {
            return section;
        }
        section = (*section).next;
    }
    return 0 as *mut config_ini_section_t;
}
