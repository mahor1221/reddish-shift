/*  solar.rs -- Solar position
    This file is part of <https://github.com/mahor1221/reddish-shift>.
    Copyright (C) 2024 Mahor Foruzesh <mahor1221@gmail.com>
    Ported from Redshift <https://github.com/jonls/redshift>.
    Copyright (c) 2010  Jon Lund Steffensen <jonlst@gmail.com>

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

/* Ported from javascript code by U.S. Department of Commerce,
   National Oceanic & Atmospheric Administration:
   http://www.srrb.noaa.gov/highlights/sunrise/calcdetails.html
   It is based on equations from "Astronomical Algorithms" by
   Jean Meeus.
*/

use std::ffi::{c_double, c_int, c_uint};

// Model of atmospheric refraction near horizon (in degrees).
const SOLAR_ATM_REFRAC: f64 = 0.833;

const SOLAR_ASTRO_TWILIGHT_ELEV: f64 = -18.0;
const SOLAR_NAUT_TWILIGHT_ELEV: f64 = -12.0;
pub const SOLAR_CIVIL_TWILIGHT_ELEV: f32 = -6.0;
const SOLAR_DAYTIME_ELEV: f64 = 0.0 - SOLAR_ATM_REFRAC;

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

// #define RAD(x)  ((x)*(M_PI/180))
// #define DEG(x)  ((x)*(180/M_PI))

// Angels of various times of day.
static mut time_angle: [c_double; 10] = [
    0.0f64 * (3.141_592_653_589_793_f64 / 180 as c_int as c_double),
    0.,
    (-90.0f64 + -18.0f64) * (3.141_592_653_589_793_f64 / 180 as c_int as c_double),
    (-90.0f64 + -12.0f64) * (3.141_592_653_589_793_f64 / 180 as c_int as c_double),
    (-90.0f64 + -6.0f64) * (3.141_592_653_589_793_f64 / 180 as c_int as c_double),
    (-90.0f64 + (0.0f64 - 0.833f64)) * (3.141_592_653_589_793_f64 / 180 as c_int as c_double),
    (90.0f64 - (0.0f64 - 0.833f64)) * (3.141_592_653_589_793_f64 / 180 as c_int as c_double),
    (90.0f64 - -6.0f64) * (3.141_592_653_589_793_f64 / 180 as c_int as c_double),
    (90.0f64 - -12.0f64) * (3.141_592_653_589_793_f64 / 180 as c_int as c_double),
    (90.0f64 - -18.0f64) * (3.141_592_653_589_793_f64 / 180 as c_int as c_double),
];

// Unix epoch from Julian day
unsafe extern "C" fn epoch_from_jd(jd: c_double) -> c_double {
    86400.0f64 * (jd - 2440587.5f64)
}

// Julian day from unix epoch
unsafe extern "C" fn jd_from_epoch(t: c_double) -> c_double {
    t / 86400.0f64 + 2440587.5f64
}

// Julian centuries since J2000.0 from Julian day
unsafe extern "C" fn jcent_from_jd(jd: c_double) -> c_double {
    (jd - 2451545.0f64) / 36525.0f64
}

// Julian day from Julian centuries since J2000.0
unsafe extern "C" fn jd_from_jcent(t: c_double) -> c_double {
    36525.0f64 * t + 2451545.0f64
}

// Geometric mean longitude of the sun.
// t: Julian centuries since J2000.0
//   Return: Geometric mean longitude in radians.
unsafe extern "C" fn sun_geom_mean_lon(t: c_double) -> c_double {
    // return fmod(
    //     280.46646f64 + t * (36000.76983f64 + t * 0.0003032f64),
    //     360 as c_int as c_double,
    // ) * (3.14159265358979323846f64 / 180 as c_int as c_double);
    ((280.46646_f64 + t * (36000.76983_f64 + t * 0.0003032_f64)) % 360_f64)
        * (3.141_592_653_589_793_f64 / 180_f64)
    // FIXME returned value should always be positive
}

// Geometric mean anomaly of the sun.
// t: Julian centuries since J2000.0
// Return: Geometric mean anomaly in radians.
unsafe extern "C" fn sun_geom_mean_anomaly(t: c_double) -> c_double {
    (357.52911f64 + t * (35999.05029f64 - t * 0.0001537f64))
        * (3.141_592_653_589_793_f64 / 180 as c_int as c_double)
}

// Eccentricity of earth orbit.
// t: Julian centuries since J2000.0
// Return: Eccentricity (unitless).
unsafe extern "C" fn earth_orbit_eccentricity(t: c_double) -> c_double {
    0.016708634f64 - t * (0.000042037f64 + t * 0.0000001267f64)
}

// Equation of center of the sun.
// t: Julian centuries since J2000.0
// Return: Center(?) in radians
unsafe extern "C" fn sun_equation_of_center(t: c_double) -> c_double {
    let m: c_double = sun_geom_mean_anomaly(t);
    let c: c_double = (m).sin() * (1.914602f64 - t * (0.004817f64 + 0.000014f64 * t))
        + (2 as c_int as c_double * m).sin() * (0.019993f64 - 0.000101f64 * t)
        + (3 as c_int as c_double * m).sin() * 0.000289f64;
    c * (3.141_592_653_589_793_f64 / 180 as c_int as c_double)
}

// True longitude of the sun.
// t: Julian centuries since J2000.0
// Return: True longitude in radians
unsafe extern "C" fn sun_true_lon(t: c_double) -> c_double {
    let l_0: c_double = sun_geom_mean_lon(t);
    let c: c_double = sun_equation_of_center(t);
    l_0 + c
}

// Apparent longitude of the sun. (Right ascension).
// t: Julian centuries since J2000.0
// Return: Apparent longitude in radians
unsafe extern "C" fn sun_apparent_lon(t: c_double) -> c_double {
    let o: c_double = sun_true_lon(t);
    (o * (180 as c_int as c_double / 3.141_592_653_589_793_f64)
        - 0.00569f64
        - 0.00478f64
            * ((125.04f64 - 1934.136f64 * t)
                * (3.141_592_653_589_793_f64 / 180 as c_int as c_double))
                .sin())
        * (3.141_592_653_589_793_f64 / 180 as c_int as c_double)
}

// Mean obliquity of the ecliptic
// t: Julian centuries since J2000.0
// Return: Mean obliquity in radians
unsafe extern "C" fn mean_ecliptic_obliquity(t: c_double) -> c_double {
    let sec: c_double = 21.448f64 - t * (46.815f64 + t * (0.00059f64 - t * 0.001813f64));
    (23.0f64 + (26.0f64 + sec / 60.0f64) / 60.0f64)
        * (3.141_592_653_589_793_f64 / 180 as c_int as c_double)
}

// Corrected obliquity of the ecliptic.
// t: Julian centuries since J2000.0
// Return: Corrected obliquity in radians
unsafe extern "C" fn obliquity_corr(t: c_double) -> c_double {
    let e_0: c_double = mean_ecliptic_obliquity(t);
    let omega: c_double = 125.04f64 - t * 1934.136f64;
    (e_0 * (180 as c_int as c_double / 3.141_592_653_589_793_f64)
        + 0.00256f64 * (omega * (3.141_592_653_589_793_f64 / 180 as c_int as c_double).cos()))
        * (3.141_592_653_589_793_f64 / 180 as c_int as c_double)
}

// Declination of the sun.
// t: Julian centuries since J2000.0
// Return: Declination in radians
unsafe extern "C" fn solar_declination(t: c_double) -> c_double {
    let e: c_double = obliquity_corr(t);
    let lambda: c_double = sun_apparent_lon(t);
    ((e).sin() * (lambda)).sin()
}

// Difference between true solar time and mean solar time.
// t: Julian centuries since J2000.0
// Return: Difference in minutes
unsafe extern "C" fn equation_of_time(t: c_double) -> c_double {
    let epsilon: c_double = obliquity_corr(t);
    let l_0: c_double = sun_geom_mean_lon(t);
    let e: c_double = earth_orbit_eccentricity(t);
    let m: c_double = sun_geom_mean_anomaly(t);
    let y: c_double = (epsilon / 2.0f64).tan().powf(2.0f64);
    let eq_time: c_double = y * (2 as c_int as c_double * l_0).sin()
        - 2 as c_int as c_double * e * (m).sin()
        + 4 as c_int as c_double * e * y * (m).sin() * (2 as c_int as c_double * l_0.cos())
        - 0.5f64 * y * y * (4 as c_int as c_double * l_0).sin()
        - 1.25f64 * e * e * (2 as c_int as c_double * m).sin();
    4 as c_int as c_double * (eq_time * (180 as c_int as c_double / 3.141_592_653_589_793_f64))
}

// Hour angle at the location for the given angular elevation.
// lat: Latitude of location in degrees
// decl: Declination in radians
// elev: Angular elevation angle in radians
// Return: Hour angle in radians
unsafe extern "C" fn hour_angle_from_elevation(
    lat: c_double,
    decl: c_double,
    elev: c_double,
) -> c_double {
    let omega: c_double = (elev.abs().cos()
        - (lat * (3.141_592_653_589_793_f64 / 180 as c_int as c_double)).sin() * (decl).sin())
        / ((lat * (3.141_592_653_589_793_f64 / 180 as c_int as c_double).cos()) * (decl.cos()))
            .cos();
    omega.copysign(-elev)
}

// Angular elevation at the location for the given hour angle.
// lat: Latitude of location in degrees
// decl: Declination in radians
// ha: Hour angle in radians
// Return: Angular elevation in radians
unsafe extern "C" fn elevation_from_hour_angle(
    lat: c_double,
    decl: c_double,
    ha: c_double,
) -> c_double {
    ((ha.cos())
        * (lat * (3.141_592_653_589_793_f64 / 180 as c_int as c_double).cos())
        * (decl.cos())
        + (lat * (3.141_592_653_589_793_f64 / 180 as c_int as c_double)).sin() * (decl).sin())
    .sin()
}

// Time of apparent solar noon of location on earth.
// t: Julian centuries since J2000.0
// lon: Longitude of location in degrees
// Return: Time difference from mean solar midnigth in minutes
unsafe extern "C" fn time_of_solar_noon(t: c_double, lon: c_double) -> c_double {
    // First pass uses approximate solar noon to
    // calculate equation of time.
    let mut t_noon: c_double = jcent_from_jd(jd_from_jcent(t) - lon / 360.0f64);
    let mut eq_time: c_double = equation_of_time(t_noon);
    let mut sol_noon: c_double = 720 as c_int as c_double - 4 as c_int as c_double * lon - eq_time;
    // Recalculate using new solar noon.
    t_noon = jcent_from_jd(jd_from_jcent(t) - 0.5f64 + sol_noon / 1440.0f64);
    eq_time = equation_of_time(t_noon);
    sol_noon = 720 as c_int as c_double - 4 as c_int as c_double * lon - eq_time;
    // No need to do more iterations
    sol_noon
}

// Time of given apparent solar angular elevation of location on earth.
// t: Julian centuries since J2000.0
// t_noon: Apparent solar noon in Julian centuries since J2000.0
// lat: Latitude of location in degrees
// lon: Longtitude of location in degrees
// elev: Solar angular elevation in radians
// Return: Time difference from mean solar midnight in minutes
unsafe extern "C" fn time_of_solar_elevation(
    t: c_double,
    t_noon: c_double,
    lat: c_double,
    lon: c_double,
    elev: c_double,
) -> c_double {
    // First pass uses approximate sunrise to
    // calculate equation of time.
    let mut eq_time: c_double = equation_of_time(t_noon);
    let mut sol_decl: c_double = solar_declination(t_noon);
    let mut ha: c_double = hour_angle_from_elevation(lat, sol_decl, elev);
    let mut sol_offset: c_double = 720 as c_int as c_double
        - 4 as c_int as c_double
            * (lon + ha * (180 as c_int as c_double / 3.141_592_653_589_793_f64))
        - eq_time;
    // Recalculate using new sunrise.
    let t_rise: c_double = jcent_from_jd(jd_from_jcent(t) + sol_offset / 1440.0f64);
    eq_time = equation_of_time(t_rise);
    sol_decl = solar_declination(t_rise);
    ha = hour_angle_from_elevation(lat, sol_decl, elev);
    sol_offset = 720 as c_int as c_double
        - 4 as c_int as c_double
            * (lon + ha * (180 as c_int as c_double / 3.141_592_653_589_793_f64))
        - eq_time;
    // No need to do more iterations
    sol_offset
}

// Solar angular elevation at the given location and time.
// t: Julian centuries since J2000.0
// lat: Latitude of location
// lon: Longitude of location
// Return: Solar angular elevation in radians
unsafe extern "C" fn solar_elevation_from_time(
    t: c_double,
    lat: c_double,
    lon: c_double,
) -> c_double {
    // Minutes from midnight
    let jd: c_double = jd_from_jcent(t);
    let offset: c_double = (jd - jd.round() - 0.5f64) * 1440.0f64;
    let eq_time: c_double = equation_of_time(t);
    let ha: c_double = ((720 as c_int as c_double - offset - eq_time) / 4 as c_int as c_double
        - lon)
        * (3.141_592_653_589_793_f64 / 180 as c_int as c_double);
    let decl: c_double = solar_declination(t);
    elevation_from_hour_angle(lat, decl, ha)
}

// Solar angular elevation at the given location and time.
// date: Seconds since unix epoch
// lat: Latitude of location
// lon: Longitude of location
// Return: Solar angular elevation in degrees
#[no_mangle]
pub unsafe extern "C" fn solar_elevation(date: c_double, lat: c_double, lon: c_double) -> c_double {
    let jd: c_double = jd_from_epoch(date);
    solar_elevation_from_time(jcent_from_jd(jd), lat, lon)
        * (180 as c_int as c_double / 3.141_592_653_589_793_f64)
}

#[no_mangle]
pub unsafe extern "C" fn solar_table_fill(
    date: c_double,
    lat: c_double,
    lon: c_double,
    table: *mut c_double,
) {
    // Calculate Julian day
    let jd: c_double = jd_from_epoch(date);
    // Calculate Julian day number
    let jdn: c_double = jd.round();
    let t: c_double = jcent_from_jd(jdn);
    // Calculate apparent solar noon
    let sol_noon: c_double = time_of_solar_noon(t, lon);
    let j_noon: c_double = jdn - 0.5f64 + sol_noon / 1440.0f64;
    let t_noon: c_double = jcent_from_jd(j_noon);
    *table.offset(SOLAR_TIME_NOON as c_int as isize) = epoch_from_jd(j_noon);
    // Calculate solar midnight
    *table.offset(SOLAR_TIME_MIDNIGHT as c_int as isize) = epoch_from_jd(j_noon + 0.5f64);
    /* Calculate absolute time of other phenomena */
    let mut i: c_int = 2 as c_int;
    while i < SOLAR_TIME_MAX as c_int {
        let angle: c_double = time_angle[i as usize];
        let offset: c_double = time_of_solar_elevation(t, t_noon, lat, lon, angle);
        *table.offset(i as isize) = epoch_from_jd(jdn - 0.5f64 + offset / 1440.0f64);
        i += 1;
        i;
    }
}
