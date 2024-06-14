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

use crate::{colorramp::GammaRamps, config::ColorSettings, Adjuster};
use anyhow::{anyhow, Result};
use x11rb::{
    connection::Connection as _,
    cookie::{Cookie, VoidCookie},
    errors::ReplyError,
    protocol::{
        randr::{ConnectionExt, GetCrtcGammaReply, GetCrtcGammaSizeReply},
        ErrorKind as X11ErrorKind,
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
    pub fn new(screen_num: Option<usize>, crtc_ids: Vec<u32>) -> Result<Self> {
        // uses the DISPLAY environment variable if screen_num is None
        let screen_num = screen_num.map(|n| ":".to_string() + &n.to_string());
        let (conn, screen_num) = x11rb::connect(screen_num.as_deref())?;

        // returns a lower version if 1.3 is not supported
        let r = conn.randr_query_version(1, 3)?.reply()?;
        // eprintln!("`{}` returned error {}", "RANDR Query Version", ec);
        if r.major_version != 1 || r.minor_version < 3 {
            let major = r.major_version;
            let minor = r.minor_version;
            Err(anyhow!("Unsupported RANDR version ({major}.{minor})"))?
        }

        let crtcs = Self::get_crtcs(&conn, screen_num, crtc_ids)?;

        Ok(Self { conn, crtcs })
    }

    fn get_crtcs(
        conn: &Conn,
        screen_num: usize,
        crtc_ids: Vec<u32>,
    ) -> Result<Vec<Crtc>> {
        let crtcs = if crtc_ids.is_empty() {
            let win = conn.setup().roots[screen_num].root;
            conn.randr_get_screen_resources_current(win)?.reply()?.crtcs
        } else {
            crtc_ids
        };

        // TODO: accumulate errors
        crtcs
            .into_iter()
            .map(|id| {
                let c_size = conn.randr_get_crtc_gamma_size(id)?;
                let c_ramp = conn.randr_get_crtc_gamma(id)?;
                Ok((id, c_size, c_ramp))
            })
            // collect to send all of the requests
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .map(|t| Self::get_crtc_from_cookies(conn, screen_num, t))
            .collect::<Result<Vec<_>>>()
    }

    fn get_crtc_from_cookies(
        conn: &Conn,
        screen_num: usize,
        (id, c_size, c_ramp): (
            u32,
            Cookie<Conn, GetCrtcGammaSizeReply>,
            Cookie<Conn, GetCrtcGammaReply>,
        ),
    ) -> Result<Crtc> {
        let r = match c_ramp.reply() {
            Ok(r) => Ok(r),
            Err(ReplyError::X11Error(e))
                if e.error_kind == X11ErrorKind::RandrBadCrtc =>
            {
                let win = conn.setup().roots[screen_num].root;
                let crtcs = conn
                    .randr_get_screen_resources_current(win)?
                    .reply()?
                    .crtcs;
                Err(anyhow!("Valid CRTCs are {crtcs:?}"))
            }
            Err(e) => Err(anyhow::Error::new(e)),
        }?;

        let saved_ramps = GammaRamps([r.red, r.green, r.blue]);
        let ramp_size = c_size.reply()?.size;
        if ramp_size == 0 {
            Err(anyhow!("Gamma ramp size too small: {ramp_size}"))?
        }

        Ok(Crtc {
            id,
            ramp_size,
            saved_ramps,
        })
    }

    fn set_gamma_ramps<'s>(
        &'s self,
        f: impl Fn(&Crtc) -> Result<VoidCookie<'s, Conn>>,
    ) -> Result<()> {
        // TODO: accumulate errors
        self.crtcs
            .iter()
            .map(f)
            // collect to send all of the requests
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            // fprintf(stderr, _("`%s' returned error %d\n"), "RANDR Set CRTC Gamma",
            .map(|c| Ok(c.check()?))
            .collect::<Result<Vec<()>>>()?;
        // fprintf(stderr, _("Unable to restore CRTC %i\n"), i);
        Ok(())
    }
}

impl Adjuster for Randr {
    fn restore(&self) -> Result<()> {
        self.set_gamma_ramps(|crtc| {
            Ok(self.conn.randr_set_crtc_gamma(
                crtc.id,
                &crtc.saved_ramps[0],
                &crtc.saved_ramps[1],
                &crtc.saved_ramps[2],
            )?)
        })
    }

    fn set(&self, reset_ramps: bool, cs: &ColorSettings) -> Result<()> {
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
    }
}
