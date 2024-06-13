use crate::{
    config::{
        Brightness, ColorSettings, Elevation, Gamma, Location, Temperature,
        Time, TimeOffset,
    },
    Period, PeriodInfo,
};
use std::fmt::{Display, Formatter, Result as FmtResult};

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
        write!(f, "Temperature: {}K", **self)
    }
}

impl Display for Brightness {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "Brightness: {}%", **self as u8 * 100)
    }
}

impl Display for Gamma {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "Gamma: {:.2}, {:.2}, {:.2}", self[0], self[1], self[2])
    }
}

impl Display for Time {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let Time { hour: h, minute: m } = self;
        write!(f, "Time: {h:2}:{m:2}")
    }
}

impl Display for Elevation {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        //// TRANSLATORS: Append degree symbol if possible
        write!(f, "Solar elevation: {:.2}°", **self)
    }
}

impl Display for Period {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Period::Daytime => f.write_str("Period: Daytime"),
            Period::Night => f.write_str("Period: Night"),
            Period::Transition { progress } => {
                write!(f, "Period: Transition ({progress}% day)")
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
        write!(f, "Location: {a1}°{a2}′{a3}″{ns}, {b1}°{b2}′{b3}″{ew}")
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
