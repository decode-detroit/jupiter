// Copyright (c) 2026 Decode Detroit
// Author: Patton Doyle
// Licence: GNU GPLv3
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

//! This module defines the interaction locations for Minerva instances
//! and their connection to the game(s) on Jupiter.

// Import crate definitions
use crate::definitions::*;

/// Define parameters for a Minerva connection
///
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MinervaParams {
    pub game_id: GameId,
    pub address: Option<String>,
}

/// A collection of Minerva parameters for the configuration file
/// 
pub type MinervaControllers = Vec<MinervaParams>;
