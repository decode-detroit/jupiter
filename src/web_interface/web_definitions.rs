// Copyright (c) 2021 Decode Detroit
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

//! A module to create handy definitions for the web interface.
//! These definitions are not used elsewhere in the program.

// Import crate definitions
use crate::definitions::*;

/// Helper struct to define JWT claims for authorized users
///
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthClaims {
    pub iss: String,
    pub exp: u64,
}

/// Helper struct to define JWT claims for a player token
///
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerClaims {
    pub iss: String,
    pub plyr: String,
    pub exp: u64,
}

/// Helper types and structs for passing requests to Jupiter
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePlayer {
    pub player_id: String,
}
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CueEvent {
    pub player_id: String,
    pub game_id: String,
    pub puzzle_id: u32,
    pub event_id: u32,
}
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerScores {
    pub player_id: String,
}
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartPuzzle {
    pub player_id: String,
    pub game_id: String,
    pub puzzle_id: u32,
}
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyPlayer {
    pub player_id: String,
}
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyCurrentPlayer {
    pub player_id: String,
    pub game_id: String,
    pub puzzle_id: u32,
}

// Implement attempted conversions into a request
impl TryFrom<CreatePlayer> for Request {
    type Error = &'static str;

    // Required method
    fn try_from(create_player: CreatePlayer) -> Result<Self, Self::Error> {
        // Try to create the ids from the provided data
        let player_id = PlayerId::new(&create_player.player_id).ok_or("Player ID is not valid.")?;

        // Return the completed request
        Ok(Request::CreatePlayer { player_id })
    }
}
impl TryFrom<CueEvent> for Request {
    type Error = &'static str;

    // Required method
    fn try_from(cue_event: CueEvent) -> Result<Self, Self::Error> {
        // Try to create the ids from the provided data
        let player_id = PlayerId::new(&cue_event.player_id).ok_or("Player ID is not valid.")?;
        let game_id = GameId::new(&cue_event.game_id).ok_or("Game ID is not valid.")?;
        let puzzle_id = PuzzleId::new(cue_event.puzzle_id).ok_or("Puzzle ID is not valid.")?;
        let event_id = ItemId::new(cue_event.event_id).ok_or("Event ID is not valid.")?;

        // Return the completed request
        Ok(Request::CueEvent { player_id, unique_puzzle: UniquePuzzle { game_id, puzzle_id }, event_id })
    }
}
// Implement attempted conversions into a request
impl TryFrom<PlayerScores> for Request {
    type Error = &'static str;

    // Required method
    fn try_from(player_scores: PlayerScores) -> Result<Self, Self::Error> {
        // Try to create the id from the provided data
        let player_id = PlayerId::new(&player_scores.player_id).ok_or("Player ID is not valid.")?;

        // Return the completed request
        Ok(Request::PlayerScores { player_id })
    }
}
impl TryFrom<StartPuzzle> for Request {
    type Error = &'static str;

    // Required method
    fn try_from(start_puzzle: StartPuzzle) -> Result<Self, Self::Error> {
        // Try to create the ids from the provided data
        let player_id = PlayerId::new(&start_puzzle.player_id).ok_or("Player ID is not valid.")?;
        let game_id = GameId::new(&start_puzzle.game_id).ok_or("Game ID is not valid.")?;
        let puzzle_id = PuzzleId::new(start_puzzle.puzzle_id).ok_or("Puzzle ID is not valid.")?;

        // Return the completed request
        Ok(Request::StartPuzzle { player_id, unique_puzzle: UniquePuzzle { game_id, puzzle_id }})
    }
}
impl TryFrom<VerifyPlayer> for Request {
    type Error = &'static str;

    // Required method
    fn try_from(verify_player: VerifyPlayer) -> Result<Self, Self::Error> {
        // Try to create the ids from the provided data
        let player_id = PlayerId::new(&verify_player.player_id).ok_or("Player ID is not valid.")?;

        // Return the completed request
        Ok(Request::VerifyPlayer { player_id })
    }
}
impl TryFrom<VerifyCurrentPlayer> for Request {
    type Error = &'static str;

    // Required method
    fn try_from(verify_current_player: VerifyCurrentPlayer) -> Result<Self, Self::Error> {
        // Try to create the ids from the provided data
        let player_id = PlayerId::new(&verify_current_player.player_id).ok_or("Player ID is not valid.")?;
        let game_id = GameId::new(&verify_current_player.game_id).ok_or("Game ID is not valid.")?;
        let puzzle_id = PuzzleId::new(verify_current_player.puzzle_id).ok_or("Puzzle ID is not valid.")?;

        // Return the completed request
        Ok(Request::VerifyCurrentPlayer { player_id, unique_puzzle: UniquePuzzle { game_id, puzzle_id }})
    }
}
