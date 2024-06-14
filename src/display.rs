use crate::{
    config::{
        AdjustmentMethod, Brightness, ColorSettings, Config, Elevation,
        ElevationRange, Gamma, Location, LocationProvider, Temperature, Time,
        TimeOffset, TimeRange, TimeRanges, TransitionScheme, Verbosity,
    },
    DaemonMode, FadeStatus, Period, PeriodInfo,
};
use anyhow::Result;
use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    io::Write as IoWrite,
};

impl Display for TimeOffset {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        Display::fmt(&Time::from(*self), f)
    }
}

impl Display for ColorSettings {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let ColorSettings { temp, gamma, brght } = self;
        write!(f, "{temp}\n{brght}\n{gamma}")
    }
}

impl Display for Temperature {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "    Temperature: {}K", **self)
    }
}

impl Display for Brightness {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "    Brightness: {}%", **self as u8 * 100)
    }
}

impl Display for Gamma {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(
            f,
            "    Gamma: {:.2}, {:.2}, {:.2}",
            self[0], self[1], self[2]
        )
    }
}

impl Display for Time {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let Time { hour: h, minute: m } = self;
        write!(f, "{h:02}:{m:02}")
    }
}

impl Display for Elevation {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        //// TRANSLATORS: Append degree symbol if possible
        write!(f, "    Solar elevation: {:.2}°", **self)
    }
}

impl Display for Period {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Period::Daytime => f.write_str("    Period: Daytime"),
            Period::Night => f.write_str("    Period: Night"),
            Period::Transition { progress } => {
                write!(f, "    Period: Transition ({progress}% day)")
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
        let a1 = a as u8;
        let a2 = (a.fract() * 100.0) as u8;
        let a3 = ((a * 100.0).fract() * 100.0) as u8;
        let b = b.abs();
        let b1 = b as u8;
        let b2 = (b.fract() * 100.0) as u8;
        let b3 = ((b * 100.0).fract() * 100.0) as u8;
        write!(f, "    Location: {a1}°{a2}′{a3}″{ns}, {b1}°{b2}′{b3}″{ew}")
    }
}

impl Display for PeriodInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            PeriodInfo::Time => Ok(()),
            PeriodInfo::Elevation { elev, loc } => {
                write!(f, "{elev}\n{loc}\n")
            }
        }
    }
}

impl Display for TimeRange {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let Self { start, end } = self;
        write!(f, "from {start} to {end}")
    }
}

//

impl Config {
    #[allow(clippy::too_many_lines)]
    pub fn write_verbose(
        &self,
        v: &mut Verbosity<impl IoWrite>,
    ) -> Result<()> {
        let Config {
            day,
            night,
            scheme,
            location,
            method,
            reset_ramps,
            disable_fade,
            fade_sleep_duration: _,
            sleep_duration: _,
            mode: _,
            time: _,
        } = self;

        let Verbosity::High(w) = v else { return Ok(()) };

        write!(w, "Location provider: ")?;
        match location {
            LocationProvider::Manual(_) => writeln!(w, "manual")?,
            LocationProvider::Geoclue2(_) => writeln!(w, "geoclue2")?,
        }

        write!(w, "Adjustment method: ")?;
        match method {
            AdjustmentMethod::Dummy(_) => writeln!(w, "dummy")?,
            AdjustmentMethod::Randr(_) => writeln!(w, "randr")?,
            AdjustmentMethod::Drm(_) => writeln!(w, "drm")?,
            AdjustmentMethod::Vidmode(_) => writeln!(w, "vidmode")?,
        }

        writeln!(w, "Reset ramps: {reset_ramps}")?;
        writeln!(w, "Disable fade: {disable_fade}")?;

        writeln!(w, "Daytime:")?;
        match scheme {
            TransitionScheme::Time(TimeRanges {
                dawn: TimeRange { end, .. },
                dusk: TimeRange { start, .. },
            }) => {
                writeln!(w, "    Time: from {end} to {start}")?;
            }
            TransitionScheme::Elevation(ElevationRange { high, .. }) => {
                writeln!(w, "    Solar elevation: above {:.2}°", **high)?;
            }
        }
        let ColorSettings { temp, gamma, brght } = day;
        writeln!(w, "{temp}")?;
        writeln!(w, "{gamma}")?;
        writeln!(w, "{brght}")?;

        writeln!(w, "Night:")?;
        match scheme {
            TransitionScheme::Time(TimeRanges {
                dawn: TimeRange { start, .. },
                dusk: TimeRange { end, .. },
            }) => {
                writeln!(w, "    Time: from {end} to {start}")?;
            }
            TransitionScheme::Elevation(ElevationRange { low, .. }) => {
                writeln!(w, "    Solar elevation: below {:.2}°", **low)?;
            }
        }
        let ColorSettings { temp, gamma, brght } = night;
        writeln!(w, "{temp}")?;
        writeln!(w, "{gamma}")?;
        writeln!(w, "{brght}")?;

        Ok(())
    }
}

impl DaemonMode<'_, '_> {
    pub fn write_verbose(
        &self,
        v: &mut Verbosity<impl IoWrite>,
    ) -> Result<()> {
        let Verbosity::High(w) = v else { return Ok(()) };

        if Some(&self.period) != self.prev_period.as_ref() {
            writeln!(w, "{}", self.period)?;
        }
        match (&self.info, &self.prev_info) {
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

        let ColorSettings { temp, gamma, brght } = &self.interp;
        if self.fade == FadeStatus::Completed || self.prev_interp.is_none() {
            if Some(temp) != self.prev_interp.as_ref().map(|c| &c.temp) {
                writeln!(w, "{temp}")?;
            }
            if Some(gamma) != self.prev_interp.as_ref().map(|c| &c.gamma) {
                writeln!(w, "{gamma}")?;
            }
            if Some(brght) != self.prev_interp.as_ref().map(|c| &c.brght) {
                writeln!(w, "{brght}")?;
            }
        } else if Some(temp) != self.prev_interp.as_ref().map(|c| &c.temp) {
            writeln!(w, "{temp}")?;
        }

        w.flush()?;
        Ok(())
    }
}
