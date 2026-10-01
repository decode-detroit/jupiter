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

//! This module also implements game and puzzle structs and enums to store
//! the current status of each game and puzzle and consistently update them.

// Import crate definitions
use crate::definitions::*;

// Import FNV HashMap
use fnv::FnvHashMap;

// Import anyhow features
use anyhow::Result;

// Define module constants
pub const STARTING_SCORE: u32 = 0_u32; // the starting score for a puzzle

/// A type definition to store all of the games
/// 
pub type Games = FnvHashMap<GameId, Game>; // a hash map of game id and game details

/// A struct to store an individual game, currently defined as a hashset of puzzles
///
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct Game {
    puzzles: FnvHashMap<PuzzleId, Puzzle>, // a set of puzzles for this game
}

// Implement key features for the Game struct
impl Game {
    /// A method to generate an empty score map for this game
    pub fn starting_scores(&self) -> GameScores {
        // Create score of zero for every puzzle
        let mut scores = GameScores::default();

        // Insert an score of 0 for every puzzle
        for puzzle in self.puzzles.keys() {
            scores.insert(*puzzle, STARTING_SCORE);
        }

        // Return the completed score map
        scores
    }
}

/// A type defining a puzzle score. Scores may mean different things in the context
/// of different games.
/// 
pub type Score = u32;

/// A type defining the score for an entire game, currently just a set of puzzle scores
/// 
pub type GameScores = FnvHashMap<PuzzleId, Score>;

/// A type defining the score for all the games, currently just a set of game scores
/// 
pub type AllScores = FnvHashMap<GameId, GameScores>;

/// A struct to identify a puzzle uniquely (even if puzzle ids are not
/// unique across games)
/// 
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct UniquePuzzle {
    pub game_id: GameId, // the game where this puzzle is located
    pub puzzle_id: PuzzleId, // the puzzle id
}


/// A struct to store the details about a particular puzzle
/// 
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct Puzzle {
    current_state: ItemId, // the current state of the puzzle
    available_state: ItemId, // the state when player(s) can be added
    starting_state: ItemId, // the state the puzzle should be changed to when a player is added
    score_map: FnvHashMap<ItemId, Score>, // a map of states to their corresponding score (including 0, typically a failure score)
    current_player: Option<PlayerId>, // the player who will receive the puzzle score
}

// Implement key features for the puzzle struct
impl Puzzle {
    /// A method to check if a puzzle is currently available for new players
    pub fn is_available(&self) -> bool {
        self.current_state == self.available_state
    }

    /// A method to start a puzzle, adding the player id to the current players
    /// and returning the starting state if successful
    pub fn start_puzzle(&mut self, player_id: PlayerId) -> Result<ItemId> {
        // Throw an error if the puzzle is not available
        if !self.is_available() {
            return Err(anyhow!("Puzzle is not available."));
        }

        // Update the current state
        self.current_state = self.starting_state;

        // Add the player id to the current players
        self.current_player = Some(player_id);

        // Return the starting state
        Ok(self.starting_state)
    }

    /// A method to update the current state of the puzzle. If the change
    /// results in a score and a current player was specified, the
    /// current player and their score is returned. In this case, the current
    /// player is also removed from the this puzzle.
    pub fn change_state(&mut self, new_state: ItemId) -> Option<(PlayerId, Score)> {
        // Update the current state
        self.current_state = new_state;

        // Check to see if the new state results in a score
        if let Some(score) = self.score_map.get(&new_state) {
            // See if there is a current player
            if let Some(player_id) = self.current_player.take() {
                // Return the player and score
                return Some((player_id, *score));
            }
        }

        // Otherwise, return None
        None
    }
}

// Tests of the status module
#[cfg(test)]
mod tests {
    use super::*;

    // Test creation of a game and exporting of starting scores
    #[test]
    fn game() {
        // Create a new game with one puzzle
        let puzzle_id = PuzzleId::new_unchecked(10);
        let puzzle = Puzzle {
            current_state: puzzle_id, // Invalid values, but not relevant here
            available_state: puzzle_id,
            starting_state: puzzle_id,
            score_map: FnvHashMap::default(),
            current_player: None,
        };
        let mut game = Game { puzzles: FnvHashMap::default() };
        game.puzzles.insert(puzzle_id, puzzle);

        // Get an set of starting scores
        let scores = game.starting_scores();

        // Verify that the score for the puzzle is the starting score
        assert_eq!(Some(&STARTING_SCORE), scores.get(&puzzle_id));
    }

    // Test creation and modification of a puzzle
    #[test]
    fn puzzle() {
        // Create a new redimentary puzzle
        let current_state = PuzzleId::new_unchecked(10);
        let available_state = PuzzleId::new_unchecked(11);
        let starting_state = PuzzleId::new_unchecked(12);
        let winning_state = PuzzleId::new_unchecked(13);
        let losing_state = PuzzleId::new_unchecked(14);
        let player_id = PlayerId::new("Player1").unwrap();
        let mut score_map = FnvHashMap::default();
        score_map.insert(winning_state, 100);
        score_map.insert(losing_state, 0);
        let mut puzzle = Puzzle {
            current_state,
            available_state,
            starting_state,
            score_map,
            current_player: None,
        };

        // Verify that the puzzle isn't currently available
        assert!(!puzzle.is_available());

        // Update the state and verify that the puzzle is available
        assert_eq!(None, puzzle.change_state(available_state));
        assert!(puzzle.is_available());
        
        // Start a player and verify that the puzzle isn't available
        assert_eq!(starting_state, puzzle.start_puzzle(player_id.clone()).unwrap());
        assert!(!puzzle.is_available());

        // Let the player win, and verify that the correct player and score is returned
        assert_eq!(Some((player_id, 100)), puzzle.change_state(winning_state));

        // Verify that the puzzle is still not available and no other 
        // winning (or losing) is allowed
        assert!(!puzzle.is_available());
        assert_eq!(None, puzzle.change_state(losing_state));
    }
}
