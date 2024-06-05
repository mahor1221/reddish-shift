/*  gamma-drm.rs -- DRM gamma adjustment
    This file is part of <https://github.com/mahor1221/reddish-shift>.
    Copyright (C) 2024 Mahor Foruzesh <mahor1221@gmail.com>
    Ported from Redshift <https://github.com/jonls/redshift>.
    Copyright (c) 2014  Mattias Andrée <maandree@member.fsf.org>
    Copyright (c) 2017  Jon Lund Steffensen <jonlst@gmail.com>

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
    colorramp::{colorramp_fill, GammaRamps},
    config::ColorSettings,
    Adjuster,
};
use anyhow::{anyhow, Result};
use drm::{
    control::{crtc::Handle as CrtcHandle, Device as ControlDevice},
    Device,
};
use std::{
    fs::{File, OpenOptions},
    io::{ErrorKind as IoErrorKind, Result as IoResult},
    os::fd::{AsFd, BorrowedFd},
    path::Path,
};

const DRM_DIR: &str = "/dev/dri";

#[derive(Debug)]
struct Card(File);

#[derive(Debug)]
pub struct Drm {
    card: Card,
    crtcs: Vec<Crtc>,
}

#[derive(Debug)]
struct Crtc {
    handle: CrtcHandle,
    ramp_size: u32,
    saved_ramps: GammaRamps,
}

impl AsFd for Card {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

impl Device for Card {}
impl ControlDevice for Card {}

impl Card {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        fn inner(path: &Path) -> Result<Card> {
            let mut options = OpenOptions::new();
            options.read(true);
            options.write(true);
            Ok(Card(options.open(path)?))
            // fprintf(stderr, _("Failed to open DRM device: %s\n"),
        }
        inner(path.as_ref())
    }
}

impl Drm {
    pub fn new(card_num: Option<usize>, crtc_ids: Vec<u32>) -> Result<Self> {
        let card_num = card_num.unwrap_or_default();
        let path = DRM_DIR.to_string() + "/card" + &card_num.to_string();
        let card = Card::open(path)?;

        let crtcs = if crtc_ids.is_empty() {
            card.resource_handles()?.crtcs
        } else {
            crtc_ids
                .into_iter()
                .map(drm::control::from_u32)
                .collect::<Option<Vec<_>>>()
                .ok_or(anyhow!("must be non zero positive"))?
        };

        // let res = card
        //     .resource_handles()
        //     .expect("Could not load normal resource ids.");
        // let coninfo: Vec<_> = res
        //     .connectors()
        //     .iter()
        //     .flat_map(|con| card.get_connector(*con, true))
        //     .collect();
        // let crtcinfo: Vec<_> = res
        //     .crtcs()
        //     .iter()
        //     .flat_map(|crtc| card.get_crtc(*crtc))
        //     .collect();

        // let con = coninfo
        //     .iter()
        //     .find(|&i| i.state() == drm::control::connector::State::Connected)
        //     .expect("No connected connectors");

        // let &mode = con.modes().first().expect("No modes found on connector");

        // dbg!(mode);

        // TODO: accumulate errors
        let crtcs = crtcs
            .into_iter()
            .map(|handle| {
                let info = match card.get_crtc(handle) {
                    Ok(i) => Ok(i),
                    Err(e) if e.kind() == IoErrorKind::NotFound => {
                        let crtcs = card
                            .resource_handles()?
                            .crtcs
                            .into_iter()
                            .map(u32::from)
                            .collect::<Vec<_>>();
                        Err(anyhow!("Valid CRTCs are {crtcs:?}"))
                    }
                    Err(e) => Err(anyhow::Error::new(e)),
                }?;

                // fprintf(stderr, _("CRTC %i lost, skipping\n"), crtcs->crtc_num);
                let ramp_size = info.gamma_length();
                if ramp_size <= 1 {
                    Err(anyhow!("gamma_length"))?
                    // "Could not get gamma ramp size for CRTC %i\non graphics card %i, ignoring device.\n"
                }

                let saved_ramps = {
                    let (mut r, mut b, mut g) =
                        (Vec::new(), Vec::new(), Vec::new());
                    card.get_gamma(handle, &mut r, &mut b, &mut g)?;
                    // _("DRM could not read gamma ramps on CRTC %i on\n"
                    // "graphics card %i, ignoring device.\n"),
                    GammaRamps([r, g, b])
                };

                Ok(Crtc {
                    handle,
                    ramp_size,
                    saved_ramps,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Self { card, crtcs })
    }

    fn set_gamma_ramps(&self, f: impl Fn(&Crtc) -> IoResult<()>) -> Result<()> {
        // TODO: accumulate errors
        self.crtcs.iter().map(f).collect::<IoResult<Vec<_>>>()?;
        Ok(())
    }
}

impl Adjuster for Drm {
    fn restore(&self) -> Result<()> {
        self.set_gamma_ramps(|crtc| {
            self.card.set_gamma(
                crtc.handle,
                &crtc.saved_ramps[0],
                &crtc.saved_ramps[1],
                &crtc.saved_ramps[2],
            )
        })
    }

    fn set_color(&self, cs: &ColorSettings, reset_ramps: bool) -> Result<()> {
        self.set_gamma_ramps(|crtc| {
            let mut ramps = if reset_ramps {
                GammaRamps::new(crtc.ramp_size)
            } else {
                crtc.saved_ramps.clone()
            };

            colorramp_fill(cs, &mut ramps);
            self.card
                .set_gamma(crtc.handle, &ramps[0], &ramps[1], &ramps[2])
        })
    }
}
