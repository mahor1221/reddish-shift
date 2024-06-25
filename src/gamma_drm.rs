/*  gamma-drm.rs -- Direct Rendering Manager gamma adjustment
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
    calc_colorramp::GammaRamps,
    error::{
        gamma::{CrtcError, DrmCrtcError, DrmError},
        AdjusterError, AdjusterErrorInner,
    },
    types::ColorSettings,
    utils::CollectResult,
    Adjuster,
};
use drm::{
    control::{
        crtc::Handle as CrtcHandle, from_u32 as handle_from_u32,
        Device as ControlDevice,
    },
    Device,
};
use std::{
    fs::{File, OpenOptions},
    io,
    os::fd::{AsFd, BorrowedFd},
    path::Path,
};

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
    pub fn open(path: impl AsRef<Path>) -> Result<Self, DrmError> {
        fn inner(path: &Path) -> Result<Card, DrmError> {
            let mut options = OpenOptions::new();
            options.read(true);
            options.write(true);
            Ok(Card(
                options
                    .open(path)
                    .map_err(|e| DrmError::OpenDeviceFailed(e, path.into()))?,
            ))
        }
        inner(path.as_ref())
    }
}

impl Drm {
    pub fn new(
        card_num: Option<usize>,
        crtc_ids: Vec<u32>,
    ) -> Result<Self, DrmError> {
        let path = format!("/dev/dri/card{}", card_num.unwrap_or_default());
        let card = Card::open(path)?;
        let crtcs = Self::get_crtcs(&card, crtc_ids)?;
        Ok(Self { card, crtcs })
    }

    fn get_crtcs(
        card: &Card,
        mut crtc_ids: Vec<u32>,
    ) -> Result<Vec<Crtc>, DrmError> {
        let all_crtcs = card
            .resource_handles()
            .map_err(DrmError::GetResourcesFailed)?
            .crtcs;

        let crtcs = if crtc_ids.is_empty() {
            all_crtcs
        } else {
            let len = crtc_ids.len();
            crtc_ids.sort();
            crtc_ids.dedup();
            if len > crtc_ids.len() {
                Err(DrmError::NonUniqueCrtc)?
            }
            let f = |h| Self::validate_crtc(&all_crtcs, h);
            crtc_ids.into_iter().map(f).collect::<Result<Vec<_>, _>>()?
        };

        crtcs
            .into_iter()
            .map(|h| Self::get_crtc(card, h))
            .collect_result()
            .map_err(DrmError::GetCrtcs)
    }

    fn validate_crtc(
        all_crtcs: &[CrtcHandle],
        id: u32,
    ) -> Result<CrtcHandle, DrmError> {
        let crtcs =
            || all_crtcs.iter().map(|&h| h.into()).collect::<Vec<u32>>();
        let handle: CrtcHandle =
            handle_from_u32(id).ok_or(DrmError::ZeroValueCrtc)?;
        if all_crtcs.iter().any(|&h| handle == h) {
            Ok(handle)
        } else {
            Err(DrmError::InvalidCrtc(crtcs()))
        }
    }

    fn get_crtc(
        card: &Card,
        handle: CrtcHandle,
    ) -> Result<Crtc, CrtcError<u32, DrmCrtcError>> {
        let f = || -> Result<Crtc, DrmCrtcError> {
            let info = card
                .get_crtc(handle)
                .map_err(DrmCrtcError::GetRampSizeFailed)?;
            let ramp_size = info.gamma_length();
            if ramp_size <= 1 {
                Err(DrmCrtcError::InvalidRampSize(ramp_size))?
            }

            let (mut r, mut g, mut b) = (Vec::new(), Vec::new(), Vec::new());
            // FIX: Error: Bad address (os error 14)
            // drm_ffi::mode::get_gamma(
            //     card.as_fd(),
            //     handle.into(),
            //     ramp_size as usize,
            //     &mut r,
            //     &mut g,
            //     &mut b,
            // )?;
            //
            // The C function drmModeCrtcGetGamma works on my system
            // Test here: https://github.com/mahor1221/redshift
            // build and run: ./redshift -m drm:card=<your card> -x
            //
            // everything is similar to the C function, why it doesn't work
            // https://gitlab.freedesktop.org/mesa/drm/-/blob/main/xf86drmMode.c#L1000
            // https://gitlab.freedesktop.org/mesa/drm/-/blob/main/include/drm/drm.h#L1155
            //
            // FIX: Error: Invalid argument (os error 22)
            card.get_gamma(handle, &mut r, &mut g, &mut b)
                .map_err(DrmCrtcError::GetRampFailed)?;
            let saved_ramps = GammaRamps([r, g, b]);
            // _("DRM could not read gamma ramps on CRTC %i on\n"
            // "graphics card %i, ignoring device.\n"),

            Ok(Crtc {
                handle,
                ramp_size,
                saved_ramps,
            })
        };

        f().map_err(|err| CrtcError {
            id: handle.into(),
            err,
        })
    }

    fn set_gamma_ramps(
        &self,
        f: impl Fn(&Crtc) -> io::Result<()>,
    ) -> Result<(), AdjusterErrorInner> {
        self.crtcs.iter().map(f).collect_result()?;
        Ok(())
    }
}

impl Adjuster for Drm {
    fn restore(&self) -> Result<(), AdjusterError> {
        self.set_gamma_ramps(|crtc| {
            self.card.set_gamma(
                crtc.handle,
                &crtc.saved_ramps[0],
                &crtc.saved_ramps[1],
                &crtc.saved_ramps[2],
            )
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
                GammaRamps::new(crtc.ramp_size)
            } else {
                crtc.saved_ramps.clone()
            };

            ramps.colorramp_fill(cs);
            self.card
                .set_gamma(crtc.handle, &ramps[0], &ramps[1], &ramps[2])
        })
        .map_err(AdjusterError::Set)
    }
}
