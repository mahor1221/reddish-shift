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

use crate::{
    coproduct::InjectErr,
    error::{gamma::CrtcError, parse::*},
    types::{
        gamma, hour, minute, AdjustmentMethodType, Brightness, DayNight,
        Elevation, ElevationRange, Gamma, Latitude, Location,
        LocationProviderType, Longitude, Temperature, Time, TimeOffset,
        TimeRange, TimeRanges, TransitionScheme,
    },
    utils::{CollectResult, IntoGeneric},
};
use frunk::validated::IntoValidated;
use std::str::FromStr;

impl FromStr for Temperature {
    type Err = TemperatureError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.trim()
            .parse::<u16>()
            .map_err(|e| TemperatureError::Parse(e, s.into()))?
            .try_into()?)
    }
}

impl FromStr for Brightness {
    type Err = BrightnessError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.trim()
            .parse::<f64>()
            .map_err(|e| BrightnessError::Parse(e, s.into()))?
            .try_into()?)
    }
}

impl FromStr for Gamma {
    type Err = GammaError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let f = |s: &str| -> Result<f64, GammaErrorT> {
            gamma(s.parse::<f64>().inject_err()?).inject_err()
        };

        match *s.split(':').map(str::trim).collect::<Vec<_>>().as_slice() {
            [r, g, b] => Ok((f(r).into_validated() + f(g) + f(b))
                .into_result()?
                .into_generic::<(f64, f64, f64)>()
                .try_into()
                .unwrap_or_else(|_| unreachable!())),
            [rbg] => Ok(f(rbg)?.try_into().unwrap_or_else(|_| unreachable!())),
            _ => Err(GammaError::Fmt),
        }
    }
}

impl FromStr for Latitude {
    type Err = LatitudeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.trim()
            .parse::<f64>()
            .map_err(|e| LatitudeError::Parse(e, s.into()))?
            .try_into()?)
    }
}

impl FromStr for Longitude {
    type Err = LongitudeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.trim()
            .parse::<f64>()
            .map_err(|e| LongitudeError::Parse(e, s.into()))?
            .try_into()?)
    }
}

impl FromStr for Location {
    type Err = LocationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match *s.split(':').collect::<Vec<_>>().as_slice() {
            [lat, lon] => Ok((lat.parse().inject_err().into_validated()
                + lon.parse().inject_err())
            .into_result()?
            .into_generic()),
            _ => Err(LocationError::Fmt),
        }
    }
}

impl FromStr for Time {
    type Err = TimeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let h = |s: &str| -> Result<u8, TimeErrorT> {
            hour(s.parse::<u8>().inject_err()?).inject_err()
        };
        let m = |s: &str| -> Result<u8, TimeErrorT> {
            minute(s.parse::<u8>().inject_err()?).inject_err()
        };
        match *s.split(':').map(str::trim).collect::<Vec<_>>().as_slice() {
            [hour, minute] => Ok((h(hour).into_validated() + m(minute))
                .into_result()?
                .into_generic()),
            _ => Err(TimeError::Fmt),
        }
    }
}

impl FromStr for TimeOffset {
    type Err = TimeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.parse::<Time>()?.into())
    }
}

fn time_range(start: &str, end: &str) -> Result<TimeRange, TimeRangeError> {
    Ok((start.parse().into_validated() + end.parse())
        .into_result()?
        .into_generic::<(TimeOffset, TimeOffset)>()
        .try_into()?)
}

impl FromStr for TimeRange {
    type Err = TimeRangeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match *s.split('-').collect::<Vec<_>>().as_slice() {
            [start, end] => time_range(start, end),
            [time] => {
                let t = time.parse::<TimeOffset>()?;
                Ok(Self { start: t, end: t })
            }
            _ => Err(TimeRangeError::Fmt),
        }
    }
}

impl FromStr for TimeRanges {
    type Err = TimeRangesError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match *s.split('-').collect::<Vec<_>>().as_slice() {
            [dawn, dusk] => Ok((dawn.parse().into_validated() + dusk.parse())
                .into_result()?
                .into_generic::<(TimeRange, TimeRange)>()
                .try_into()?),
            [dawn_start, dawn_end, dusk_start, dusk_end] => {
                Ok((time_range(dawn_start, dawn_end).into_validated()
                    + time_range(dusk_start, dusk_end))
                .into_result()?
                .into_generic::<(TimeRange, TimeRange)>()
                .try_into()?)
            }
            _ => Err(TimeRangesError::Fmt),
        }
    }
}

impl FromStr for Elevation {
    type Err = ElevationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.trim()
            .parse::<f64>()
            .map_err(|e| ElevationError::Parse(e, s.into()))?
            .try_into()?)
    }
}

impl FromStr for ElevationRange {
    type Err = ElevationRangeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match *s.split(':').collect::<Vec<_>>().as_slice() {
            [high, low] => Ok((high.parse().into_validated() + low.parse())
                .into_result()?
                .into_generic::<(Elevation, Elevation)>()
                .try_into()?),
            _ => Err(ElevationRangeError::Fmt),
        }
    }
}

impl FromStr for TransitionScheme {
    type Err = TransitionSchemeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Err(())
            .or_else(|_| Ok(Self::Time(s.parse()?)))
            .or_else(|e1| Ok(Self::Elev(s.parse().map_err(|e2| (e1, e2))?)))
            .map_err(|(time, elev)| TransitionSchemeError { time, elev })
    }
}

impl FromStr for LocationProviderType {
    type Err = LocationProviderError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "geoclue2" => Ok(Self::Geoclue2),
            _ => s
                .parse()
                .map(Self::Manual)
                .map_err(|loc| LocationProviderError { loc }),
        }
    }
}

impl FromStr for AdjustmentMethodType {
    type Err = AdjustmentMethodTypeError;

    #[allow(clippy::too_many_lines)]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let kind = |s: &str| match s {
            "dummy" => Ok(Self::Dummy),
            "drm" => Ok(Self::Drm {
                card_num: None,
                crtcs: vec![],
            }),
            "vidmode" => Ok(Self::Vidmode { screen_num: None }),
            "randr" => Ok(Self::Randr {
                screen_num: None,
                crtcs: vec![],
            }),
            _ => Err(AdjustmentMethodTypeParamError::InvalidName(s.into())),
        };

        let num = |o: Option<&str>| match o {
            None => Ok(None),
            Some(s) => Ok(Some(s.parse().map_err(|e| {
                AdjustmentMethodTypeParamError::Display(e, s.into())
            })?)),
        };
        let crtcs = |o: Option<&str>| match o {
            None => Ok(Vec::new()),
            Some(s) => Ok(s
                .split(',')
                .map(|id| {
                    id.trim().parse().map_err(|err| CrtcError {
                        id: id.to_string(),
                        err,
                    })
                })
                .collect_result()?),
        };

        let f = |k: &str, n: Option<&str>, c: Option<&str>| {
            let (mut k, n, c) = (kind(k).into_validated() + num(n) + crtcs(c))
                .into_result()?
                .into_generic::<(_, _, _)>();
            match &mut k {
                AdjustmentMethodType::Dummy => {}
                AdjustmentMethodType::Drm { card_num, crtcs } => {
                    *card_num = n;
                    *crtcs = c;
                }
                AdjustmentMethodType::Randr { screen_num, crtcs } => {
                    *screen_num = n;
                    *crtcs = c;
                }
                AdjustmentMethodType::Vidmode { screen_num } => {
                    *screen_num = n;
                    if !c.is_empty() {
                        Err(AdjustmentMethodTypeError::CrtcOnVidmode)?
                    }
                }
            };
            Ok(k)
        };

        match s.split(':').map(str::trim).collect::<Vec<_>>().as_slice() {
            [k] => f(k, None, None),
            [k, n] => f(k, Some(n), None),
            [k, n, c] => f(k, Some(n), Some(c)),
            _ => Err(AdjustmentMethodTypeError::Fmt),
        }
    }
}

impl<E, T> FromStr for DayNight<T>
where
    E: DayNightErrorType,
    T: Clone + FromStr<Err = E>,
{
    type Err = DayNightError<E>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match *s.split('-').collect::<Vec<_>>().as_slice() {
            [day_night] => {
                let day_night = day_night.parse::<T>()?;
                Ok(Self {
                    day: day_night.clone(),
                    night: day_night,
                })
            }
            [day, night] => Ok((day.parse().into_validated() + night.parse())
                .into_result()?
                .into_generic()),
            _ => Err(DayNightError::Fmt),
        }
    }
}
