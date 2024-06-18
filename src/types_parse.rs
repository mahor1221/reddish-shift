/*  types_parse.rs -- FromStr implementation for types
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

use crate::types::{
    AdjustmentMethodType, Brightness, DayNight, Elevation, ElevationRange,
    Gamma, Latitude, Location, LocationProviderType, Longitude, Temperature,
    Time, TimeOffset, TimeRange, TimeRanges, TransitionScheme,
};
use anyhow::{anyhow, Result};
use std::str::FromStr;

//

impl FromStr for Temperature {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.trim().parse::<u16>()?.try_into()
    }
}

impl FromStr for Brightness {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.trim().parse::<f64>()?.try_into()
    }
}

impl FromStr for Gamma {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match *s.split(':').map(str::trim).collect::<Vec<_>>().as_slice() {
            [r, g, b] => {
                [r.parse::<f64>()?, g.parse::<f64>()?, b.parse::<f64>()?]
                    .try_into()
            }
            [rbg] => rbg.parse::<f64>()?.try_into(),
            _ => Err(anyhow!("gamma")),
        }
    }
}

impl FromStr for Latitude {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.trim().parse::<f64>()?.try_into()
    }
}

impl FromStr for Longitude {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.trim().parse::<f64>()?.try_into()
    }
}

impl FromStr for Location {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match *s.split(':').collect::<Vec<_>>().as_slice() {
            [lat, lon] => Ok(Self {
                lat: lat.parse()?,
                lon: lon.parse()?,
            }),
            _ => Err(anyhow!("location")),
        }
    }
}

impl FromStr for Time {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match *s.split(':').map(str::trim).collect::<Vec<_>>().as_slice() {
            [h, m] => (h.parse()?, m.parse()?).try_into(),
            _ => Err(anyhow!("time")),
        }
    }
}

impl FromStr for TimeOffset {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.parse::<Time>()?.into())
    }
}

impl FromStr for Elevation {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.trim().parse::<f64>()?.try_into()
    }
}

impl FromStr for TimeRange {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match *s.split('-').collect::<Vec<_>>().as_slice() {
            [start, end] => {
                (start.parse::<TimeOffset>()?, end.parse::<TimeOffset>()?)
                    .try_into()
            }
            [time] => {
                let t = time.parse::<TimeOffset>()?;
                Ok(Self { start: t, end: t })
            }
            _ => Err(anyhow!("time_range")),
        }
    }
}

impl FromStr for TimeRanges {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let time = match *s.split('-').collect::<Vec<_>>().as_slice() {
            [dawn, dusk] => Self {
                dawn: dawn.parse()?,
                dusk: dusk.parse()?,
            },
            [dawn_start, dawn_end, dusk_start, dusk_end] => Self {
                dawn: (dawn_start.parse()?, dawn_end.parse()?).try_into()?,
                dusk: (dusk_start.parse()?, dusk_end.parse()?).try_into()?,
            },
            _ => Err(anyhow!("time_ranges"))?,
        };

        if time.dawn.end < time.dusk.start {
            Ok(time)
        } else {
            Err(anyhow!("time_ranges"))
        }
    }
}

impl FromStr for ElevationRange {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match *s.split(':').collect::<Vec<_>>().as_slice() {
            [high, low] => {
                let high = high.parse()?;
                let low = low.parse()?;
                if high >= low {
                    Ok(Self { high, low })
                } else {
                    // b"High transition elevation cannot be lower than the low transition elevation.\n\0"
                    Err(anyhow!("elevation"))
                }
            }
            _ => Err(anyhow!("elevation")),
        }
    }
}

impl FromStr for TransitionScheme {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Err(())
            .or_else(|_| Ok::<_, Self::Err>(Self::Time(s.parse()?)))
            .or_else(|_| Ok::<_, Self::Err>(Self::Elevation(s.parse()?)))
            .map_err(|_| anyhow!("asdf"))
    }
}

impl FromStr for LocationProviderType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "geoclue2" => Ok(Self::Geoclue2),
            _ => s.parse().map(Self::Manual),
        }
        .map_err(|_| anyhow!("asdf"))
    }
}

impl FromStr for AdjustmentMethodType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let num = |o: Option<&str>| match o {
            None => Ok::<_, anyhow::Error>(None),
            Some(s) => Ok(Some(s.parse()?)),
        };
        let crtcs = |o: Option<&str>| match o {
            None => Ok(Vec::new()),
            Some(s) => s
                .split(',')
                .map(|s| Ok(s.trim().parse()?))
                .collect::<Result<Vec<_>>>(),
        };

        let drm = |n: Option<&str>, c: Option<&str>| {
            Ok(Self::Drm {
                card_num: num(n)?,
                crtcs: crtcs(c)?,
            })
        };
        let randr = |n: Option<&str>, c: Option<&str>| {
            Ok(Self::Randr {
                screen_num: num(n)?,
                crtcs: crtcs(c)?,
            })
        };

        match s.split(':').map(str::trim).collect::<Vec<_>>().as_slice() {
            ["dummy"] => Ok(Self::Dummy),
            ["drm"] => drm(None, None),
            ["drm", n] => drm(Some(n), None),
            ["drm", n, c] => drm(Some(n), Some(c)),
            ["vidmode"] => Ok(Self::Vidmode { screen_num: None }),
            ["vidmode", n] => Ok(Self::Vidmode {
                screen_num: Some(n.parse()?),
            }),
            ["randr"] => randr(None, None),
            ["randr", n] => randr(Some(n), None),
            ["randr", n, c] => randr(Some(n), Some(c)),
            _ => Err(anyhow!("method")),
        }
    }
}

impl<T: Clone + FromStr<Err = anyhow::Error>> FromStr for DayNight<T> {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match *s.split('-').collect::<Vec<_>>().as_slice() {
            [day_night] => {
                let day_night = day_night.parse::<T>()?;
                Ok(Self {
                    day: day_night.clone(),
                    night: day_night,
                })
            }
            [day, night] => Ok(Self {
                day: day.parse()?,
                night: night.parse()?,
            }),
            _ => Err(anyhow!("day_night")),
        }
    }
}
