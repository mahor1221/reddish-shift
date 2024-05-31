/*  location-manual.rs -- Manual location provider
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

// TODO: map country names to geo location

// #[repr(C)]
// pub struct location_provider_t {
//     pub name: *mut c_char,

//     // Initialize state. Options can be set between init and start.
//     pub init: Option<location_provider_init_func>,
//     // Allocate storage and make connections that depend on options.
//     pub start: Option<location_provider_start_func>,
//     // Free all allocated storage and close connections.
//     pub free: Option<location_provider_free_func>,

//     // Print help on options for this location provider.
//     pub print_help: Option<location_provider_print_help_func>,
//     // Set an option key, value-pair.
//     pub set_option: Option<location_provider_set_option_func>,

//     // Listen and handle location updates.
//     pub get_fd: Option<location_provider_get_fd_func>,
//     pub handle: Option<location_provider_handle_func>,

use crate::options::Location;
use anyhow::{anyhow, Result};

pub trait LocationProvider {
    fn start();
    fn help() -> &'static str;
    fn fd() -> Result<()>;
    fn handle(&self) -> Result<(Location, bool)>;
}

pub struct Manual {
    pub location: Location,
}

impl LocationProvider for Manual {
    fn start() {}

    fn help() -> &'static str {
        // TRANSLATORS: Manual location help output
        // left column must not be translated
        "Specify location manually.

  lat=N\t\tLatitude
  lon=N\t\tLongitude
  
  Both values are expected to be floating point numbers,
  negative values representing west / south, respectively."
    }

    fn fd() -> Result<()> {
        Err(anyhow!("-1"))
    }

    fn handle(&self) -> Result<(Location, bool)> {
        let available = true;
        Ok((self.location, available))
    }
}
