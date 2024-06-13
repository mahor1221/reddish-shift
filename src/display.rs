use crate::{
    config::{
        Brightness, Elevation, Gamma, Location, Temperature, Time, TimeOffset,
    },
    Period,
};
use std::fmt::{Display, Formatter, Result as FmtResult};

impl Display for TimeOffset {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        Display::fmt(&Time::from(*self), f)
    }
}

impl Display for Temperature {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "Screen temperature: {self}K")
    }
}

impl Display for Brightness {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "Screen brightness: {self:.2}")
    }
}

impl Display for Gamma {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "Screen gamma: {self:.2}")
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
        write!(f, "Solar elevation: {self:.2}°")
    }
}

impl Display for Period {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Period::Daytime => f.write_str("Daytime"),
            Period::Night => f.write_str("Night"),
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
