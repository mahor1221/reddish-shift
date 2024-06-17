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

// TODO: https://doc.rust-lang.org/std/backtrace/
// TODO: add tldr page: https://github.com/tldr-pages/tldr
// TODO: benchmark: https://github.com/nvzqz/divan
// TODO: add setting screen brightness, a percentage of the current brightness
//       See: https://github.com/qualiaa/redshift-hooks
// TODO: #[instrument]: https://docs.rs//latest/tracing/index.html mod

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

use crate::{
    config::{ClapColorExt, Config, ConfigBuilder, FADE_STEPS, HEADER, WARN},
    types::{
        AdjustmentMethod, ColorSettings, Elevation, ElevationRange, Location,
        LocationProvider, Mode, Period, TimeOffset, TimeRanges,
        TransitionScheme,
    },
};

use anstream::AutoStream;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Local, SubsecRound, TimeDelta};
use std::{
    borrow::BorrowMut,
    cell::RefMut,
    fmt::{Debug, Write},
    io::{self, Stderr, StderrLock, Stdout, StdoutLock},
    rc::Rc,
    sync::mpsc::{self, Receiver, RecvTimeoutError},
};
use tracing::{info, warn, Level, Metadata, Subscriber};
use tracing_subscriber::{
    fmt::{writer::MakeWriterExt, MakeWriter},
    Layer,
};

pub struct StdErrLayer {}
impl<S: Subscriber> Layer<S> for StdErrLayer {}
pub struct StdOutLayer {}
impl<S: Subscriber> Layer<S> for StdOutLayer {}

fn main() -> Result<()> {
    let c = ConfigBuilder::new()?.build()?;
    // tracing_init(&c);

    if let (
        Mode::Daemon | Mode::Oneshot,
        TransitionScheme::Elevation(_),
        LocationProvider::Manual(l),
    ) = (&c.mode, &c.scheme, &c.location)
    {
        let loc = l.get()?;
        if loc.is_default() {
            warn!("{WARN}Warning{WARN:#}: Using default location ({loc})");
        }
    }

    if let AdjustmentMethod::Dummy(_) = c.method {
        let s = "Using dummy method! Display will not be affected";
        warn!("{WARN}Warning{WARN:#}: {s}");
    }

    let (tx, rx) = mpsc::channel();
    ctrlc::set_handler(move || {
        #[allow(clippy::expect_used)]
        tx.send(()).expect("Could not send signal on channel")
    })?;

    run(&c, &rx)
}

// fn tracing_init(c: &Config) {
//     let stdout = c.color.to_auto_stream(std::io::stdout()).lock();
//     let stderr = c.color.to_auto_stream(std::io::stderr()).lock();
//     let stdio = stderr.with_max_level(Level::WARN).or_else(stdout);

//     let (non_blocking, _stdout_guard) = tracing_appender::non_blocking(w);

//     racing_subscriber::fmt()
//         .with_writer(stdio)
//         .with_level(false)
//         .with_target(false)
//         .with_max_level(c.verbosity.level_filter())
//         .without_time()
//         .init();
// }

fn run(c: &Config, sig: &Receiver<()>) -> Result<()> {
    match c.mode {
        Mode::Daemon => {
            c.log()?;
            DaemonMode::new(c, sig).run_loop()?;
            c.method.restore()?;
        }
        Mode::Oneshot => {
            // Use period and transition progress to set color temperature
            let (p, i) = Period::from(&c.scheme, &c.location, c.time)?;
            let interp = c.night.interpolate_with(&c.day, p.into());
            c.log()?;
            info!("{HEADER}Current{HEADER:#}:\n{p}\n{i}{interp}");
            c.method.set(c.reset_ramps, &interp)?;
        }
        Mode::Set => {
            // for this command, color settings are stored in the day field
            c.method.set(c.reset_ramps, &c.day)?;
        }
        Mode::Reset => {
            c.method.set(true, &ColorSettings::default())?;
        }
        Mode::Print => run_print_mode(c)?,
    }

    Ok(())
}

fn run_print_mode(c: &Config) -> Result<()> {
    let now = (c.time)();
    let delta = now.to_utc() - DateTime::UNIX_EPOCH;
    let mut buf = String::from("Time     | Degree\n---------+-------\n");
    for d in (0..24).map(TimeDelta::hours) {
        let time = (now + d).time().trunc_subsecs(0);
        let elev = Elevation::new(
            (delta + d).num_seconds() as f64,
            c.location.get()?,
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
    fn run_loop(&mut self) -> Result<()> {
        let c = self.cfg;
        info!("{HEADER}Current{HEADER:#}:");

        loop {
            (self.period, self.info) =
                Period::from(&c.scheme, &c.location, c.time)?;

            let target = match self.signal {
                Signal::None => {
                    c.night.interpolate_with(&c.day, self.period.into())
                }
                Signal::Interrupt => ColorSettings::default(),
            };

            (self.interp, self.fade) = self.next_interpolate(target);

            self.log()?;

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
    ) -> Result<(Self, PeriodInfo)> {
        match scheme {
            TransitionScheme::Elevation(elev_range) => {
                let now = (datetime().to_utc() - DateTime::UNIX_EPOCH)
                    .num_seconds() as f64;
                let here = location.get()?;
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
    fn get(&self) -> Result<Location> {
        Err(anyhow!("Unable to get location from provider"))
    }
}

pub trait Adjuster {
    /// Restore the adjustment to the state before the Adjuster object was created
    fn restore(&self) -> Result<()> {
        Err(anyhow!("Temperature adjustment failed"))
    }

    /// Set a specific temperature
    fn set(&self, reset_ramps: bool, cs: &ColorSettings) -> Result<()>;
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Geoclue2;
impl Provider for Geoclue2 {
    // Listen and handle location updates
    // fn fd() -> c_int;

    fn get(&self) -> Result<Location> {
        // b"Waiting for current location to become available...\n\0" as *const u8

        // Wait for location provider
        // b"Unable to get location from provider.\n\0" as *const u8 as *const c_char,
        // print_location(&mut loc);
        Err(anyhow!("WIP"))
    }
}

impl Provider for LocationProvider {
    fn get(&self) -> Result<Location> {
        match self {
            Self::Manual(t) => t.get(),
            Self::Geoclue2(t) => t.get(),
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

//

pub struct StdioMakeWriter<'a> {
    stdout: Rc<RefCell<AutoStream<StdoutLock<'a>>>>,
    stderr: Rc<RefCell<AutoStream<StderrLock<'a>>>>,
}

/// A lock on either stdout or stderr, depending on the verbosity level of the
/// event being written.
// pub enum StdioLock<'a> {
//     Stdout(Rc<RefCell<AutoStream<StdoutLock<'a>>>>),
//     Stderr(Rc<RefCell<AutoStream<StderrLock<'a>>>>),
// }

// impl<'a> io::Write for StdioLock<'a> {
//     fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
//         match self {
//             StdioLock::Stdout(_) => todo!(),
//             StdioLock::Stderr(_) => todo!(),
//         }
//         // match self {
//         //     StdioLock::Stdout(lock) => lock.as_ref().borrow_mut().write(buf),
//         //     StdioLock::Stderr(lock) => lock.as_ref().borrow_mut().write(buf),
//         // }
//     }

//     fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
//         match self {
//             StdioLock::Stdout(lock) => lock.write_all(buf),
//             StdioLock::Stderr(lock) => lock.write_all(buf),
//         }
//     }

//     fn flush(&mut self) -> io::Result<()> {
//         match self {
//             StdioLock::Stdout(lock) => lock.flush(),
//             StdioLock::Stderr(lock) => lock.flush(),
//         }
//     }
// }

// impl<'a> MakeWriter<'a> for StdioMakeWriter<'a> {
//     type Writer = StdioLock<'a>;

//     fn make_writer(&'a self) -> Self::Writer {
//         // just return stdout in that case.
//         StdioLock::Stdout(self.stdout.clone())
//     }

//     fn make_writer_for(&'a self, meta: &Metadata<'_>) -> Self::Writer {
//         // Here's where we can implement our special behavior. We'll
//         // check if the metadata's verbosity level is WARN or ERROR,
//         // and return stderr in that cwith .
//         if meta.level() <= &Level::WARN {
//             return StdioLock::Stderr(self.stderr.clone());
//         }

//         // Otherwise, we'll return stdout.
//         StdioLock::Stdout(self.stdout.clone())
//     }
// }

pub trait IsDefault {
    fn is_default(&self) -> bool;
}

impl<T: Default + PartialEq> IsDefault for T {
    fn is_default(&self) -> bool {
        *self == T::default()
    }
}
