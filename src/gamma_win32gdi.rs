/*  gamma-vidmode.rs -- Windows GDI gamma adjustment
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

#![allow(unsafe_code)]
#![allow(clippy::undocumented_unsafe_blocks)]
use crate::{
    calc_colorramp::GammaRampsWin32,
    error::{gamma::Win32GdiError, AdjusterError, AdjusterErrorInner},
    types::ColorSettings,
    Adjuster,
};
use std::ffi::c_void;
use windows::Win32::{
    Foundation::HWND,
    Graphics::Gdi::{GetDC, GetDeviceCaps, ReleaseDC, COLORMGMTCAPS},
    UI::ColorSystem::{GetDeviceGammaRamp, SetDeviceGammaRamp},
};

const MAX_ATTEMPTS: u8 = 10;
const GAMMA_RAMP_SIZE: usize = 256;

// https://learn.microsoft.com/en-us/windows/win32/winprog/windows-data-types
type Word = u16;
// https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getdc
const HWND_NULL: HWND = HWND(0);

#[derive(Debug)]
pub struct Win32Gdi {
    saved_ramps: GammaRampsWin32<GAMMA_RAMP_SIZE>,
}

impl Win32Gdi {
    pub fn new() -> Result<Self, Win32GdiError> {
        unsafe {
            // Open device context
            let hdc = GetDC(HWND_NULL);
            if hdc.is_invalid() {
                Err(Win32GdiError::GetDCFailed)?;
            }

            // Check support for gamma ramps
            let cmcap = GetDeviceCaps(hdc, COLORMGMTCAPS);
            if cmcap as i64 != COLORMGMTCAPS.0 as i64 {
                ReleaseDC(HWND_NULL, hdc);
                Err(Win32GdiError::NotSupported)?;
            }

            // Save current gamma ramps so we can restore them at program exit
            let saved_ramps = {
                // https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-getdevicegammaramp
                // saved_ramps = malloc(3*GAMMA_RAMP_SIZE*sizeof(WORD));
                let mut saved_ramps: [[Word; GAMMA_RAMP_SIZE]; 3] =
                    [[0; GAMMA_RAMP_SIZE]; 3];

                let ptr = saved_ramps.as_mut_ptr() as *mut c_void;
                if GetDeviceGammaRamp(hdc, ptr).0 == 0 {
                    ReleaseDC(HWND_NULL, hdc);
                    Err(Win32GdiError::GetRampFailed)?;
                }
                GammaRampsWin32(Box::new(saved_ramps))
            };

            ReleaseDC(HWND_NULL, hdc);
            Ok(Self { saved_ramps })
        }
    }

    fn set_gamma_ramps(
        &self,
        ramps: &GammaRampsWin32<GAMMA_RAMP_SIZE>,
    ) -> Result<(), AdjusterErrorInner> {
        unsafe {
            // Open device context
            let hdc = GetDC(HWND_NULL);
            if hdc.is_invalid() {
                Err(Win32GdiError::GetDCFailed)?;
            }

            // We retry a few times before giving up because some buggy drivers
            // fail on the first invocation of SetDeviceGammaRamp just to
            // succeed on the second
            let mut i = 0;
            let mut err = true;
            while i < MAX_ATTEMPTS && err {
                i += 1;
                err = {
                    let ptr = ramps.0.as_ptr() as *const c_void;
                    SetDeviceGammaRamp(hdc, ptr).0 == 0
                };
            }
            if err {
                Err(Win32GdiError::SetRampFailed)?
            }

            ReleaseDC(HWND_NULL, hdc);
            Ok(())
        }
    }
}

impl Adjuster for Win32Gdi {
    fn restore(&self) -> Result<(), AdjusterError> {
        self.set_gamma_ramps(&self.saved_ramps)
            .map_err(AdjusterError::Restore)
    }

    fn set(
        &self,
        reset_ramps: bool,
        cs: &ColorSettings,
    ) -> Result<(), AdjusterError> {
        let mut ramps = if reset_ramps {
            GammaRampsWin32::new()
        } else {
            self.saved_ramps.clone()
        };

        ramps.colorramp_fill(cs);
        self.set_gamma_ramps(&ramps).map_err(AdjusterError::Set)
    }
}
