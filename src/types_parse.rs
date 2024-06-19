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

use frunk::validated::IntoValidated;

use crate::{
    coproduct::InjectErr,
    error::*,
    types::{
        gamma, hour, minute, AdjustmentMethodType, Brightness, DayNight,
        Elevation, ElevationRange, Gamma, Latitude, Location,
        LocationProviderType, Longitude, Temperature, Time, TimeOffset,
        TimeRange, TimeRanges, TransitionScheme,
    },
    utils::FromGeneric,
};
use std::str::FromStr;

impl FromStr for Temperature {
    type Err = ParseErrorTemperature;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.trim().parse::<u16>()?.try_into()?)
    }
}

impl FromStr for Brightness {
    type Err = ParseErrorBrightness;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.trim().parse::<f64>()?.try_into()?)
    }
}

impl FromStr for Gamma {
    type Err = ParseErrorGammaRgb;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let f = |s: &str| -> Result<f64, ParseErrorGamma> {
            gamma(s.parse::<f64>().inject_err()?).inject_err()
        };

        match *s.split(':').map(str::trim).collect::<Vec<_>>().as_slice() {
            [r, g, b] => Ok((f(r).into_validated() + f(g) + f(b))
                .into_result()?
                .from_generic::<(f64, f64, f64)>()
                .try_into()
                .unwrap_or_else(|_| unreachable!())),
            [rbg] => Ok(f(rbg)?.try_into().unwrap_or_else(|_| unreachable!())),
            _ => Err(ParseErrorGammaRgb::Fmt),
        }
    }
}

impl FromStr for Latitude {
    type Err = ParseErrorLatitude;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.trim().parse::<f64>()?.try_into()?)
    }
}

impl FromStr for Longitude {
    type Err = ParseErrorLongitude;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.trim().parse::<f64>()?.try_into()?)
    }
}

impl FromStr for Location {
    type Err = ParseErrorLocation;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match *s.split(':').collect::<Vec<_>>().as_slice() {
            [lat, lon] => Ok((lat.parse().inject_err().into_validated()
                + lon.parse().inject_err())
            .into_result()?
            .from_generic()),
            _ => Err(ParseErrorLocation::Fmt),
        }
    }
}

impl FromStr for Time {
    type Err = ParseErrorTime;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let h = |s: &str| -> Result<u8, ParseErrorTimeT> {
            hour(s.parse::<u8>().inject_err()?).inject_err()
        };
        let m = |s: &str| -> Result<u8, ParseErrorTimeT> {
            minute(s.parse::<u8>().inject_err()?).inject_err()
        };
        match *s.split(':').map(str::trim).collect::<Vec<_>>().as_slice() {
            [hour, minute] => Ok((h(hour).into_validated() + m(minute))
                .into_result()?
                .from_generic()),
            _ => Err(ParseErrorTime::Fmt),
        }
    }
}

impl FromStr for TimeOffset {
    type Err = ParseErrorTime;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.parse::<Time>()?.into())
    }
}

fn time_range(
    start: &str,
    end: &str,
) -> Result<TimeRange, ParseErrorTimeRange> {
    Ok((start.parse().into_validated() + end.parse())
        .into_result()?
        .from_generic::<(TimeOffset, TimeOffset)>()
        .try_into()?)
}

impl FromStr for TimeRange {
    type Err = ParseErrorTimeRange;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match *s.split('-').collect::<Vec<_>>().as_slice() {
            [start, end] => time_range(start, end),
            [time] => {
                let t = time.parse::<TimeOffset>()?;
                Ok(Self { start: t, end: t })
            }
            _ => Err(ParseErrorTimeRange::Fmt),
        }
    }
}

impl FromStr for TimeRanges {
    type Err = ParseErrorTimeRanges;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match *s.split('-').collect::<Vec<_>>().as_slice() {
            [dawn, dusk] => Ok((dawn.parse().into_validated() + dusk.parse())
                .into_result()?
                .from_generic::<(TimeRange, TimeRange)>()
                .try_into()?),
            [dawn_start, dawn_end, dusk_start, dusk_end] => {
                Ok((time_range(dawn_start, dawn_end).into_validated()
                    + time_range(dusk_start, dusk_end))
                .into_result()?
                .from_generic::<(TimeRange, TimeRange)>()
                .try_into()?)
            }
            _ => Err(ParseErrorTimeRanges::Fmt),
        }
    }
}

impl FromStr for Elevation {
    type Err = ParseErrorElevation;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.trim().parse::<f64>()?.try_into()?)
    }
}

impl FromStr for ElevationRange {
    type Err = ParseErrorElevationRange;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match *s.split(':').collect::<Vec<_>>().as_slice() {
            [high, low] => Ok((high.parse().into_validated() + low.parse())
                .into_result()?
                .from_generic::<(Elevation, Elevation)>()
                .try_into()?),
            _ => Err(ParseErrorElevationRange::Fmt),
        }
    }
}

impl FromStr for TransitionScheme {
    type Err = TypeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Err(())
            .or_else(|_| Ok::<_, Self::Err>(Self::Time(s.parse()?)))
            .or_else(|_| Ok::<_, Self::Err>(Self::Elevation(s.parse()?)))
            .map_err(|_| anyhow!("asdf"))
    }
}

impl FromStr for LocationProviderType {
    type Err = TypeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "geoclue2" => Ok(Self::Geoclue2),
            _ => s.parse().map(Self::Manual),
        }
        .map_err(|_| anyhow!("asdf"))
    }
}

impl FromStr for AdjustmentMethodType {
    type Err = TypeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let num = |o: Option<&str>| match o {
            None => Ok::<_, TypeParseError>(None),
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

impl<T: Clone + FromStr<Err = TypeParseError>> FromStr for DayNight<T> {
    type Err = TypeParseError;

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
