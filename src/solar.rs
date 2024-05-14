use std::ffi::{c_double, c_int, c_uint};

pub type C2RustUnnamed = c_uint;
pub const SOLAR_TIME_MAX: C2RustUnnamed = 10;
pub const SOLAR_TIME_ASTRO_DUSK: C2RustUnnamed = 9;
pub const SOLAR_TIME_NAUT_DUSK: C2RustUnnamed = 8;
pub const SOLAR_TIME_CIVIL_DUSK: C2RustUnnamed = 7;
pub const SOLAR_TIME_SUNSET: C2RustUnnamed = 6;
pub const SOLAR_TIME_SUNRISE: C2RustUnnamed = 5;
pub const SOLAR_TIME_CIVIL_DAWN: C2RustUnnamed = 4;
pub const SOLAR_TIME_NAUT_DAWN: C2RustUnnamed = 3;
pub const SOLAR_TIME_ASTRO_DAWN: C2RustUnnamed = 2;
pub const SOLAR_TIME_MIDNIGHT: C2RustUnnamed = 1;
pub const SOLAR_TIME_NOON: C2RustUnnamed = 0;

static mut time_angle: [c_double; 10] = [
    0.0f64 * (3.14159265358979323846f64 / 180 as c_int as c_double),
    0.,
    (-90.0f64 + -18.0f64) * (3.14159265358979323846f64 / 180 as c_int as c_double),
    (-90.0f64 + -12.0f64) * (3.14159265358979323846f64 / 180 as c_int as c_double),
    (-90.0f64 + -6.0f64) * (3.14159265358979323846f64 / 180 as c_int as c_double),
    (-90.0f64 + (0.0f64 - 0.833f64)) * (3.14159265358979323846f64 / 180 as c_int as c_double),
    (90.0f64 - (0.0f64 - 0.833f64)) * (3.14159265358979323846f64 / 180 as c_int as c_double),
    (90.0f64 - -6.0f64) * (3.14159265358979323846f64 / 180 as c_int as c_double),
    (90.0f64 - -12.0f64) * (3.14159265358979323846f64 / 180 as c_int as c_double),
    (90.0f64 - -18.0f64) * (3.14159265358979323846f64 / 180 as c_int as c_double),
];

unsafe extern "C" fn epoch_from_jd(mut jd: c_double) -> c_double {
    return 86400.0f64 * (jd - 2440587.5f64);
}

unsafe extern "C" fn jd_from_epoch(mut t: c_double) -> c_double {
    return t / 86400.0f64 + 2440587.5f64;
}

unsafe extern "C" fn jcent_from_jd(mut jd: c_double) -> c_double {
    return (jd - 2451545.0f64) / 36525.0f64;
}

unsafe extern "C" fn jd_from_jcent(mut t: c_double) -> c_double {
    return 36525.0f64 * t + 2451545.0f64;
}

unsafe extern "C" fn sun_geom_mean_lon(mut t: c_double) -> c_double {
    // return fmod(
    //     280.46646f64 + t * (36000.76983f64 + t * 0.0003032f64),
    //     360 as c_int as c_double,
    // ) * (3.14159265358979323846f64 / 180 as c_int as c_double);
    ((280.46646_f64 + t * (36000.76983_f64 + t * 0.0003032_f64)) % 360_f64)
        * (3.14159265358979323846_f64 / 180_f64)
}

unsafe extern "C" fn sun_geom_mean_anomaly(mut t: c_double) -> c_double {
    return (357.52911f64 + t * (35999.05029f64 - t * 0.0001537f64))
        * (3.14159265358979323846f64 / 180 as c_int as c_double);
}

unsafe extern "C" fn earth_orbit_eccentricity(mut t: c_double) -> c_double {
    return 0.016708634f64 - t * (0.000042037f64 + t * 0.0000001267f64);
}

unsafe extern "C" fn sun_equation_of_center(mut t: c_double) -> c_double {
    let mut m: c_double = sun_geom_mean_anomaly(t);
    let mut c: c_double = (m).sin() * (1.914602f64 - t * (0.004817f64 + 0.000014f64 * t))
        + (2 as c_int as c_double * m).sin() * (0.019993f64 - 0.000101f64 * t)
        + (3 as c_int as c_double * m).sin() * 0.000289f64;
    return c * (3.14159265358979323846f64 / 180 as c_int as c_double);
}

unsafe extern "C" fn sun_true_lon(mut t: c_double) -> c_double {
    let mut l_0: c_double = sun_geom_mean_lon(t);
    let mut c: c_double = sun_equation_of_center(t);
    return l_0 + c;
}

unsafe extern "C" fn sun_apparent_lon(mut t: c_double) -> c_double {
    let mut o: c_double = sun_true_lon(t);
    return (o * (180 as c_int as c_double / 3.14159265358979323846f64)
        - 0.00569f64
        - 0.00478f64
            * ((125.04f64 - 1934.136f64 * t)
                * (3.14159265358979323846f64 / 180 as c_int as c_double))
                .sin())
        * (3.14159265358979323846f64 / 180 as c_int as c_double);
}

unsafe extern "C" fn mean_ecliptic_obliquity(mut t: c_double) -> c_double {
    let mut sec: c_double = 21.448f64 - t * (46.815f64 + t * (0.00059f64 - t * 0.001813f64));
    return (23.0f64 + (26.0f64 + sec / 60.0f64) / 60.0f64)
        * (3.14159265358979323846f64 / 180 as c_int as c_double);
}

unsafe extern "C" fn obliquity_corr(mut t: c_double) -> c_double {
    let mut e_0: c_double = mean_ecliptic_obliquity(t);
    let mut omega: c_double = 125.04f64 - t * 1934.136f64;
    return (e_0 * (180 as c_int as c_double / 3.14159265358979323846f64)
        + 0.00256f64 * (omega * (3.14159265358979323846f64 / 180 as c_int as c_double).cos()))
        * (3.14159265358979323846f64 / 180 as c_int as c_double);
}

unsafe extern "C" fn solar_declination(mut t: c_double) -> c_double {
    let mut e: c_double = obliquity_corr(t);
    let mut lambda: c_double = sun_apparent_lon(t);
    return ((e).sin() * (lambda)).sin();
}

unsafe extern "C" fn equation_of_time(mut t: c_double) -> c_double {
    let mut epsilon: c_double = obliquity_corr(t);
    let mut l_0: c_double = sun_geom_mean_lon(t);
    let mut e: c_double = earth_orbit_eccentricity(t);
    let mut m: c_double = sun_geom_mean_anomaly(t);
    let mut y: c_double = (epsilon / 2.0f64).tan().powf(2.0f64);
    let mut eq_time: c_double = y * (2 as c_int as c_double * l_0).sin()
        - 2 as c_int as c_double * e * (m).sin()
        + 4 as c_int as c_double * e * y * (m).sin() * (2 as c_int as c_double * l_0.cos())
        - 0.5f64 * y * y * (4 as c_int as c_double * l_0).sin()
        - 1.25f64 * e * e * (2 as c_int as c_double * m).sin();
    return 4 as c_int as c_double
        * (eq_time * (180 as c_int as c_double / 3.14159265358979323846f64));
}

unsafe extern "C" fn hour_angle_from_elevation(
    mut lat: c_double,
    mut decl: c_double,
    mut elev: c_double,
) -> c_double {
    let mut omega: c_double = (elev.abs().cos()
        - (lat * (3.14159265358979323846f64 / 180 as c_int as c_double)).sin() * (decl).sin())
        / ((lat * (3.14159265358979323846f64 / 180 as c_int as c_double).cos()) * (decl.cos()))
            .cos();
    return omega.copysign(-elev);
}

unsafe extern "C" fn elevation_from_hour_angle(
    mut lat: c_double,
    mut decl: c_double,
    mut ha: c_double,
) -> c_double {
    return ((ha.cos())
        * (lat * (3.14159265358979323846f64 / 180 as c_int as c_double).cos())
        * (decl.cos())
        + (lat * (3.14159265358979323846f64 / 180 as c_int as c_double)).sin() * (decl).sin())
    .sin();
}

unsafe extern "C" fn time_of_solar_noon(mut t: c_double, mut lon: c_double) -> c_double {
    let mut t_noon: c_double = jcent_from_jd(jd_from_jcent(t) - lon / 360.0f64);
    let mut eq_time: c_double = equation_of_time(t_noon);
    let mut sol_noon: c_double = 720 as c_int as c_double - 4 as c_int as c_double * lon - eq_time;
    t_noon = jcent_from_jd(jd_from_jcent(t) - 0.5f64 + sol_noon / 1440.0f64);
    eq_time = equation_of_time(t_noon);
    sol_noon = 720 as c_int as c_double - 4 as c_int as c_double * lon - eq_time;
    return sol_noon;
}

unsafe extern "C" fn time_of_solar_elevation(
    mut t: c_double,
    mut t_noon: c_double,
    mut lat: c_double,
    mut lon: c_double,
    mut elev: c_double,
) -> c_double {
    let mut eq_time: c_double = equation_of_time(t_noon);
    let mut sol_decl: c_double = solar_declination(t_noon);
    let mut ha: c_double = hour_angle_from_elevation(lat, sol_decl, elev);
    let mut sol_offset: c_double = 720 as c_int as c_double
        - 4 as c_int as c_double
            * (lon + ha * (180 as c_int as c_double / 3.14159265358979323846f64))
        - eq_time;
    let mut t_rise: c_double = jcent_from_jd(jd_from_jcent(t) + sol_offset / 1440.0f64);
    eq_time = equation_of_time(t_rise);
    sol_decl = solar_declination(t_rise);
    ha = hour_angle_from_elevation(lat, sol_decl, elev);
    sol_offset = 720 as c_int as c_double
        - 4 as c_int as c_double
            * (lon + ha * (180 as c_int as c_double / 3.14159265358979323846f64))
        - eq_time;
    return sol_offset;
}

unsafe extern "C" fn solar_elevation_from_time(
    mut t: c_double,
    mut lat: c_double,
    mut lon: c_double,
) -> c_double {
    let mut jd: c_double = jd_from_jcent(t);
    let mut offset: c_double = (jd - jd.round() - 0.5f64) * 1440.0f64;
    let mut eq_time: c_double = equation_of_time(t);
    let mut ha: c_double = ((720 as c_int as c_double - offset - eq_time) / 4 as c_int as c_double
        - lon)
        * (3.14159265358979323846f64 / 180 as c_int as c_double);
    let mut decl: c_double = solar_declination(t);
    return elevation_from_hour_angle(lat, decl, ha);
}

#[no_mangle]
pub unsafe extern "C" fn solar_elevation(
    mut date: c_double,
    mut lat: c_double,
    mut lon: c_double,
) -> c_double {
    let mut jd: c_double = jd_from_epoch(date);
    return solar_elevation_from_time(jcent_from_jd(jd), lat, lon)
        * (180 as c_int as c_double / 3.14159265358979323846f64);
}

#[no_mangle]
pub unsafe extern "C" fn solar_table_fill(
    mut date: c_double,
    mut lat: c_double,
    mut lon: c_double,
    mut table: *mut c_double,
) {
    let mut jd: c_double = jd_from_epoch(date);
    let mut jdn: c_double = jd.round();
    let mut t: c_double = jcent_from_jd(jdn);
    let mut sol_noon: c_double = time_of_solar_noon(t, lon);
    let mut j_noon: c_double = jdn - 0.5f64 + sol_noon / 1440.0f64;
    let mut t_noon: c_double = jcent_from_jd(j_noon);
    *table.offset(SOLAR_TIME_NOON as c_int as isize) = epoch_from_jd(j_noon);
    *table.offset(SOLAR_TIME_MIDNIGHT as c_int as isize) = epoch_from_jd(j_noon + 0.5f64);
    let mut i: c_int = 2 as c_int;
    while i < SOLAR_TIME_MAX as c_int {
        let mut angle: c_double = time_angle[i as usize];
        let mut offset: c_double = time_of_solar_elevation(t, t_noon, lat, lon, angle);
        *table.offset(i as isize) = epoch_from_jd(jdn - 0.5f64 + offset / 1440.0f64);
        i += 1;
        i;
    }
}
