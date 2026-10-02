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

//! This module also implements player structs and enums to store
//! the current player status and consistently update that status.

// Import crate definitions
use crate::definitions::*;

// Import standard library features
use std::collections::HashMap;

/// A type to store a hashmap of player ids and status details
///
pub type PlayerMap = HashMap<PlayerId, PlayerDetail>; // a hash map of player id and player details

/// A struct to hold the state of a partiuclar player
///
#[derive(PartialEq, Eq, Clone, Debug, Default, Serialize, Deserialize)]
pub struct PlayerDetail {
    name: Option<String>, // name of the player, if provided
    email: Option<String>, // email of the player, if provided
    current_puzzle: Option<UniquePuzzle>, // the puzzle currently being played
    current_scores: AllScores, // the score for the player for each game
}

// Implement key features for player status
impl PlayerDetail {
    /// A method to set the player name
    /// 
    pub fn set_name(&mut self, new_name: String) {
        self.name = Some(new_name);
    }

    /// A method to set the player email
    /// 
    pub fn set_email(&mut self, new_email: String) {
        self.email = Some(new_email);
    }

    /// A method to set or clear the current puzzle for this player
    /// 
    pub fn set_current_puzzle(&mut self, current_puzzle: Option<UniquePuzzle>) {
        self.current_puzzle = current_puzzle;
    }

    /// A method to set the score for a particular puzzle
    /// 
    pub fn set_score(&mut self, unique_puzzle: UniquePuzzle, score: Score) {
        // Insert or access the scores for this game
        self.current_scores.entry(unique_puzzle.game_id)
        .and_modify(|game_scores| { 
            // Update the existing score for this puzzle
            game_scores.insert(unique_puzzle.puzzle_id, score);
        })
        .or_insert_with(|| {
            // Create a new game scores for this game and add this puzzle
            let mut game_scores = GameScores::default();
            game_scores.insert(unique_puzzle.puzzle_id, score);
            game_scores
        });
    }

    /// A method to set all the scores for a particular game
    /// 
    pub fn set_scores(&mut self, game_id: GameId, game_scores: GameScores) {
        self.current_scores.insert(game_id, game_scores);
    }

    /// A method to remove identifying data
    /// 
    pub fn clear_pii(&mut self) {
        self.name = None;
        self.email = None;
    }

    /// A method to get a copy of the player name, if it exists
    /// 
    pub fn get_name(&self) -> Option<String> {
        self.name.clone()
    }

    /// A method to get a copy of the player email, if it exists
    /// 
    pub fn get_email(&self) -> Option<String> {
        self.email.clone()
    }

    /// A method to get the current puzzle for this player, if it exists
    /// 
    pub fn get_current_puzzle(&self) -> Option<UniquePuzzle> {
        self.current_puzzle.clone()
    }

    /// A method to get all the scores for this player
    /// 
    pub fn get_all_scores(&self) -> AllScores {
        self.current_scores.clone()
    }

    /// A method to get all the detail for this player
    /// 
    pub fn get_detail(&self) -> PlayerDetail {
        self.clone()
    }
}



// Tests of the status module
#[cfg(test)]
mod tests {
    use super::*;

    // Test creation and modification of a player detail
    #[test]
    fn missing_tests() {
        unimplemented!();
    }
}
