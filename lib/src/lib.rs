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

// TODO: add tldr page: https://github.com/tldr-pages/tldr
// TODO: add setting screen brightness, a percentage of the current brightness
//       See: https://github.com/qualiaa/redshift-hooks
// TODO: ? benchmark: https://github.com/nvzqz/divan
// TODO: Fix large fade steps
// TODO: ? Box large errors
// TODO: Win support & Choco package
// TODO: move coproduct.rs to a fork of frunk after Error got stable in core
//       https://github.com/rust-lang/rust/issues/103765

mod calc_colorramp;
mod calc_solar;
mod cli;
mod config;
mod coproduct;
mod error;
mod gamma_drm;
mod gamma_dummy;
mod gamma_randr;
mod gamma_vidmode;
mod location_manual;
mod types;
mod types_display;
mod types_parse;
mod utils;

pub use cli::cli_args_command;
use error::ReddishError;
pub use gamma_drm::Drm;
pub use gamma_dummy::Dummy;
pub use gamma_randr::Randr;
pub use gamma_vidmode::Vidmode;
use itertools::Itertools;
pub use location_manual::Manual;
use types::Location;

use crate::{
    cli::ClapColorChoiceExt,
    config::{Config, ConfigBuilder, FADE_STEPS},
    error::{AdjusterError, ProviderError},
    types::{ColorSettings, Elevation, Mode, Period, PeriodInfo},
    types_display::{BODY, HEADER},
};
use anstream::AutoStream;
use chrono::{DateTime, SubsecRound, TimeDelta};
use std::{
    fmt::Debug,
    io,
    sync::mpsc::{self, Receiver, RecvTimeoutError},
};
use tracing::{error, info, Level};
use tracing_subscriber::fmt::writer::MakeWriterExt;

pub fn main() {
    (|| -> Result<(), ReddishError> {
        let c = ConfigBuilder::new(|verbosity, color| {
            let choice = color.to_choice();
            let stdout = move || AutoStream::new(io::stdout(), choice).lock();
            let stderr = move || AutoStream::new(io::stderr(), choice).lock();
            let stdio = stderr.with_max_level(Level::WARN).or_else(stdout);

            tracing_subscriber::fmt()
                .with_writer(stdio)
                .with_max_level(verbosity.level_filter())
                .without_time()
                .with_level(false)
                .with_target(false)
                .init();
        })?
        .build()?;

        let (tx, rx) = mpsc::channel();
        ctrlc::set_handler(move || {
            #[allow(clippy::expect_used)]
            tx.send(()).expect("Could not send signal on channel")
        })
        .or_else(|e| match c.mode {
            Mode::Oneshot | Mode::Set | Mode::Reset | Mode::Print => Ok(()),
            Mode::Daemon => Err(e),
        })?;

        run(&c, &rx)
    })()
    .unwrap_or_else(|e| error!("{e}"))
}

fn run(c: &Config, sig: &Receiver<()>) -> Result<(), ReddishError> {
    match c.mode {
        Mode::Daemon => {
            info!("{c}\n{HEADER}Current{HEADER:#}:");
            DaemonMode::new(c, sig).run_loop()?;
            c.method.restore()?;
        }
        Mode::Oneshot => {
            // Use period and transition progress to set color temperature
            let (p, i) = Period::from(&c.scheme, &c.location, c.time)?;
            let interp = c.night.interpolate_with(&c.day, p.into());
            info!("{c}\n{HEADER}Current{HEADER:#}:\n{p}\n{i}\n{interp}");
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

fn run_print_mode(c: &Config) -> Result<(), ReddishError> {
    let now = (c.time)();
    let delta = now.to_utc() - DateTime::UNIX_EPOCH;
    let loc = c.location.get()?;
    let mut buf = (0..24).map(|h| {
        let d = TimeDelta::hours(h);
        let time = (now + d).time().trunc_subsecs(0);
        let elev = Elevation::new((delta + d).num_seconds() as f64, loc);
        format!("{BODY}{time}{BODY:#}: {:6.2}°", *elev)
    });
    Ok(info!("{}", buf.join("\n")))
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
    fn run_loop(&mut self) -> Result<(), ReddishError> {
        let c = self.cfg;
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

            self.log();

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

trait Provider {
    fn get(&self) -> Result<Location, ProviderError>;
}

trait Adjuster {
    /// Restore the adjustment to the state before the Adjuster object was created
    fn restore(&self) -> Result<(), AdjusterError>;
    /// Set a specific temperature
    fn set(
        &self,
        reset_ramps: bool,
        cs: &ColorSettings,
    ) -> Result<(), AdjusterError>;
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

//

#[derive(Debug, PartialEq)]
pub enum LocationProvider {
    Manual(Manual),
    Geoclue2(Geoclue2),
}

#[derive(Debug)]
pub enum AdjustmentMethod {
    Dummy(Dummy),
    Randr(Randr),
    Drm(Drm),
    Vidmode(Vidmode),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Geoclue2;

impl Provider for Geoclue2 {
    // Listen and handle location updates
    // fn fd() -> c_int;

    fn get(&self) -> Result<Location, ProviderError> {
        // b"Waiting for current location to become available...\n\0" as *const u8
        // Wait for location provider
        Err(ProviderError)
    }
}

impl Provider for LocationProvider {
    fn get(&self) -> Result<Location, ProviderError> {
        match self {
            Self::Manual(t) => t.get(),
            Self::Geoclue2(t) => t.get(),
        }
    }
}

impl Adjuster for AdjustmentMethod {
    fn restore(&self) -> Result<(), AdjusterError> {
        match self {
            Self::Dummy(t) => t.restore(),
            Self::Randr(t) => t.restore(),
            Self::Drm(t) => t.restore(),
            Self::Vidmode(t) => t.restore(),
        }
    }

    fn set(
        &self,
        reset_ramps: bool,
        cs: &ColorSettings,
    ) -> Result<(), AdjusterError> {
        match self {
            Self::Dummy(t) => t.set(reset_ramps, cs),
            Self::Randr(t) => t.set(reset_ramps, cs),
            Self::Drm(t) => t.set(reset_ramps, cs),
            Self::Vidmode(t) => t.set(reset_ramps, cs),
        }

        // TODO: MacOS support
        // // In Quartz (macOS) the gamma adjustments will
        // // automatically revert when the process exits
        // // Therefore, we have to loop until CTRL-C is received
        // if strcmp(options.method.name, "quartz") == 0 {
        //     // b"Press ctrl-c to stop...\n" as *const u8 as *const c_char,
        //     pause();
        // }
    }
}
