/*  gamma-vidmode.rs -- X VidMode gamma adjustment
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

use crate::{calc_colorramp::GammaRamps, types::ColorSettings, Adjuster};
use anyhow::{anyhow, Result};
use x11rb::{
    protocol::xf86vidmode::ConnectionExt,
    rust_connection::RustConnection as X11Connection,
};

#[derive(Debug)]
pub struct Vidmode {
    conn: X11Connection,
    screen_num: u16,
    ramp_size: u16,
    saved_ramps: GammaRamps,
}

impl Vidmode {
    pub fn new(screen_num: Option<usize>) -> Result<Self> {
        // it uses the DISPLAY environment variable if screen_num is None
        let screen_num = screen_num.map(|n| ":".to_string() + &n.to_string());
        let (conn, screen_num) = x11rb::connect(screen_num.as_deref())?;
        let screen_num = screen_num as u16;

        // check connection
        conn.xf86vidmode_query_version()?.reply()?;
        // eprintln!("X request failed: XF86VidModeQueryVersion");

        let ramp_size = conn
            .xf86vidmode_get_gamma_ramp_size(screen_num)?
            .reply()?
            .size;
        // eprintln!("X request failed: XF86VidModeGetGammaRampSize");
        if ramp_size == 0 {
            Err(anyhow!("Gamma ramp size too small: {ramp_size}"))?
        }

        let ramp = conn
            .xf86vidmode_get_gamma_ramp(screen_num, ramp_size)?
            .reply()?;
        // eprintln!("X request failed: XF86VidModeGetGammaRamp");
        let saved_ramps = GammaRamps([ramp.red, ramp.green, ramp.blue]);

        Ok(Self {
            conn,
            screen_num,
            ramp_size,
            saved_ramps,
        })
    }

    fn set_gamma_ramps(&self, ramps: &GammaRamps) -> Result<()> {
        self.conn
            .xf86vidmode_set_gamma_ramp(
                self.screen_num,
                self.ramp_size,
                &ramps[0],
                &ramps[1],
                &ramps[2],
            )?
            .check()?;
        // eprintln!("X request failed: XF86VidModeSetGammaRamp")
        Ok(())
    }
}

impl Adjuster for Vidmode {
    fn restore(&self) -> Result<()> {
        self.set_gamma_ramps(&self.saved_ramps)
    }

    fn set(&self, reset_ramps: bool, cs: &ColorSettings) -> Result<()> {
        let mut ramps = if reset_ramps {
            GammaRamps::new(self.ramp_size as u32)
        } else {
            self.saved_ramps.clone()
        };

        ramps.colorramp_fill(cs);
        self.set_gamma_ramps(&ramps)
    }
}
