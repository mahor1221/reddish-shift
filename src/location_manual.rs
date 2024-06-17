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

use crate::{
    types::Location,
    utils::{Verbosity, Write},
    Provider,
};
use anyhow::Result;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Manual {
    location: Location,
}

impl Manual {
    pub fn new(location: Location) -> Self {
        Self { location }
    }
}

impl Provider for Manual {
    fn get(
        &self,
        _v: &mut Verbosity<impl Write, impl Write>,
    ) -> Result<Location> {
        Ok(self.location)
    }
}
