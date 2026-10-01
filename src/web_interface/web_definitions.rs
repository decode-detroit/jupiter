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

// Import Tokio and warp features
use tokio::sync::mpsc;
use warp::ws::Message;

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

/// Helper struct to share a websocket with its expiration time (JWT standard expiration)
///
pub struct ListenerWithExpiration {
    pub socket: mpsc::Sender<Result<Message, warp::Error>>, // the sender for the websocket
    pub expiration: u64, // the expiration time of the websocket, in UNIX Epoch seconds. 0 for no expiration
}

/// Helper types and structs for passing requests to Jupiter
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePlayer {
    pub player_id: String,
}
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartPuzzle {
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
