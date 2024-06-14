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

// TODO: add setting screen brightness, a percentage of the current brightness

pub mod colorramp;
pub mod config;
pub mod display;
pub mod gamma_drm;
pub mod gamma_dummy;
pub mod gamma_randr;
pub mod gamma_vidmode;
pub mod location_manual;
pub mod solar;
pub mod utils;

use anyhow::{anyhow, Result};
use chrono::{DateTime, Local};
use config::{
    AdjustmentMethod, ColorSettings, Config, ConfigBuilder, Elevation,
    ElevationRange, Location, LocationProvider, Mode, TimeOffset, TimeRanges,
    TransitionScheme, Verbosity, FADE_STEPS,
};
use std::{
    fmt::Debug,
    io::Write,
    ops::Deref,
    sync::mpsc::{self, Receiver, RecvTimeoutError},
};
use utils::IsDefault;

fn main() -> Result<()> {
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

    let stdout = std::io::stdout();
    let (c, mut v) = ConfigBuilder::new()?.build(stdout.lock())?;

    if let (
        Mode::Daemon | Mode::Oneshot,
        TransitionScheme::Elevation(_),
        true,
    ) = (&c.mode, &c.scheme, c.location.get(&mut v)?.is_default())
    {
        writeln!(v, "Warning: using default location")?;
    }

    let (tx, rx) = mpsc::channel();
    ctrlc::set_handler(move || {
        tx.send(()).expect("Could not send signal on channel")
    })
    .expect("Error setting Ctrl-C handler");

    run(c, rx, &mut v)
}

fn run(
    c: Config,
    sig: Receiver<()>,
    v: &mut Verbosity<impl Write>,
) -> Result<()> {
    // TODO: add a command for calculating solar elevation for the next 24h
    match c.mode {
        Mode::Daemon => {
            DaemonMode::new(&c, &sig).run_loop(v)?;
            c.method.restore(c.dry_run)?;
        }

        Mode::Oneshot => {
            // Use period and transition progress to set color temperature
            let (p, i) = Period::from(&c.scheme, &c.location, c.time, v)?;
            let interp = c.night.interpolate_with(&c.day, p.into());
            writeln_verbose!(v, "{p}\n{i}{interp}")?;
            c.method.set(c.dry_run, c.reset_ramps, &interp)?;
        }

        Mode::Set => {
            // for the set command, color settings are stored in the day field
            c.method.set(c.dry_run, c.reset_ramps, &c.day)?;
            // if cfg.verbosity {
            //     // b"Color settings: %uK\n\0"
            // }
        }

        Mode::Reset => {
            let cs = ColorSettings::default();
            c.method.set(c.dry_run, true, &cs)?;
        }
    }

    Ok(())
}

#[derive(Debug)]
struct DaemonMode<'a, 'b> {
    cfg: &'a Config,
    sig: &'b Receiver<()>,

    signal: Signal,
    fade: FadeStatus,

    period: Period,
    info: PeriodInfo,
    interp: ColorSettings,

    // Save previous parameters so we can avoid printing status updates if the
    // values did not change
    period_prev: Option<Period>,
    info_prev: Option<PeriodInfo>,
    interp_prev: Option<ColorSettings>,
}

impl<'a, 'b> DaemonMode<'a, 'b> {
    fn new(cfg: &'a Config, sig: &'b Receiver<()>) -> Self {
        Self {
            cfg,
            sig,
            signal: Default::default(),
            fade: Default::default(),
            period: Default::default(),
            info: Default::default(),
            interp: Default::default(),
            period_prev: Default::default(),
            info_prev: Default::default(),
            interp_prev: Default::default(),
        }
    }

    /// This is the main loop of the daemon mode which keeps track of the
    /// current time and continuously updates the screen to the appropriate
    /// color temperature
    fn run_loop(&mut self, v: &mut Verbosity<impl Write>) -> Result<()> {
        let c = self.cfg;

        loop {
            (self.period, self.info) =
                Period::from(&c.scheme, &c.location, c.time, v)?;

            let target = match self.signal {
                Signal::None => {
                    c.night.interpolate_with(&c.day, self.period.into())
                }
                Signal::Interrupt => ColorSettings::default(),
            };

            (self.interp, self.fade) = self.next_interpolate(target);

            self.write_verbose(v)?;

            // // Activate hooks if period changed
            // if period != prev_period {
            //     hooks_signal_period_change(prev_period, period);
            // }
            c.method.set(c.dry_run, c.reset_ramps, &self.interp)?;

            self.period_prev = Some(self.period);
            self.info_prev = Some(self.info.clone());
            self.interp_prev = Some(self.interp.clone());

            // sleep for a duration then continue the loop
            // or wake up and restore the default colors slowly on first ctrl-c
            // or break the loop on the second ctrl-c immediately
            let sleep_duration = match (self.signal, self.fade) {
                (Signal::None, FadeStatus::Completed) => c.sleep_duration,
                (_, FadeStatus::Ungoing { .. }) => c.fade_sleep_duration,
                (Signal::Interrupt, FadeStatus::Completed) => break Ok(()),
            };

            match self.sig.recv_timeout(sleep_duration) {
                Err(RecvTimeoutError::Timeout) => {}
                Err(e) => Err(e)?,
                Ok(()) => match self.signal {
                    Signal::None => self.signal = Signal::Interrupt,
                    Signal::Interrupt => break Ok(()),
                },
            }
        }
    }

    fn next_interpolate(
        &self,
        target: ColorSettings,
    ) -> (ColorSettings, FadeStatus) {
        let target_is_very_different = self.interp.is_very_diff_from(&target);
        match (&self.fade, target_is_very_different, self.cfg.disable_fade) {
            (_, _, true)
            | (FadeStatus::Completed, false, false)
            | (FadeStatus::Ungoing { .. }, false, false) => {
                (target, FadeStatus::Completed)
            }

            (FadeStatus::Completed, true, false) => {
                let next = Self::interpolate(&self.interp, &target, 0);
                (next, FadeStatus::Ungoing { step: 0 })
            }

            (FadeStatus::Ungoing { step }, true, false) => {
                if *step < FADE_STEPS {
                    let step = *step + 1;
                    let next = Self::interpolate(&self.interp, &target, step);
                    (next, FadeStatus::Ungoing { step })
                } else {
                    (target, FadeStatus::Completed)
                }
            }
        }
    }

    fn interpolate(
        start: &ColorSettings,
        end: &ColorSettings,
        step: u8,
    ) -> ColorSettings {
        let frac = step as f64 / FADE_STEPS as f64;
        let alpha = Self::ease_fade(frac)
            .clamp(0.0, 1.0)
            .try_into()
            .unwrap_or_else(|_| unreachable!());
        start.interpolate_with(end, alpha)
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

    fn write_verbose(&self, v: &mut Verbosity<impl Write>) -> Result<()> {
        let w = match v {
            Verbosity::High(w) => w,
            _ => return Ok(()),
        };

        let ColorSettings { temp, gamma, brght } = &self.interp;

        if self.fade == FadeStatus::Completed || self.interp_prev.is_none() {
            if Some(&self.period) != self.period_prev.as_ref() {
                writeln!(w, "{}", self.period)?;
            }
            match (&self.info, &self.info_prev) {
                (PeriodInfo::Elevation { .. }, None) => {
                    write!(w, "{}", self.info)?;
                }
                (
                    PeriodInfo::Elevation { elev: e1, .. },
                    Some(PeriodInfo::Elevation { elev: e2, .. }),
                ) if e1 != e2 => {
                    writeln!(w, "{e1}")?;
                }
                (
                    PeriodInfo::Elevation { loc: l1, .. },
                    Some(PeriodInfo::Elevation { loc: l2, .. }),
                ) if l1 != l2 => {
                    writeln!(w, "{l1}")?;
                }
                _ => {}
            }
            if Some(gamma) != self.interp_prev.as_ref().map(|c| &c.gamma) {
                writeln!(w, "{gamma}")?;
            }
            if Some(brght) != self.interp_prev.as_ref().map(|c| &c.brght) {
                writeln!(w, "{brght}")?;
            }
            if Some(temp) != self.interp_prev.as_ref().map(|c| &c.temp) {
                writeln!(w, "{temp}")?;
            }
        } else {
            if Some(temp) != self.interp_prev.as_ref().map(|c| &c.temp) {
                writeln!(w, "{temp}")?;
            }
        }

        w.flush()?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Signal {
    #[default]
    None,
    Interrupt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FadeStatus {
    Completed,
    Ungoing { step: u8 },
}

impl Default for FadeStatus {
    fn default() -> Self {
        Self::Completed
    }
}

impl FadeStatus {}

/// Periods of day
#[derive(Debug, Clone, Copy, PartialEq)]
enum Period {
    Daytime,
    Night,
    Transition {
        progress: u8, // Between 0 and 100
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

impl From<Period> for Alpha {
    fn from(period: Period) -> Self {
        match period {
            Period::Daytime => Self(1.0),
            Period::Night => Self(0.0),
            Period::Transition { progress } => Self(progress as f64 / 100.0),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PeriodInfo {
    Elevation { elev: Elevation, loc: Location },
    Time,
}

impl Period {
    /// Determine which period we are currently in based on time offset
    fn from_time(time: TimeOffset, time_ranges: TimeRanges) -> Self {
        let TimeRanges { dawn, dusk } = time_ranges;
        let sub =
            |a: TimeOffset, b: TimeOffset| (*a as i32 - *b as i32) as f64;

        if time < dawn.start || time >= dusk.end {
            Self::Night
        } else if time < dawn.end {
            let progress = sub(dawn.start, time) / sub(dawn.start, dawn.end);
            let progress = (progress * 100.0) as u8;
            Self::Transition { progress }
        } else if time > dusk.start {
            let progress = sub(dusk.end, time) / sub(dusk.end, dusk.start);
            let progress = (progress * 100.0) as u8;
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
            let progress = (progress * 100.0) as u8;
            Self::Transition { progress }
        } else {
            Self::Daytime
        }
    }

    fn from(
        scheme: &TransitionScheme,
        location: &LocationProvider,
        datetime: impl Fn() -> DateTime<Local>,
        v: &mut Verbosity<impl Write>,
    ) -> Result<(Self, PeriodInfo)> {
        match scheme {
            TransitionScheme::Elevation(elev_range) => {
                let now = (datetime().to_utc() - DateTime::UNIX_EPOCH)
                    .num_seconds() as f64;
                let here = location.get(v)?;
                let elev = Elevation::new(now, here);
                let period = Period::from_elevation(elev, *elev_range);
                let info = PeriodInfo::Elevation { elev, loc: here };
                Ok((period, info))
            }

            TransitionScheme::Time(time_ranges) => {
                let time = datetime().time().into();
                let period = Period::from_time(time, *time_ranges);
                Ok((period, PeriodInfo::Time))
            }
        }
    }
}

impl Default for Period {
    fn default() -> Self {
        Self::Daytime
    }
}

impl Default for PeriodInfo {
    fn default() -> Self {
        Self::Elevation {
            elev: Default::default(),
            loc: Default::default(),
        }
    }
}

//

pub trait Provider {
    // Listen and handle location updates
    // fn fd() -> c_int;

    fn get(&self, _v: &mut Verbosity<impl Write>) -> Result<Location> {
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
    fn set(&self, reset_ramps: bool, cs: &ColorSettings) -> Result<()> {
        Err(anyhow!("Temperature adjustment failed"))
    }
}

impl Provider for LocationProvider {
    fn get(&self, v: &mut Verbosity<impl Write>) -> Result<Location> {
        // b"Waiting for current location to become available...\n\0" as *const u8

        // Wait for location provider
        // b"Unable to get location from provider.\n\0" as *const u8 as *const c_char,
        // print_location(&mut loc);

        match self {
            Self::Manual(t) => t.get(v),
            // Self::Geoclue2(t) => t.get(),
        }
    }
}

impl AdjustmentMethod {
    fn restore(&self, dry_run: bool) -> Result<()> {
        match (dry_run, self) {
            (false, Self::Dummy(t)) => t.restore(),
            (false, Self::Randr(t)) => t.restore(),
            (false, Self::Drm(t)) => t.restore(),
            (false, Self::Vidmode(t)) => t.restore(),
            (true, _) => Ok(()),
        }
    }

    fn set(
        &self,
        dry_run: bool,
        reset_ramps: bool,
        cs: &ColorSettings,
    ) -> Result<()> {
        match (dry_run, self) {
            (false, Self::Dummy(t)) => t.set(reset_ramps, cs),
            (false, Self::Randr(t)) => t.set(reset_ramps, cs),
            (false, Self::Drm(t)) => t.set(reset_ramps, cs),
            (false, Self::Vidmode(t)) => t.set(reset_ramps, cs),
            (true, _) => Ok(()),
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
