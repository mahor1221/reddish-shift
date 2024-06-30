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
use crate::{
    calc_colorramp::GammaRampsWin32,
    error::{gamma::Win32GdiError, AdjusterError, AdjusterErrorInner},
    types::ColorSettings,
    utils::InjectMapErr,
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
const HWND_0: HWND = HWND(0);

#[derive(Debug)]
pub struct Win32Gdi {
    saved_ramps: GammaRampsWin32<GAMMA_RAMP_SIZE>,
}

impl Win32Gdi {
    pub fn new() -> Result<Self, Win32GdiError> {
        // Open device context
        let hdc = unsafe {
            let hdc = GetDC(HWND_0);
            if hdc.is_invalid() {
                Err(Win32GdiError::GetDCFailed)?;
            }
            hdc
        };

        // Check support for gamma ramps
        unsafe {
            let cmcap = GetDeviceCaps(hdc, COLORMGMTCAPS);
            if cmcap as i64 != COLORMGMTCAPS.0 as i64 {
                ReleaseDC(HWND_0, hdc);
                Err(Win32GdiError::NotSupported)?;
            }
        }

        // Save current gamma ramps so we can restore them at program exit
        let saved_ramps = unsafe {
            // https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-getdevicegammaramp
            // saved_ramps = malloc(3*GAMMA_RAMP_SIZE*sizeof(WORD));
            let mut saved_ramps: [[Word; GAMMA_RAMP_SIZE]; 3] =
                [[0; GAMMA_RAMP_SIZE]; 3];

            let ptr = saved_ramps.as_mut_ptr() as *mut c_void;
            // 0 means failure
            if GetDeviceGammaRamp(hdc, ptr).0 == 0 {
                ReleaseDC(HWND_0, hdc);
                Err(Win32GdiError::GetRampFailed)?;
            }
            GammaRampsWin32(saved_ramps)
        };

        unsafe { ReleaseDC(HWND_0, hdc) };
        Ok(Self { saved_ramps })
    }

    fn set_gamma_ramps(
        &self,
        ramps: &GammaRampsWin32<GAMMA_RAMP_SIZE>,
    ) -> Result<(), AdjusterErrorInner> {
        // Open device context
        let hdc = unsafe {
            let hdc = GetDC(HWND_0);
            if hdc.is_invalid() {
                Err(Win32GdiError::GetDCFailed)?;
            }
            hdc
        };

        // We retry a few times before giving up because some buggy drivers
        // fail on the first invocation of SetDeviceGammaRamp just to succeed
        // on the second
        let mut i = 0;
        let mut r = true;
        while i < MAX_ATTEMPTS && r {
            i += 1;
            r = unsafe {
                let ptr = ramps.0.as_ptr() as *const c_void;
                SetDeviceGammaRamp(hdc, ptr).0 == 0 // 0 means failure
            };
        }

        unsafe { ReleaseDC(HWND_0, hdc) };
        Ok(())
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

// static int
// w32gdi_set_temperature(
// 	w32gdi_state_t *state, const color_setting_t *setting, int preserve)
// {
// 	BOOL r;

// 	/* Open device context */
// 	HDC hDC = GetDC(NULL);
// 	if (hDC == NULL) {
// 		fputs(_("Unable to open device context.\n"), stderr);
// 		return -1;
// 	}

// 	/* Create new gamma ramps */
// 	WORD *gamma_ramps = malloc(3*GAMMA_RAMP_SIZE*sizeof(WORD));
// 	if (gamma_ramps == NULL) {
// 		perror("malloc");
// 		ReleaseDC(NULL, hDC);
// 		return -1;
// 	}

// 	WORD *gamma_r = &gamma_ramps[0*GAMMA_RAMP_SIZE];
// 	WORD *gamma_g = &gamma_ramps[1*GAMMA_RAMP_SIZE];
// 	WORD *gamma_b = &gamma_ramps[2*GAMMA_RAMP_SIZE];

// 	if (preserve) {
// 		/* Initialize gamma ramps from saved state */
// 		memcpy(gamma_ramps, state->saved_ramps,
// 		       3*GAMMA_RAMP_SIZE*sizeof(WORD));
// 	} else {
// 		/* Initialize gamma ramps to pure state */
// 		for (int i = 0; i < GAMMA_RAMP_SIZE; i++) {
// 			WORD value = (double)i/GAMMA_RAMP_SIZE *
// 				(UINT16_MAX+1);
// 			gamma_r[i] = value;
// 			gamma_g[i] = value;
// 			gamma_b[i] = value;
// 		}
// 	}

// 	colorramp_fill(gamma_r, gamma_g, gamma_b, GAMMA_RAMP_SIZE,
// 		       setting);

// 	/* Set new gamma ramps */
// 	r = FALSE;
// 	for (int i = 0; i < MAX_ATTEMPTS && !r; i++) {
// 		/* We retry a few times before giving up because some
// 		   buggy drivers fail on the first invocation of
// 		   SetDeviceGammaRamp just to succeed on the second. */
// 		r = SetDeviceGammaRamp(hDC, gamma_ramps);
// 	}
// 	if (!r) {
// 		fputs(_("Unable to set gamma ramps.\n"), stderr);
// 		free(gamma_ramps);
// 		ReleaseDC(NULL, hDC);
// 		return -1;
// 	}

// 	free(gamma_ramps);

// 	/* Release device context */
// 	ReleaseDC(NULL, hDC);

// 	return 0;
// }
