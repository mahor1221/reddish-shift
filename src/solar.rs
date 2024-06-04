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

/*  From Redshift:
    > Ported from javascript code by U.S. Department of Commerce,
    > National Oceanic & Atmospheric Administration:
    > http://www.srrb.noaa.gov/highlights/sunrise/calcdetails.html
    > It is based on equations from "Astronomical Algorithms" by
    > Jean Meeus.
*/

#![allow(dead_code)]
use std::f64::consts::PI;

macro_rules! rad {
    ($x:expr) => {
        $x * PI / 180.0
    };
}

// Model of atmospheric refraction near horizon (in degrees)
const SOLAR_ATM_REFRAC: f64 = 0.833;

const SOLAR_ASTRO_TWILIGHT_ELEV: f64 = -18.0;
const SOLAR_NAUT_TWILIGHT_ELEV: f64 = -12.0;
pub const SOLAR_CIVIL_TWILIGHT_ELEV: f64 = -6.0;
const SOLAR_DAYTIME_ELEV: f64 = 0.0 - SOLAR_ATM_REFRAC;

// Angels of various times of day
const SOLAR_TIME_MAX: usize = 10;
const TIME_ANGLE: [f64; SOLAR_TIME_MAX] = [
    rad!(0.0),                               // Noon
    rad!(0.0),                               // Midnight
    rad!(-90.0 + SOLAR_ASTRO_TWILIGHT_ELEV), // AstroDawn
    rad!(-90.0 + SOLAR_NAUT_TWILIGHT_ELEV),  // NautDawn
    rad!(-90.0 + SOLAR_CIVIL_TWILIGHT_ELEV), // CivilDawn
    rad!(-90.0 + SOLAR_DAYTIME_ELEV),        // Sunrise
    rad!(90.0 - SOLAR_DAYTIME_ELEV),         // Sunset
    rad!(90.0 - SOLAR_CIVIL_TWILIGHT_ELEV),  // CivilDusk
    rad!(90.0 - SOLAR_NAUT_TWILIGHT_ELEV),   // NautDusk
    rad!(90.0 - SOLAR_ASTRO_TWILIGHT_ELEV),  // AstroDusk
];

/// Unix epoch from Julian day
fn epoch_from_jd(jd: f64) -> f64 {
    86400.0 * (jd - 2440587.5)
}

/// Julian day from unix epoch
fn jd_from_epoch(t: f64) -> f64 {
    t / 86400.0 + 2440587.5
}

/// Julian centuries since J2000.0 from Julian day
fn jcent_from_jd(jd: f64) -> f64 {
    (jd - 2451545.0f64) / 36525.0f64
}

/// Julian day from Julian centuries since J2000.0
fn jd_from_jcent(t: f64) -> f64 {
    36525.0 * t + 2451545.0
}

/// Geometric mean longitude of the sun
/// t: Julian centuries since J2000.0
///   Return: Geometric mean longitude in radians
fn sun_geom_mean_lon(t: f64) -> f64 {
    // FIXME returned value should always be positive
    ((280.46646 + t * (36000.76983 + t * 0.0003032)) % 360.0).to_radians()
}

/// Geometric mean anomaly of the sun
/// t: Julian centuries since J2000.0
/// Return: Geometric mean anomaly in radians
fn sun_geom_mean_anomaly(t: f64) -> f64 {
    (357.52911 + t * (35999.05029 - t * 0.0001537)).to_radians()
}

/// Eccentricity of earth orbit
/// t: Julian centuries since J2000.0
/// Return: Eccentricity (unitless)
fn earth_orbit_eccentricity(t: f64) -> f64 {
    0.016708634 - t * (0.000042037 + t * 0.0000001267)
}

/// Equation of center of the sun
/// t: Julian centuries since J2000.0
/// Return: Center(?) in radians
fn sun_equation_of_center(t: f64) -> f64 {
    let m = sun_geom_mean_anomaly(t);
    let c = (m).sin() * (1.914602 - t * (0.004817 + 0.000014 * t))
        + (2.0 * m).sin() * (0.019993 - 0.000101 * t)
        + (3.0 * m).sin() * 0.000289;
    c.to_radians()
}

/// True longitude of the sun
/// t: Julian centuries since J2000.0
/// Return: True longitude in radians
fn sun_true_lon(t: f64) -> f64 {
    let l_0 = sun_geom_mean_lon(t);
    let c = sun_equation_of_center(t);
    l_0 + c
}

/// Apparent longitude of the sun. (Right ascension)
/// t: Julian centuries since J2000.0
/// Return: Apparent longitude in radians
fn sun_apparent_lon(t: f64) -> f64 {
    let o = sun_true_lon(t);
    (o.to_degrees()
        - 0.00569
        - 0.00478 * (125.04 - 1934.136 * t).to_radians().sin())
    .to_radians()
}

/// Mean obliquity of the ecliptic
/// t: Julian centuries since J2000.0
/// Return: Mean obliquity in radians
fn mean_ecliptic_obliquity(t: f64) -> f64 {
    let sec = 21.448 - t * (46.815 + t * (0.00059 - t * 0.001813));
    (23.0 + (26.0 + (sec / 60.0)) / 60.0).to_radians()
}

/// Corrected obliquity of the ecliptic
/// t: Julian centuries since J2000.0
/// Return: Corrected obliquity in radians
fn obliquity_corr(t: f64) -> f64 {
    let e_0 = mean_ecliptic_obliquity(t);
    let omega = 125.04 - t * 1934.136;
    (e_0.to_degrees() + 0.00256 * omega.to_radians().cos()).to_radians()
}

/// Declination of the sun
/// t: Julian centuries since J2000.0
/// Return: Declination in radians
fn solar_declination(t: f64) -> f64 {
    let e = obliquity_corr(t);
    let lambda = sun_apparent_lon(t);
    ((e).sin() * (lambda)).asin()
}

/// Difference between true solar time and mean solar time
/// t: Julian centuries since J2000.0
/// Return: Difference in minutes
fn equation_of_time(t: f64) -> f64 {
    let epsilon = obliquity_corr(t);
    let l_0 = sun_geom_mean_lon(t);
    let e = earth_orbit_eccentricity(t);
    let m = sun_geom_mean_anomaly(t);
    let y = (epsilon / 2.0).tan().powf(2.0);

    let eq_time = y * (2.0 * l_0).sin() - 2.0 * e * m.sin()
        + 4.0 * e * y * m.sin() * (2.0 * l_0).cos()
        - 0.5 * y * y * (4.0 * l_0).sin()
        - 1.25 * e * e * (2.0 * m).sin();
    4.0 * eq_time.to_degrees()
}

/// Hour angle at the location for the given angular elevation
/// lat: Latitude of location in degrees
/// decl: Declination in radians
/// elev: Angular elevation angle in radians
/// Return: Hour angle in radians
fn hour_angle_from_elevation(lat: f64, decl: f64, elev: f64) -> f64 {
    let omega = elev.abs().cos() - lat.to_radians().sin() * decl.sin();
    let omega = omega / lat.to_radians().cos() * decl.cos();
    omega.acos().copysign(-elev)
}

/// Angular elevation at the location for the given hour angle
/// lat: Latitude of location in degrees
/// decl: Declination in radians
/// ha: Hour angle in radians
/// Return: Angular elevation in radians
fn elevation_from_hour_angle(lat: f64, decl: f64, ha: f64) -> f64 {
    (ha.cos() * lat.to_radians().cos() * decl.cos()
        + lat.to_radians().sin() * decl.sin())
    .asin()
}

/// Time of apparent solar noon of location on earth
/// t: Julian centuries since J2000.0
/// lon: Longitude of location in degrees
/// Return: Time difference from mean solar midnigth in minutes
fn time_of_solar_noon(t: f64, lon: f64) -> f64 {
    // First pass uses approximate solar noon to
    // calculate equation of time
    let mut t_noon = jcent_from_jd(jd_from_jcent(t) - lon / 360.0);
    let mut eq_time = equation_of_time(t_noon);
    let mut sol_noon = 720.0 - 4.0 * lon - eq_time;

    // Recalculate using new solar noon
    t_noon = jcent_from_jd(jd_from_jcent(t) - 0.5 + sol_noon / 1440.0);
    eq_time = equation_of_time(t_noon);
    sol_noon = 720.0 - 4.0 * lon - eq_time;
    // No need to do more iterations
    sol_noon
}

/// Time of given apparent solar angular elevation of location on earth
/// t: Julian centuries since J2000.0
/// t_noon: Apparent solar noon in Julian centuries since J2000.0
/// lat: Latitude of location in degrees
/// lon: Longtitude of location in degrees
/// elev: Solar angular elevation in radians
/// Return: Time difference from mean solar midnight in minutes
fn time_of_solar_elevation(
    t: f64,
    t_noon: f64,
    lat: f64,
    lon: f64,
    elev: f64,
) -> f64 {
    // First pass uses approximate sunrise to
    // calculate equation of time
    let mut eq_time = equation_of_time(t_noon);
    let mut sol_decl = solar_declination(t_noon);
    let mut ha = hour_angle_from_elevation(lat, sol_decl, elev);
    let mut sol_offset = 720.0 - 4.0 * (lon + ha.to_degrees()) - eq_time;

    // Recalculate using new sunrise
    let t_rise = jcent_from_jd(jd_from_jcent(t) + sol_offset / 1440.0);
    eq_time = equation_of_time(t_rise);
    sol_decl = solar_declination(t_rise);
    ha = hour_angle_from_elevation(lat, sol_decl, elev);
    sol_offset = 720.0 - 4.0 * (lon + ha.to_degrees()) - eq_time;
    // No need to do more iterations
    sol_offset
}

/// Solar angular elevation at the given location and time
/// t: Julian centuries since J2000.0
/// lat: Latitude of location
/// lon: Longitude of location
/// Return: Solar angular elevation in radians
fn solar_elevation_from_time(t: f64, lat: f64, lon: f64) -> f64 {
    // Minutes from midnight
    let jd = jd_from_jcent(t);
    let offset = (jd - jd.round() - 0.5) * 1440.0;
    let eq_time = equation_of_time(t);
    let ha = ((720.0 - offset - eq_time) / 4.0 - lon).to_radians();
    let decl = solar_declination(t);
    elevation_from_hour_angle(lat, decl, ha)
}

/// Solar angular elevation at the given location and time
/// date: Seconds since unix epoch
/// lat: Latitude of location
/// lon: Longitude of location
/// Return: Solar angular elevation in degrees
pub fn solar_elevation(date: f64, lat: f64, lon: f64) -> f64 {
    let jd = jd_from_epoch(date);
    let jcent = jcent_from_jd(jd);
    solar_elevation_from_time(jcent, lat, lon).to_degrees()
}

fn solar_table_fill(
    date: f64,
    lat: f64,
    lon: f64,
    table: &mut [f64; SOLAR_TIME_MAX],
) {
    // Calculate Julian day
    let jd = jd_from_epoch(date);

    // Calculate Julian day number
    let jdn = jd.round();
    let t = jcent_from_jd(jdn);

    // Calculate apparent solar noon
    let sol_noon = time_of_solar_noon(t, lon);
    let j_noon = jdn - 0.5 + sol_noon / 1440.0;
    let t_noon = jcent_from_jd(j_noon);
    table[0] = epoch_from_jd(j_noon);

    // Calculate solar midnight
    table[1] = epoch_from_jd(j_noon + 0.5);

    // Calculate absolute time of other phenomena
    for i in 2..SOLAR_TIME_MAX {
        let angle = TIME_ANGLE[i as usize];
        let offset = time_of_solar_elevation(t, t_noon, lat, lon, angle);
        table[i] = epoch_from_jd(jdn - 0.5 + offset / 1440.0);
    }
}

#[cfg(test)]
mod test {
    use super::solar_elevation;
    use anyhow::Result;
    use insta::assert_snapshot;
    use std::{fmt::Write, time::Duration};

    #[test]
    fn test_solar_elevation() -> Result<()> {
        let res = (0..24).try_fold(String::new(), |mut buff, i| {
            let s = Duration::from_secs(i * 3600).as_secs_f64();
            let e = solar_elevation(s, 0.0, 0.0);
            write!(&mut buff, "{i:02}:00, {e:6.2}°\n")?;
            Ok::<_, anyhow::Error>(buff)
        })?;

        Ok(assert_snapshot!(res, @r###"
        00:00, -56.32°
        01:00, -53.81°
        02:00, -46.63°
        03:00, -36.68°
        04:00, -25.28°
        05:00, -13.15°
        06:00,  -0.71°
        07:00,  11.76°
        08:00,  23.96°
        09:00,  35.51°
        10:00,  45.73°
        11:00,  53.37°
        12:00,  56.56°
        13:00,  54.05°
        14:00,  46.84°
        15:00,  36.84°
        16:00,  25.40°
        17:00,  13.23°
        18:00,   0.76°
        19:00, -11.74°
        20:00, -23.98°
        21:00, -35.58°
        22:00, -45.86°
        23:00, -53.57°
        "###))
    }
}
