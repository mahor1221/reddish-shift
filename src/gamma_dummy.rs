/*  gamma-dummy.rs -- No-op gamma adjustment
    This file is part of <https://github.com/mahor1221/reddish-shift>.
    Copyright (C) 2024 Mahor Foruzesh <mahor1221@gmail.com>
    Ported from Redshift <https://github.com/jonls/redshift>.
    Copyright (c) 2013-2017  Jon Lund Steffensen <jonlst@gmail.com>

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

// #[repr(C)]
// pub struct gamma_method_t {
//     pub name: *mut c_char,

//     // If true, this method will be tried if none is explicitly chosen.
//     pub autostart: c_int,

//     // Initialize state. Options can be set between init and start.
//     pub init: Option<gamma_method_init_func>,
//     // Allocate storage and make connections that depend on options.
//     pub start: Option<gamma_method_start_func>,
//     // Free all allocated storage and close connections.
//     pub free: Option<gamma_method_free_func>,

//     // Print help on options for this adjustment method.
//     pub print_help: Option<gamma_method_print_help_func>,
//     // Set an option key, value-pair
//     pub set_option: Option<gamma_method_set_option_func>,

//     // Restore the adjustment to the state before start was called.
//     pub restore: Option<gamma_method_restore_func>,
//     // Set a specific color temperature.
//     pub set_temperature: Option<gamma_method_set_temperature_func>,
// }

use crate::{config::ColorSettings, Method};
use anyhow::Result;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Dummy;
impl Method for Dummy {
    fn set_color(
        &self,
        cs: &ColorSettings,
        preserve_gamma: bool,
    ) -> Result<()> {
        // println!("Temperature: {temp}"); // (*setting).temperature);
        println!("{cs:?}\npreserve_gamma: {preserve_gamma}");
        Ok(())
    }
}

// impl GammaAdjuster for Dummy {
//     fn start() {
//         eprintln!(
//             "WARNING: Using dummy gamma method! Display will not be affected by this gamma method."
//         );
//     }

//     fn restore() {}

//     fn free() {}

//     fn print_help() {
//         println!("Does not affect the display but prints the color temperature to the terminal.")
//     }

//     fn set_option() {
//         // state: *mut c_void,
//         // key: *const c_char,
//         // value: *const c_char,
//         let key = "";
//         eprintln!("Unknown method parameter: `{key}`");
//         // return -1
//     }

//     fn set_temperature(&self) {
//         // state: *mut c_void,
//         // setting: *const ColorSetting,
//         // preserve: c_int,

//         let temp = "";
//         println!("Temperature: {temp}"); // (*setting).temperature);

//         // return 0
//     }
// }
