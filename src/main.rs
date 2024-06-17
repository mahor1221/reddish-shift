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

// TODO: use snafu for error handling: https://github.com/shepmaster/snafu
// TODO: add tldr page: https://github.com/tldr-pages/tldr
// TODO: benchmark: https://github.com/nvzqz/divan
// TODO: add setting screen brightness, a percentage of the current brightness
//       See: https://github.com/qualiaa/redshift-hooks

mod calc_colorramp;
mod calc_solar;
mod cli;
mod config;
mod gamma_drm;
mod gamma_dummy;
mod gamma_randr;
mod gamma_vidmode;
mod location_manual;
mod types;
mod types_display;
mod types_parse;
mod utils;

use crate::{
    config::{Config, ConfigBuilder, FADE_STEPS},
    types::{
        AdjustmentMethod, ColorSettings, Elevation, ElevationRange, Location,
        LocationProvider, Mode, Period, TimeOffset, TimeRanges,
        TransitionScheme, Verbosity,
    },
    utils::IsDefault,
};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Local, SubsecRound, TimeDelta};
use std::{
    fmt::{Debug, Write as FmtWrite},
    io::Write as IoWrite,
    sync::mpsc::{self, Receiver, RecvTimeoutError},
};

fn main() -> Result<()> {
    let stdout = std::io::stdout();
    let stderr = std::io::stderr();
    let (c, mut v) =
        ConfigBuilder::new()?.build(stdout.lock(), stderr.lock())?;

    if let (
        Mode::Daemon | Mode::Oneshot,
        TransitionScheme::Elevation(_),
        LocationProvider::Manual(l),
    ) = (&c.mode, &c.scheme, &c.location)
    {
        if l.get(&mut v)?.is_default() {
            ewriteln!(&mut v, "Warning: using default location")?;
        }
    }

    if let AdjustmentMethod::Dummy(_) = c.method {
        let s = "Warning: Using dummy method! Display will not be affected";
        ewriteln!(&mut v, "{s}")?;
    }

    let (tx, rx) = mpsc::channel();
    ctrlc::set_handler(move || {
        #[allow(clippy::expect_used)]
        tx.send(()).expect("Could not send signal on channel")
    })?;

    run(&c, &rx, &mut v)
}

fn run(
    c: &Config,
    sig: &Receiver<()>,
    v: &mut Verbosity<impl IoWrite, impl IoWrite>,
) -> Result<()> {
    match c.mode {
        Mode::Daemon => {
            c.write_verbose(v)?;
            DaemonMode::new(c, sig).run_loop(v)?;
            c.method.restore()?;
        }
        Mode::Oneshot => {
            // Use period and transition progress to set color temperature
            let (p, i) = Period::from(&c.scheme, &c.location, c.time, v)?;
            let interp = c.night.interpolate_with(&c.day, p.into());
            c.write_verbose(v)?;
            writeln_verbose!(v, "Current:\n{p}\n{i}{interp}")?;
            c.method.set(c.reset_ramps, &interp)?;
        }
        Mode::Set => {
            // for this command, color settings are stored in the day field
            c.method.set(c.reset_ramps, &c.day)?;
        }
        Mode::Reset => {
            c.method.set(true, &ColorSettings::default())?;
        }
        Mode::Print => run_print_mode(c, v)?,
    }

    Ok(())
}

fn run_print_mode(
    c: &Config,
    v: &mut Verbosity<impl IoWrite, impl IoWrite>,
) -> Result<()> {
    let now = (c.time)();
    let delta = now.to_utc() - DateTime::UNIX_EPOCH;
    let mut buf = String::from("Time     | Degree\n---------+-------\n");
    for d in (0..24).map(TimeDelta::hours) {
        let time = (now + d).time().trunc_subsecs(0);
        let elev = Elevation::new(
            (delta + d).num_seconds() as f64,
            c.location.get(v)?,
        );
        writeln!(&mut buf, "{time} | {:6.2}", *elev)?;
    }

    Ok(print!("{buf}"))
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
    prev_period: Option<Period>,
    prev_info: Option<PeriodInfo>,
    prev_interp: Option<ColorSettings>,
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
            prev_period: Default::default(),
            prev_info: Default::default(),
            prev_interp: Default::default(),
        }
    }

    /// This is the main loop of the daemon mode which keeps track of the
    /// current time and continuously updates the screen to the appropriate
    /// color temperature
    fn run_loop(
        &mut self,
        v: &mut Verbosity<impl IoWrite, impl IoWrite>,
    ) -> Result<()> {
        let c = self.cfg;
        writeln_verbose!(v, "Current:")?;

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

            if Some(&self.interp) != self.prev_interp.as_ref() {
                c.method.set(c.reset_ramps, &self.interp)?;
            }

            self.prev_period = Some(self.period);
            self.prev_info = Some(self.info.clone());
            self.prev_interp = Some(self.interp.clone());

            // sleep for a duration then continue the loop
            // or wake up and restore the default colors slowly on first ctrl-c
            // or break the loop on the second ctrl-c immediately
            let sleep_duration = match (self.signal, self.fade) {
                (Signal::None, FadeStatus::Completed) => c.sleep_duration,
                (_, FadeStatus::Ungoing { .. }) => c.sleep_duration_short,
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
        use FadeStatus::*;
        let target_is_very_different = self.interp.is_very_diff_from(&target);
        match (&self.fade, target_is_very_different, self.cfg.disable_fade) {
            (_, _, true) | (Completed | Ungoing { .. }, false, false) => {
                (target, Completed)
            }

            (Completed, true, false) => {
                let next = Self::interpolate(&self.interp, &target, 0);
                (next, Ungoing { step: 0 })
            }

            (Ungoing { step }, true, false) => {
                if *step < FADE_STEPS {
                    let step = *step + 1;
                    let next = Self::interpolate(&self.interp, &target, step);
                    (next, Ungoing { step })
                } else {
                    (target, Completed)
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

#[derive(Debug, Clone, PartialEq)]
enum PeriodInfo {
    Elevation { elev: Elevation, loc: Location },
    Time,
}

impl Default for PeriodInfo {
    fn default() -> Self {
        Self::Elevation {
            elev: Default::default(),
            loc: Default::default(),
        }
    }
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
        v: &mut Verbosity<impl IoWrite, impl IoWrite>,
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

//

pub trait Provider {
    fn get(
        &self,
        _v: &mut Verbosity<impl IoWrite, impl IoWrite>,
    ) -> Result<Location> {
        Err(anyhow!("Unable to get location from provider"))
    }
}

pub trait Adjuster {
    /// Restore the adjustment to the state before the Adjuster object was created
    fn restore(&self) -> Result<()> {
        Err(anyhow!("Temperature adjustment failed"))
    }

    /// Set a specific temperature
    #[allow(unused_variables)]
    fn set(&self, reset_ramps: bool, cs: &ColorSettings) -> Result<()> {
        Err(anyhow!("Temperature adjustment failed"))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Geoclue2;
impl Provider for Geoclue2 {
    // Listen and handle location updates
    // fn fd() -> c_int;

    fn get(
        &self,
        _v: &mut Verbosity<impl IoWrite, impl IoWrite>,
    ) -> Result<Location> {
        // b"Waiting for current location to become available...\n\0" as *const u8

        // Wait for location provider
        // b"Unable to get location from provider.\n\0" as *const u8 as *const c_char,
        // print_location(&mut loc);
        Err(anyhow!("WIP"))
    }
}

impl Provider for LocationProvider {
    fn get(
        &self,
        v: &mut Verbosity<impl IoWrite, impl IoWrite>,
    ) -> Result<Location> {
        match self {
            Self::Manual(t) => t.get(v),
            Self::Geoclue2(t) => t.get(v),
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

    fn set(&self, reset_ramps: bool, cs: &ColorSettings) -> Result<()> {
        match self {
            Self::Dummy(t) => t.set(reset_ramps, cs),
            Self::Randr(t) => t.set(reset_ramps, cs),
            Self::Drm(t) => t.set(reset_ramps, cs),
            Self::Vidmode(t) => t.set(reset_ramps, cs),
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
