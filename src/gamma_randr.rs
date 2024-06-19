/*  gamma-randr.rs -- X RANDR gamma adjustment
    This file is part of <https://github.com/mahor1221/reddish-shift>.
    Copyright (C) 2024 Mahor Foruzesh <mahor1221@gmail.com>
    Ported from Redshift <https://github.com/jonls/redshift>.
    Copyright (c) 2010-2017  Jon Lund Steffensen <jonlst@gmail.com>

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
    calc_colorramp::GammaRamps,
    config::{RANDR_MAJOR_VERSION, RANDR_MINOR_VERSION_MIN},
    error::{
        gamma::{RandrError, RandrErrorCrtc},
        AdjusterError, AdjusterErrorInner,
    },
    types::ColorSettings,
    utils::{CollectResult, InjectMapErr},
    Adjuster,
};
use x11rb::{
    connection::Connection as _,
    cookie::{Cookie, VoidCookie},
    errors::ConnectionError,
    protocol::randr::{
        ConnectionExt, GetCrtcGammaReply, GetCrtcGammaSizeReply,
    },
    rust_connection::RustConnection as Conn,
};

#[derive(Debug)]
pub struct Randr {
    conn: Conn,
    crtcs: Vec<Crtc>,
}

#[derive(Debug)]
struct Crtc {
    id: u32,
    ramp_size: u16,
    saved_ramps: GammaRamps,
}

impl Randr {
    pub fn new(
        screen_num: Option<usize>,
        crtc_ids: Vec<u32>,
    ) -> Result<Self, RandrError> {
        // uses the DISPLAY environment variable if screen_num is None
        let screen_num = screen_num.map(|n| ":".to_string() + &n.to_string());
        let (conn, screen_num) = x11rb::connect(screen_num.as_deref())
            .map_err(RandrError::ConnectFailed)?;

        // returns a lower version if 1.3 is not supported
        let r = conn
            .randr_query_version(1, 3)
            .inject_map_err(RandrError::GetVersionFailed)?
            .reply()
            .inject_map_err(RandrError::GetVersionFailed)?;

        // eprintln!("`{}` returned error {}", "RANDR Query Version", ec);
        if r.major_version != RANDR_MAJOR_VERSION
            || r.minor_version < RANDR_MINOR_VERSION_MIN
        {
            Err(RandrError::UnsupportedVersion {
                major: r.major_version,
                minor: r.minor_version,
            })?
        }

        let crtcs = Self::get_crtcs(&conn, screen_num, crtc_ids)?;

        Ok(Self { conn, crtcs })
    }

    fn get_crtcs(
        conn: &Conn,
        screen_num: usize,
        mut crtc_ids: Vec<u32>,
    ) -> Result<Vec<Crtc>, RandrError> {
        let win = conn.setup().roots[screen_num].root;
        let all_crtcs = conn
            .randr_get_screen_resources_current(win)
            .inject_map_err(RandrError::GetResourcesFailed)?
            .reply()
            .inject_map_err(RandrError::GetResourcesFailed)?
            .crtcs;

        let crtcs = if crtc_ids.is_empty() {
            all_crtcs
        } else {
            let len = crtc_ids.len();
            crtc_ids.sort();
            crtc_ids.dedup();
            if len > crtc_ids.len() {
                Err(RandrError::NonUniqueCrtc)?
            }
            let f = |&h| Self::validate_crtc(&all_crtcs, h);
            crtc_ids.iter().try_for_each(f)?;
            crtc_ids
        };

        crtcs
            .into_iter()
            .map(|id| {
                let c_ramp = conn.randr_get_crtc_gamma(id)?;
                let c_size = conn.randr_get_crtc_gamma_size(id)?;
                Ok((id, c_size, c_ramp))
            })
            // collect to send all of the requests
            .collect_result()
            .map_err(RandrError::SendRequestFailed)?
            .into_iter()
            .map(Self::get_crtc)
            .collect_result()
            .map_err(RandrError::Crtcs)
    }

    fn validate_crtc(all_crtcs: &[u32], id: u32) -> Result<(), RandrError> {
        if all_crtcs.iter().any(|&i| id == i) {
            Ok(())
        } else {
            Err(RandrError::InvalidCrtc(all_crtcs.to_vec()))
        }
    }

    fn get_crtc(
        (id, c_size, c_ramp): (
            u32,
            Cookie<Conn, GetCrtcGammaSizeReply>,
            Cookie<Conn, GetCrtcGammaReply>,
        ),
    ) -> Result<Crtc, RandrErrorCrtc> {
        let r = c_ramp.reply().map_err(RandrErrorCrtc::GetRampFailed)?;
        let saved_ramps = GammaRamps([r.red, r.green, r.blue]);
        let ramp_size = c_size
            .reply()
            .map_err(RandrErrorCrtc::GetRampSizeFailed)?
            .size;
        if ramp_size == 0 {
            Err(RandrErrorCrtc::InvalidRampSize(ramp_size))?
        }

        Ok(Crtc {
            id,
            ramp_size,
            saved_ramps,
        })
    }

    fn set_gamma_ramps<'s>(
        &'s self,
        f: impl Fn(&Crtc) -> Result<VoidCookie<'s, Conn>, ConnectionError>,
    ) -> Result<(), AdjusterErrorInner> {
        // TODO: accumulate errors
        self.crtcs
            .iter()
            .map(f)
            // collect to send all of the requests
            .collect_result()
            .inject_map_err(AdjusterErrorInner::Randr)?
            .into_iter()
            .map(|c| c.check())
            .collect_result()
            .inject_map_err(AdjusterErrorInner::Randr)?;
        Ok(())
    }
}

impl Adjuster for Randr {
    fn restore(&self) -> Result<(), AdjusterError> {
        self.set_gamma_ramps(|crtc| {
            Ok(self.conn.randr_set_crtc_gamma(
                crtc.id,
                &crtc.saved_ramps[0],
                &crtc.saved_ramps[1],
                &crtc.saved_ramps[2],
            )?)
        })
        .map_err(AdjusterError::Restore)
    }

    fn set(
        &self,
        reset_ramps: bool,
        cs: &ColorSettings,
    ) -> Result<(), AdjusterError> {
        self.set_gamma_ramps(|crtc| {
            let mut ramps = if reset_ramps {
                GammaRamps::new(crtc.ramp_size as u32)
            } else {
                crtc.saved_ramps.clone()
            };

            ramps.colorramp_fill(cs);
            Ok(self.conn.randr_set_crtc_gamma(
                crtc.id, &ramps[0], &ramps[1], &ramps[2],
            )?)
        })
        .map_err(AdjusterError::Restore)
    }
}
