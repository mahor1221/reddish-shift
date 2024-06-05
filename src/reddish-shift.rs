/*  redshift.rs -- Main program
    This file is part of <https://github.com/mahor1221/reddish-shift>.
    Copyright (C) 2024 Mahor Foruzesh <mahor1221@gmail.com>
    Ported from Redshift <https://github.com/jonls/redshift>.
    Copyright (c) 2009-2017  Jon Lund Steffensen <jonlst@gmail.com>

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

pub mod colorramp;
pub mod config;
pub mod gamma_drm;
pub mod gamma_dummy;
pub mod gamma_randr;
pub mod gamma_vidmode;
pub mod hooks;
// pub mod location_geoclue2;
pub mod location_manual;
pub mod pipeutils;
pub mod signals;
pub mod solar;

// use libc::{
//     __errno_location, exit, fprintf, fputs, pause, perror, poll, pollfd,
//     printf, setlocale, setvbuf, strcmp,
// };
// use signals::{disable, exiting, signals_install_handlers};
use anyhow::{anyhow, Result};
use config::{
    AdjustmentMethod, ColorSettings, ConfigBuilder, Elevation, ElevationRange,
    Location, LocationProvider, Mode, TimeOffset, TimeRanges, TransitionScheme,
};
// use hooks::hooks_signal_period_change;
use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    ops::Deref,
};
use time::OffsetDateTime as DateTime;

pub type Nfds = u64;
pub type SigAtomic = i32;

// extern "C" {
//     pub static stdout: *mut libc::FILE;
//     pub static stderr: *mut libc::FILE;
// }

// Duration of sleep between screen updates (milliseconds)
const SLEEP_DURATION: u32 = 5000;
const SLEEP_DURATION_SHORT: u32 = 100;
// Length of fade in numbers of short sleep durations
const FADE_LENGTH: u32 = 40;

fn main() -> Result<()> {
    // // Init locale
    // setlocale(0 as c_int, b"\0" as *const u8 as *const c_char);
    // setlocale(5 as c_int, b"\0" as *const u8 as *const c_char);

    // // Internationalisation
    // bindtextdomain(
    //     b"redshift\0" as *const u8 as *const c_char,
    //     b"/home/mahor/redshift/root/share/locale\0" as *const u8 as *const c_char,
    // );
    // textdomain(b"redshift\0" as *const u8 as *const c_char);

    // // Flush messages consistently even if redirected to a pipe or
    // // file.  Change the flush behaviour to line-buffered, without
    // // changing the actual buffers being used
    // setvbuf(stdout, std::ptr::null_mut::<c_char>(), 1 as c_int, 0);
    // setvbuf(stderr, std::ptr::null_mut::<c_char>(), 1 as c_int, 0);

    // use TransitionScheme::*;
    // cfg.need_location = match (&cfg.mode, &cfg.scheme) {
    //     (Mode::Daemon | Mode::Oneshot, Elevation(_)) => true,
    //     (Mode::Set | Mode::Reset, _) | (_, Time(_)) => false,
    // };

    // match (cfg.need_location, &cfg.location) {
    //     (true, LocationProvider::Manual(loc)) if loc.is_default() => {
    //         eprintln!("using default location");
    //     }
    //     _ => {}
    // }

    // if cfg.need_location {
    // if !(options.provider).is_null() {
    //     // Use provider specified on command line
    // } else {
    //     // Try all providers, use the first that works
    //     // b"Trying location provider `%s'...\n\0" as *const u8 as *const c_char,
    //     //     b"Trying next provider...\n\0" as *const u8 as *const c_char,
    //     //     b"Using provider `%s'.\n\0" as *const u8 as *const c_char,
    //     // b"No more location providers to try.\n\0" as *const u8 as *const c_char,
    // }

    // if options.verbosity {
    //     // TRANSLATORS: Append degree symbols if possible
    //     // b"Solar elevations: day above %.1f, night below %.1f\n\0" as *const u8
    // }
    // }

    // if let Mode::Daemon | Mode::Oneshot = cfg.mode {
    // if options.verbosity {
    //     // b"Temperatures: %dK at day, %dK at night\n\0" as *const u8 as *const c_char,
    // }
    // }

    // if options.verbosity {
    //     // b"Brightness: %.2f:%.2f\n\0" as *const u8 as *const c_char,
    //     // options.scheme.day.brightness as c_double,
    //     // options.scheme.night.brightness as c_double,
    // }

    // if options.verbosity {
    //     // printf(
    //     //     // gettext(
    //     //     b"Gamma (%s): %.3f, %.3f, %.3f\n\0" as *const u8 as *const c_char,
    //     //     // gettext(
    //     //     b"Daytime\0" as *const u8 as *const c_char,
    //     //     options.scheme.day.gamma[0 as c_int as usize] as c_double,
    //     //     options.scheme.day.gamma[1 as c_int as usize] as c_double,
    //     //     options.scheme.day.gamma[2 as c_int as usize] as c_double,
    //     // );
    //     // printf(
    //     //     // gettext(
    //     //     b"Gamma (%s): %.3f, %.3f, %.3f\n\0" as *const u8 as *const c_char,
    //     //     // gettext(
    //     //     b"Night\0" as *const u8 as *const c_char,
    //     //     options.scheme.night.gamma[0 as c_int as usize] as c_double,
    //     //     options.scheme.night.gamma[1 as c_int as usize] as c_double,
    //     //     options.scheme.night.gamma[2 as c_int as usize] as c_double,
    //     // );
    // }

    let cfg = ConfigBuilder::new()?.build()?;

    // TODO: add a command for calculating solar elevation for the next 24h
    match cfg.mode {
        Mode::Daemon => {
            // r = run_continual_mode(
            //     options.provider,
            //     location_state,
            //     scheme,
            //     options.method,
            //     method_state,
            //     options.use_fade,
            //     options.preserve_gamma,
            //     options.verbose,
            // );
            // if r < 0 as c_int {
            //     exit(1 as c_int);
            // }
        }

        Mode::Oneshot => {
            // TODO: add time-zone to config, or convert location to timezone
            // b"Unable to read system time.\n\0" as *const u8 as *const c_char,
            let now = DateTime::now_local()?;

            let period = match cfg.scheme {
                TransitionScheme::Time(time_ranges) => {
                    Period::from_time(now.time().into(), time_ranges)
                }
                TransitionScheme::Elevation(elev_range) => {
                    let now = (now - DateTime::UNIX_EPOCH).as_seconds_f64();
                    let (loc, available) = cfg.location.get()?;
                    let elev = Elevation::new(now, loc);
                    Period::from_elevation(elev, elev_range)
                    // if config.verbose {
                    //     // TRANSLATORS: Append degree symbol if possible
                    //     // b"Solar elevation: %f\n\0" as *const u8 as *const c_char,
                    //     todo!()
                    // }
                }
            };

            // Use transition progress to set color temperature
            let interp = cfg.night.interpolate_with(&cfg.day, period.into());

            // if options.verbosity {
            //     // print_period(period, transition_prog);
            //     // b"Color settings: %uK\n\0"
            // }
            cfg.method.set_color(&interp, cfg.reset_ramps)?;
        }

        Mode::Set => {
            // for the set command, color settings are stored in the day field
            cfg.method.set_color(&cfg.day, cfg.reset_ramps)?;
            // if cfg.verbosity {
            //     // b"Color settings: %uK\n\0"
            // }
        }

        Mode::Reset => {
            let cs = ColorSettings::default();
            cfg.method.set_color(&cs, true)?;
        }
    }

    Ok(())
}

/// Periods of day
#[derive(Debug, Clone, Copy, PartialEq)]
enum Period {
    Daytime,
    Night,
    Transition {
        progress: f64, // Between 0 and 1
    },
}

#[derive(Debug, Clone, Copy)]
pub struct Alpha(f64);

// Read NOTE in src/config.rs
impl Deref for Alpha {
    type Target = f64;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TryFrom<f64> for Alpha {
    type Error = anyhow::Error;

    fn try_from(n: f64) -> Result<Self, Self::Error> {
        if n >= 0.0 && n <= 1.0 {
            Ok(Self(n))
        } else {
            Err(anyhow!("alpha"))
        }
    }
}

impl Alpha {
    // TODO: why?
    pub fn clamp(n: f64) -> Self {
        if n < 0.0 {
            Self(0.0)
        } else if n < 1.0 {
            Self(n)
        } else {
            Self(1.0)
        }
    }
}

impl From<Period> for Alpha {
    fn from(period: Period) -> Self {
        match period {
            Period::Daytime => Self(1.0),
            Period::Night => Self(0.0),
            Period::Transition { progress } => Self(progress),
        }
    }
}

impl Period {
    /// Determine which period we are currently in based on time offset
    fn from_time(time: TimeOffset, time_ranges: TimeRanges) -> Self {
        let TimeRanges { dawn, dusk } = time_ranges;
        let sub = |a: TimeOffset, b: TimeOffset| (*a - *b) as f64;

        if time < dawn.start || time >= dusk.end {
            Self::Night
        } else if time < dawn.end {
            let progress = sub(dawn.start, time) / sub(dawn.start, dawn.end);
            Self::Transition { progress }
        } else if time > dusk.start {
            let progress = sub(dusk.end, time) / sub(dusk.end, dusk.start);
            Self::Transition { progress }
        } else {
            Self::Daytime
        }
    }

    /// Determine which period we are currently in based on solar elevation
    fn from_elevation(elev: Elevation, elev_range: ElevationRange) -> Self {
        let ElevationRange { high, low } = elev_range;
        let sub = |a: Elevation, b: Elevation| (*a - *b);

        if elev < low {
            Self::Night
        } else if elev < high {
            let progress = sub(low, elev) / sub(low, high);
            Self::Transition { progress }
        } else {
            Self::Daytime
        }
    }
}

impl Display for Period {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Period::Daytime => f.write_str("Period: Daytime"),
            Period::Night => f.write_str("Period: Night"),
            Period::Transition { progress } => {
                write!(f, "Period: Transition ({progress:.2}% day)")
            }
        }
    }
}

impl Display for Location {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let a = *self.lat;
        let b = *self.lon;
        let ns = if a >= 0.0 { "N" } else { "S" };
        let ew = if b >= 0.0 { "E" } else { "W" };
        let a = a.abs();
        let aa = a.fract() * 100.0;
        let b = b.abs();
        let bb = b.fract() * 100.0;
        write!(f, "Location: {a:.0}°{aa:.0}′{ns} {b:.0}°{bb:.0}′{ew}")
    }
}

/// Easing function for fade
/// See https://github.com/mietek/ease-tween
fn ease_fade(t: f64) -> f64 {
    if t <= 0.0 {
        0.0
    } else if t >= 1.0 {
        1.0
    } else {
        1.0042954579734844
            * (-6.404173895841566 * (-7.290824133098134 * t).exp()).exp()
    }
}

pub trait IsDefault {
    fn is_default(&self) -> bool;
}

impl<T: Default + PartialEq> IsDefault for T {
    fn is_default(&self) -> bool {
        *self == T::default()
    }
}

pub trait Provider {
    // Listen and handle location updates
    // fn fd() -> c_int;

    fn get(&self) -> Result<(Location, bool)> {
        Err(anyhow!("Unable to get location from provider"))
    }
}

pub trait Adjuster {
    /// Restore the adjustment to the state before the Adjuster object was created
    fn restore(&self) -> Result<()> {
        Err(anyhow!("Temperature adjustment failed"))
    }

    /// Set a specific color temperature
    #[allow(unused_variables)]
    fn set_color(&self, cs: &ColorSettings, reset_ramps: bool) -> Result<()> {
        Err(anyhow!("Temperature adjustment failed"))
    }
}

impl Provider for LocationProvider {
    fn get(&self) -> Result<(Location, bool)> {
        // if cfg.need_location {
        // b"Waiting for current location to become available...\n\0" as *const u8

        // Wait for location provider
        // b"Unable to get location from provider.\n\0" as *const u8 as *const c_char,
        // print_location(&mut loc);
        // }

        match self {
            Self::Manual(t) => t.get(),
            // Self::Geoclue2(t) => t.get(),
        }
    }
}

impl Adjuster for AdjustmentMethod {
    fn restore(&self) -> Result<()> {
        match self {
            Self::Dummy(t) => t.restore(),
            Self::Randr(t) => t.restore(),
            Self::Drm(t) => t.restore(),
            Self::Vidmode(t) => t.restore(),
        }
    }

    fn set_color(&self, cs: &ColorSettings, reset_ramps: bool) -> Result<()> {
        match self {
            Self::Dummy(t) => t.set_color(cs, reset_ramps),
            Self::Randr(t) => t.set_color(cs, reset_ramps),
            Self::Drm(t) => t.set_color(cs, reset_ramps),
            Self::Vidmode(t) => t.set_color(cs, reset_ramps),
        }

        // // In Quartz (macOS) the gamma adjustments will
        // // automatically revert when the process exits
        // // Therefore, we have to loop until CTRL-C is received
        // if strcmp(options.method.name, "quartz") == 0 {
        //     // b"Press ctrl-c to stop...\n" as *const u8 as *const c_char,
        //     pause();
        // }
    }
}

// Run continual mode loop
// This is the main loop of the continual mode which keeps track of the
// current time and continuously updates the screen to the appropriate
// color temperature
// fn run_continual_mode(config: &Config) -> c_int {
//     let mut r: c_int = 0;

//     // Short fade parameters
//     let mut fade_length = 0;
//     let mut fade_time = 0;
//     let mut fade_start_interp: ColorSetting = ColorSetting {
//         temperature: 0,
//         gamma: [0.; 3],
//         brightness: 0.,
//     };

//     r = signals_install_handlers();
//     if r < 0 as c_int {
//         return r;
//     }

//     // Save previous parameters so we can avoid printing status updates if
//     // the values did not change
//     let mut prev_period: Period;

//     // Previous target color setting and current actual color setting
//     // Actual color setting takes into account the current color fade
//     let mut prev_target_interp = ColorSetting::default();
//     let mut interp = ColorSetting::default();
//     let mut loc: location_t = location_t {
//         lat: ::core::f32::NAN,
//         lon: ::core::f32::NAN,
//     };

//     let need_location = config.transition_scheme.select == TransitionSchemeKind::Elevation;

//     if need_location {
//         // println!("Waiting for initial location to become available...");

//         // Get initial location from provider
//         // r = provider_get_location(provider, location_state, -(1 as c_int), &mut loc);
//         // eprintln!("Unable to get location from provider.");
//         // print_location(&mut loc);
//         todo!()
//     }

//     if config.verbose {
//         // b"Color temperature: %uK\n\0" as *const u8 as *const c_char,
//         // interp.temperature,
//         // b"Brightness: %.2f\n\0" as *const u8 as *const c_char,
//         // interp.brightness as c_double,
//         todo!()
//     }

//     // Continuously adjust color temperature
//     let mut done: c_int = 0 as c_int;
//     let mut prev_disabled: c_int = 1 as c_int;
//     let mut disabled: c_int = 0 as c_int;
//     let mut location_available: c_int = 1 as c_int;
//     loop {
//         // Check to see if disable signal was caught
//         if disable != 0 && done == 0 {
//             disabled = (disabled == 0) as c_int;
//             ::core::ptr::write_volatile(addr_of_mut!(disable) as *mut SigAtomic, 0 as c_int);
//         }

//         // Check to see if exit signal was caught
//         if exiting != 0 {
//             if done != 0 {
//                 // On second signal stop the ongoing fade
//                 break;
//             }
//             done = 1 as c_int;
//             disabled = 1 as c_int;
//             ::core::ptr::write_volatile(addr_of_mut!(exiting) as *mut SigAtomic, 0 as c_int);
//         }

//         // Print status change
//         if config.verbose && disabled != prev_disabled {
//             // printf(
//             //     // gettext(
//             //     b"Status: %s\n\0" as *const u8 as *const c_char,
//             //     if disabled != 0 {
//             //         // gettext(
//             //         b"Disabled\0" as *const u8 as *const c_char
//             //     } else {
//             //         // gettext(
//             //         b"Enabled\0" as *const u8 as *const c_char
//             //     },
//             // );
//             todo!()
//         }
//         prev_disabled = disabled;

//         // Read timestamp
//         let mut now: c_double = 0.;
//         r = systemtime_get_time(&mut now);
//         if r < 0 as c_int {
//             eprintln!("Unable to read system time.");
//             return -(1 as c_int);
//         }

//         let (period, transition_prog): (Period, TransitionProgress) =
//             match config.transition_scheme.select {
//                 TransitionSchemeKind::TimeRanges => {
//                     // let time_offset: c_int = get_seconds_since_midnight(now);
//                     // period = get_period_from_time(scheme, time_offset);
//                     // transition_prog = get_transition_progress_from_time(scheme, time_offset);
//                     todo!()
//                 }
//                 TransitionSchemeKind::Elevation => {
//                     // let elevation: c_double =
//                     //     solar_elevation(now, loc.lat as c_double, loc.lon as c_double);
//                     // period = get_period_from_elevation(scheme, elevation);
//                     // transition_prog = get_transition_progress_from_elevation(scheme, elevation);
//                     todo!()
//                 }
//             };

//         // Use transition progress to get target color
//         //    temperature
//         let target_interp = interpolate_color_settings(
//             &config.night_color_setting,
//             &config.day_color_setting,
//             transition_prog,
//         );

//         if disabled != 0 {
//             period = PERIOD_NONE;
//             color_setting_reset(&mut target_interp);
//         }
//         if done != 0 {
//             period = PERIOD_NONE;
//         }

//         // Print period if it changed during this update,
//         // or if we are in the transition period. In transition we
//         // print the progress, so we always print it in
//         // that case
//         if config.verbose && (period != prev_period || period == Period::Transition) {
//             print_period(period, transition_prog);
//         }

//         /* Activate hooks if period changed */
//         if period != prev_period {
//             hooks_signal_period_change(prev_period, period);
//         }

//         // Start fade if the parameter differences are too big to apply
//         // instantly
//         if config.fade
//             && (fade_length == 0 && is_color_setting_diff_major(&mut interp, &mut target_interp)
//                 || fade_length != 0 as c_int
//                     && is_color_setting_diff_major(&mut target_interp, &mut prev_target_interp))
//         {
//             fade_length = 40 as c_int;
//             fade_time = 0 as c_int;
//             fade_start_interp = interp;
//         }

//         // Handle ongoing fade
//         if fade_length != 0 as c_int {
//             fade_time += 1;
//             let frac = fade_time as f32 / fade_length as f32;
//             let alpha = if 0.0 > ease_fade(frac) {
//                 Alpha(0.0)
//             } else if ease_fade(frac) < 1.0 {
//                 Alpha(ease_fade(frac))
//             } else {
//                 Alpha(1.0)
//             };
//             let interp =
//                 interpolate_color_settings(&mut fade_start_interp, &mut target_interp, alpha);
//             if fade_time > fade_length {
//                 fade_time = 0;
//                 fade_length = 0;
//             }
//         } else {
//             interp = target_interp;
//         }

//         // Break loop when done and final fade is over
//         if done != 0 && fade_length == 0 as c_int {
//             break;
//         }

//         if config.verbose {
//             if prev_target_interp.temperature != target_interp.temperature {
//                 // b"Color temperature: %uK\n\0" as *const u8 as *const c_char,
//                 // target_interp.temperature,
//                 todo!()
//             }
//             if prev_target_interp.brightness != target_interp.brightness {
//                 // b"Brightness: %.2f\n\0" as *const u8 as *const c_char,
//                 // target_interp.brightness as c_double,
//                 todo!()
//             }
//         }

//         // Adjust temperature
//         r = ((*method).set_temperature).expect("non-null function pointer")(
//             method_state,
//             &mut interp,
//             preserve_gamma,
//         );
//         // eprintln!("Temperature adjustment failed.");

//         // Save period and target color setting as previous
//         prev_period = period;
//         prev_target_interp = target_interp;

//         // Sleep length depends on whether a fade is ongoing
//         let mut delay = 5000;
//         if fade_length != 0 {
//             delay = 100;
//         }

//         // Update location
//         let mut loc_fd = -1;
//         if need_location {
//             loc_fd = ((*provider).get_fd).expect("non-null function pointer")(location_state);
//         }

//         if loc_fd >= 0 as c_int {
//             // Provider is dynamic
//             let mut pollfds: [pollfd; 1] = [pollfd {
//                 fd: 0,
//                 events: 0,
//                 revents: 0,
//             }; 1];

//             pollfds[0 as c_int as usize].fd = loc_fd;
//             pollfds[0 as c_int as usize].events = 0x1 as c_int as c_short;
//             let mut r_0: c_int = poll(pollfds.as_mut_ptr(), 1 as c_int as Nfds, delay);
//             if r_0 < 0 as c_int {
//                 if *__errno_location() == 4 as c_int {
//                     continue;
//                 }
//                 perror(b"poll\0" as *const u8 as *const c_char);
//                 eprintln!("Unable to get location from provider.");
//                 return -(1 as c_int);
//             } else {
//                 if r_0 == 0 as c_int {
//                     continue;
//                 }

//                 // Get new location and availability
//                 // information
//                 let mut new_loc: location_t = location_t { lat: 0., lon: 0. };
//                 let mut new_available: c_int = 0;
//                 r_0 = ((*provider).handle).expect("non-null function pointer")(
//                     location_state,
//                     &mut new_loc,
//                     &mut new_available,
//                 );
//                 if r_0 < 0 as c_int {
//                     eprintln!("Unable to get location from provider.");
//                     return -(1 as c_int);
//                 }
//                 if new_available == 0 && new_available != location_available {
//                     eprintln!("Location is temporarily unavailable; Using previous location until it becomes available...");
//                 }

//                 if new_available != 0
//                     && (new_loc.lat != loc.lat
//                         || new_loc.lon != loc.lon
//                         || new_available != location_available)
//                 {
//                     loc = new_loc;
//                     print_location(&mut loc);
//                 }
//                 location_available = new_available;
//             }
//         } else {
//             systemtime_msleep(delay as c_uint);
//         }
//     }

//     // Restore saved gamma ramps
//     ((*method).restore).expect("non-null function pointer")(method_state);
//     0 as c_int
// }