// Copyright (c) 2019-21 Decode Detroit
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

//! This module implements the puzzle handler to maintain the status of
//! each puzzle. This handler stores the status locally and expects to receive
//! updates on the puzzle status from Minerva.

// Import crate definitions
use crate::definitions::*;

// Import standard library features
use std::collections::hash_map::Entry::Occupied;

/// A structure which holds the puzzle status and manages any state changes.
///
pub struct PuzzleHandler {
    game_map: AllGames, // hash map of the individual games
}

// Implement key features for the puzzle handler
impl PuzzleHandler {
    /// A function to create and return a new puzzle handler.
    ///
    pub fn new(game_map: AllGames) -> Self {
        // Return the new status handler
        Self { game_map }
    }

    /// A method to check if a puzzle is currently available.
    /// 
    /// # Errors
    /// 
    /// This method will raise an error if the specified game or puzzle does
    /// not exist.
    /// 
    #[allow(dead_code)]
    pub fn is_available(&mut self, unique_puzzle: UniquePuzzle) -> bool {
        // Try to look up the game
        match self.game_map.entry(unique_puzzle.game_id) {
            Occupied(mut game) => {
                // Try to look up the puzzle
                match game.get_mut().puzzles.entry(unique_puzzle.puzzle_id) {
                    // Change the puzzle state and return the result
                    Occupied(puzzle) => puzzle.get().is_available(),
        
                    // The game does not exist
                    _ => {
                        error!("Puzzle Id does not exist.");
                        false
                    }
                }
            }

            // The game does not exist
            _ => {
                error!("Game Id does not exist.");
                false
            }
        }
    }

    /// A method to start a puzzle, if it is currently available. This method
    /// returns the starting event which should be forwarded to Minerva and will
    /// update the puzzle to the starting state.
    /// 
    /// # Errors
    /// 
    /// This method will return an error if the specified game or puzzle does
    /// not exist or if the puzzle is not available.
    /// 
    pub fn start_puzzle(&mut self, unique_puzzle: UniquePuzzle, player_id: PlayerId) -> Result<ItemId> {
        // Try to look up the game
        match self.game_map.entry(unique_puzzle.game_id) {
            Occupied(mut game) => {
                // Try to look up the puzzle
                match game.get_mut().puzzles.entry(unique_puzzle.puzzle_id) {
                    // Change the puzzle state and return the result
                    Occupied(mut puzzle) => puzzle.get_mut().start_puzzle(player_id),
        
                    // The puzzle does not exist
                    _ => Err(anyhow!("Puzzle Id does not exist.")),
                }
            }

            // The game does not exist
            _ => Err(anyhow!("Game Id does not exist.")),
        }
    }

    /// A method to return a puzzle to its available state (should be used
    /// only when attempting to start a puzzle but Minerva could not be reached)
    /// 
    /// # Errors
    /// 
    /// This method will return an error if the specified game or puzzle does
    /// not exist or if the puzzle is not available.
    /// 
    pub fn reset_puzzle(&mut self, unique_puzzle: UniquePuzzle) -> Result<()> {
        // Try to look up the game
        match self.game_map.entry(unique_puzzle.game_id) {
            Occupied(mut game) => {
                // Try to look up the puzzle
                match game.get_mut().puzzles.entry(unique_puzzle.puzzle_id) {
                    // Change the puzzle state and return the result
                    Occupied(mut puzzle) => {
                        puzzle.get_mut().reset_puzzle();
                        Ok(())
                    }
        
                    // The puzzle does not exist
                    _ => Err(anyhow!("Puzzle Id does not exist.")),
                }
            }

            // The game does not exist
            _ => Err(anyhow!("Game Id does not exist.")),
        }
    }

    /// A method to verify that the provided player is the current player
    /// for the specified puzzle.
    /// 
    /// This method will return an error if the specified game or puzzle does
    /// not exist or if the player is not the current player.
    /// 
    pub fn verify_current_player(&self, unique_puzzle: &UniquePuzzle, player_id: &PlayerId) -> Result<()> {
        // Try to look up the game
        if let Some(game) = self.game_map.get(&unique_puzzle.game_id) {
            // Try to look up the puzzle
            if let Some(puzzle) = game.puzzles.get(&unique_puzzle.puzzle_id) {
                // Verify the current player
                puzzle.verify_current_player(player_id)
    
            // The puzzle does not exist
            } else {
                Err(anyhow!("Puzzle Id does not exist."))
            }

        // The game does not exist
        } else {
            Err(anyhow!("Game Id does not exist."))
        }
    }

    /// A method to update the state of the specified puzzle. This method
    /// returns a player id and score if the new state yielded a score.
    /// 
    /// # Errors
    /// 
    /// This method will raise an error if the specified game or puzzle does
    /// not exist.
    /// 
    pub fn change_state(&mut self, unique_puzzle: UniquePuzzle, new_state: ItemId) -> Option<(PlayerId, Score)> {
        // Try to look up the game
        match self.game_map.entry(unique_puzzle.game_id) {
            Occupied(mut game) => {
                // Try to look up the puzzle
                match game.get_mut().puzzles.entry(unique_puzzle.puzzle_id) {
                    // Change the puzzle state and return the result
                    Occupied(mut puzzle) => puzzle.get_mut().change_state(new_state),
        
                    // The puzzle does not exist
                    _ => {
                        warn!("Puzzle Id does not exist.");
                        None
                    }
                }
            }

            // The game does not exist
            _ => {
                error!("Game Id does not exist.");
                None
            }
        }
    }
}

// Tests of the status module
#[cfg(test)]
mod tests {
    use super::*;

    // Test getting and modifying a status
    #[tokio::test]
    async fn missing_tests() {
        unimplemented!();
    }
}
