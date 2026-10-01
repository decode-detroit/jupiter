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

//! This module defines the unique identifier system (PuzzleId, GameId,
//! and PlayerId) to allow robust identification of all components.

// Import crate definitions
use crate::definitions::*;

// Import standard library features
use std::fmt;

// Import Serde macros
use serde::{Deserialize, Serialize};

pub const UNIVERSAL_IDENTIFIER: u32 = 0;

/// The Jupiter instance identifier. Instances with the same identifier will
/// share data when reloading, but risk overwriting each other during operation.
///
/// If no identifier is specified, this instance will use the universal identifier.
///
/// Note: Specifying an identifier with the universersal identifier (0) is the
/// same as specifying None.
///
#[derive(PartialEq, Eq, Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Identifier {
    pub id: Option<u32>, // An optionally-specified identifier for this instance
}

// Implement display for identifier
impl fmt::Display for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.id {
            Some(id) => write!(f, "{}", id),
            _ => write!(f, "~"),
        }
    }
}

/// A type definition for a puzzle id. PuzzleId matches the format of the ItemId
/// from sister program Minerva as they are used together.
/// 
pub type PuzzleId = ItemId;

/// A structure to hold a game id. This id is constrained to (approximately)
/// between 4 and 10 lowercase characters
/// 
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct GameId {
    id: String,
}

// Implement key GameId struct features
impl GameId {
    /// A function to create a new game id from a str.
    /// 
    /// This function will automatically trucate the string
    /// to (approximately) 10 characters and convert the to uppercase.
    /// 
    /// # Errors
    /// 
    /// This function will return None if the string is shorter than 4 characters.
    /// 
    pub fn new(unchecked_id: &str) -> Option<GameId> {
        // Truncate the string to the closest char boundary and convert to lowercase
        let id = unchecked_id[..unchecked_id.floor_char_boundary(10)].to_lowercase().clone();

        // If the id is too short, return none
        if id.len() < 4 {
            None
        } else {
            Some(GameId { id })
        }
    }

    /// A method to return the id of the game as a string
    ///
    pub fn id(&self) -> String {
        self.id.clone()
    }
}

// Implement displaying that shows the ID
impl fmt::Display for GameId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.id)
    }
}

/// A structure to hold a player id. This id is constrained to (approximately)
/// between 4 and 10 uppercase characters
///  
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct PlayerId {
    id: String,
}

// Implement key GameId struct features
impl PlayerId {
    /// A function to create a new player id from a str.
    /// 
    /// This function will automatically trucate the string
    /// to (approximately) 10 characters and convert the to uppercase.
    /// 
    /// # Errors
    /// 
    /// This function will return None if the string is shorter than 4 characters.
    /// 
    pub fn new(unchecked_id: &str) -> Option<PlayerId> {
        // Truncate the string to the closest char boundary and convert to uppercase
        let id= unchecked_id[..unchecked_id.floor_char_boundary(10)].to_uppercase().clone();

        // If the id is too short, return none
        if id.len() < 4 {
            None
        } else {
            Some(PlayerId { id })
        }
    }

    /// A method to return the id of the player as a string
    ///
    pub fn id(&self) -> String {
        self.id.clone()
    }
}

// Implement displaying that shows the ID
impl fmt::Display for PlayerId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.id)
    }
}


// Tests of the item module
#[cfg(test)]
mod tests {
    use super::*;

    // Test id with CAN bus limiter
    #[test]
    #[cfg_attr(not(feature = "no_can_limit"), should_panic)]
    fn create_puzzle_id() {
        // Try to create an id out of CAN range
        let _ = PuzzleId::new(0xFFFFFFFF).unwrap();
    }

    // Test comparison of simple item pairs
    #[test]
    fn compare_puzzle_ids() {
        // Create several ids
        let id = PuzzleId::new(1).unwrap();
        let same_id = id.clone();
        let different_id = PuzzleId::new(2).unwrap();

        // Compare the ids
        assert_eq!(id, same_id);
        assert_ne!(id, different_id);
        assert_ne!(same_id, different_id);
    }

    // Test creation and comparison of game ids
    #[test]
    fn compare_game_ids() {
        // Create several ids
        let id = GameId::new("game1game1");
        let same_id = GameId::new("GAME1GAME1GAME1");
        let different_id = GameId::new("game2game2");

        // Compare the ids
        assert_eq!(id, same_id);
        assert_ne!(id, different_id);
        assert_ne!(same_id, different_id);
    }

    // Test player id with string minimum
    #[test]
    #[should_panic]
    fn create_player_id() {
        // Try to create an id out of CAN range
        let _ = PlayerId::new("aa").unwrap();
    }

    // Test creation and comparison of player ids
    #[test]
    fn compare_player_ids() {
        // Create several ids
        let id = PlayerId::new("player11").unwrap();
        let same_id = PlayerId::new("PLAYER11111").unwrap();
        let different_id = PlayerId::new("player12").unwrap();

        // Compare the ids
        assert_eq!(id, same_id);
        assert_ne!(id, different_id);
        assert_ne!(same_id, different_id);
    }
    
    // Try to compare game and puzzle ids
    #[test]
    fn compare_game_and_player_ids() {
        // Create several ids
        let game_id = GameId::new("name1").unwrap();
        let player_id = PlayerId::new("name1").unwrap();

        // Compare the ids
        assert_ne!(game_id.id(), player_id.id());
    }
}
