/*  types_display.rs -- Display implementation for types
    This file is part of <https://github.com/mahor1221/reddish-shift>.
    Copyright (C) 2024 Mahor Foruzesh <mahor1221@gmail.com>

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

use crate::{
    config::Config,
    types::{
        Brightness, ColorSettings, Elevation, ElevationRange, Gamma, Location,
        Period, PeriodInfo, Temperature, Time, TimeOffset, TimeRange,
        TimeRanges, TransitionScheme,
    },
    AdjustmentMethod, DaemonMode, FadeStatus, LocationProvider,
};
use anstyle::{AnsiColor, Color, Style};
use std::fmt::{self, Display, Formatter};
use tracing::info;

pub const WARN: Style = Style::new()
    .bold()
    .fg_color(Some(Color::Ansi(AnsiColor::Yellow)));
pub const HEADER: Style = Style::new().bold().underline();
pub const BODY: Style = Style::new().bold();

impl Display for Temperature {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}K", **self)
    }
}

impl Display for Brightness {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}%", **self as u8 * 100)
    }
}

impl Display for Gamma {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:.2}, {:.2}, {:.2}", self[0], self[1], self[2])
    }
}

impl Display for Time {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let Time { hour: h, minute: m } = self;
        write!(f, "{h:02}:{m:02}")
    }
}

impl Display for TimeOffset {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&Time::from(*self), f)
    }
}

impl Display for TimeRange {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let Self { start, end } = self;
        write!(f, "from {start} to {end}")
    }
}

impl Display for Elevation {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        //// TRANSLATORS: Append degree symbol if possible
        write!(f, "{:.2}°", **self)
    }
}

impl Display for Location {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let a = *self.lat;
        let b = *self.lon;
        let ns = if a >= 0.0 { "N" } else { "S" };
        let ew = if b >= 0.0 { "E" } else { "W" };
        let a = a.abs();
        let a1 = a as u8;
        let a2 = (a.fract() * 100.0) as u8;
        let a3 = ((a * 100.0).fract() * 100.0) as u8;
        let b = b.abs();
        let b1 = b as u8;
        let b2 = (b.fract() * 100.0) as u8;
        let b3 = ((b * 100.0).fract() * 100.0) as u8;
        write!(f, "{a1}°{a2}′{a3}″{ns}, {b1}°{b2}′{b3}″{ew}")
    }
}

//

impl Display for ColorSettings {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let ColorSettings { temp, gamma, brght } = self;
        write!(
            f,
            "    {BODY}Temperature{BODY:#}: {temp}
    {BODY}Brightness{BODY:#}: {brght}
    {BODY}Gamma{BODY:#}: {gamma}"
        )
    }
}

impl Display for Period {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Period::Night => write!(f, "    {BODY}Period{BODY:#}: night"),
            Period::Daytime => write!(f, "    {BODY}Period{BODY:#}: daytime"),
            Period::Transition { progress } => {
                write!(
                    f,
                    "    {BODY}Period{BODY:#}: transition ({progress}% day)"
                )
            }
        }
    }
}

impl Display for PeriodInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            PeriodInfo::Time => Ok(()),
            PeriodInfo::Elevation { elev, loc } => {
                write!(
                    f,
                    "    {BODY}Solar elevation{BODY:#}: {elev}
    {BODY}Location{BODY:#}: {loc}"
                )
            }
        }
    }
}

impl Display for Config {
    #[allow(clippy::too_many_lines)]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let Config {
            day,
            night,
            scheme,
            location,
            method,
            reset_ramps,
            disable_fade,
            sleep_duration_short: _,
            sleep_duration: _,
            mode: _,
            verbosity: _,
            color: _,
            time: _,
        } = self;

        let l = match location {
            LocationProvider::Manual(_) => "manual",
            LocationProvider::Geoclue2(_) => "geoclue2",
        };
        writeln!(f, "{BODY}Location provider{BODY:#}: {l}")?;

        let m = match method {
            AdjustmentMethod::Dummy(_) => "dummy",
            AdjustmentMethod::Randr(_) => "randr",
            AdjustmentMethod::Drm(_) => "drm",
            AdjustmentMethod::Vidmode(_) => "vidmode",
        };
        writeln!(f, "{BODY}Adjustment method{BODY:#}: {m}")?;

        writeln!(f, "{BODY}Reset ramps{BODY:#}: {reset_ramps}")?;
        writeln!(f, "{BODY}Disable fade{BODY:#}: {disable_fade}")?;

        writeln!(f, "{HEADER}Daytime{HEADER:#}:")?;
        match scheme {
            TransitionScheme::Time(TimeRanges {
                dawn: TimeRange { end, .. },
                dusk: TimeRange { start, .. },
            }) => {
                writeln!(f, "    {BODY}Time{BODY:#}: from {end} to {start}")?;
            }
            TransitionScheme::Elev(ElevationRange { high, .. }) => {
                writeln!(
                    f,
                    "    {BODY}Solar elevation{BODY:#}: above {high}"
                )?;
            }
        }
        writeln!(f, "{day}")?;

        writeln!(f, "{HEADER}Night{HEADER:#}:")?;
        match scheme {
            TransitionScheme::Time(TimeRanges {
                dawn: TimeRange { start, .. },
                dusk: TimeRange { end, .. },
            }) => {
                writeln!(f, "    {BODY}Time{BODY:#}: from {end} to {start}")?;
            }
            TransitionScheme::Elev(ElevationRange { low, .. }) => {
                writeln!(f, "    {BODY}Solar elevation{BODY:#}: below {low}")?;
            }
        }
        write!(f, "{night}")?;
        Ok(())
    }
}

impl DaemonMode<'_, '_> {
    #[allow(clippy::too_many_lines)]
    pub fn log(&self) {
        if Some(&self.period) != self.prev_period.as_ref() {
            info!("{}", self.period);
        }
        match (&self.info, &self.prev_info) {
            (
                PeriodInfo::Elevation { elev: e1, .. },
                Some(PeriodInfo::Elevation { elev: e2, .. }),
            ) if e1 != e2 => {
                info!("    {BODY}Solar elevation{BODY:#}: {e1}");
            }
            (
                PeriodInfo::Elevation { loc: l1, .. },
                Some(PeriodInfo::Elevation { loc: l2, .. }),
            ) if l1 != l2 => {
                info!("    {BODY}Location{BODY:#}: {l1}");
            }
            (PeriodInfo::Elevation { .. }, None) => {
                info!("{}", self.info);
            }
            (
                i1 @ PeriodInfo::Elevation { .. },
                Some(i2 @ PeriodInfo::Elevation { .. }),
            ) if i1 != i2 => {
                info!("{}", self.info);
            }
            _ => {}
        }

        let ColorSettings { temp, gamma, brght } = &self.interp;
        if self.fade == FadeStatus::Completed || self.prev_interp.is_none() {
            if Some(temp) != self.prev_interp.as_ref().map(|c| &c.temp) {
                info!("    {BODY}Temperature{BODY:#}: {temp}");
            }
            if Some(gamma) != self.prev_interp.as_ref().map(|c| &c.gamma) {
                info!("    {BODY}Gamma{BODY:#}: {gamma}");
            }
            if Some(brght) != self.prev_interp.as_ref().map(|c| &c.brght) {
                info!("    {BODY}Brightness{BODY:#}: {brght}");
            }
        } else if Some(temp) != self.prev_interp.as_ref().map(|c| &c.temp) {
            info!("    {BODY}Temperature{BODY:#}: {temp}");
        }
    }
}
