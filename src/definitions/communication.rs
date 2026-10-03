// Copyright (c) 2019-2021 Decode Detroit
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

//! This module implements shared communication structures for communicating
//! across the modules of the system.

// Import crate definitions
use crate::definitions::*;

// Import Tokio features
use tokio::sync::{mpsc, oneshot};

// Import warp features
use warp::ws::Message;

// Import FNV HashMap
use fnv::FnvHashMap;

/// The stucture and methods to send requests to jupiter
///
#[derive(Clone, Debug)]
pub struct JupiterSend {
    send: mpsc::Sender<WebRequest>, // the mpsc sending line to pass player requests
}

// Implement the key features of the jupiter send struct
impl JupiterSend {
    /// A function to create a new JupiterSend
    ///
    /// The function returns the the JupiterSend structure and the system
    /// receive channel which will return the provided updates.
    ///
    pub fn new() -> (Self, mpsc::Receiver<WebRequest>) {
        // Create the new channel
        let (send, receive) = mpsc::channel(512);

        // Create and return both new items
        (Self { send }, receive)
    }

    /// A method to send a web request. This method fails silently.
    ///
    pub async fn send(&self, reply_to: oneshot::Sender<Reply>, request: Request) {
        self.send
            .send(WebRequest { reply_to, request })
            .await
            .unwrap_or(());
    }
}

/// A structure for carrying requests from the web interface
///
pub struct WebRequest {
    pub reply_to: oneshot::Sender<Reply>, // the handle for replying to the reqeust
    pub request: Request,                 // the request
}

/// An enum to carry requests from the player(s)
///
#[derive(Clone, Debug)]
pub enum Request {
    /// A special variant to close the program.
    Close,

    /// A variant to create (or connect to an existing) player id
    CreatePlayer { player_id: PlayerId },

    /// A variant to cue an event for a particular puzzle
    CueEvent { player_id: PlayerId, unique_puzzle: UniquePuzzle, event_id: ItemId },

    /// A variant to get the current scores for a player
    PlayerScores { player_id: PlayerId },

    /// A variant to request updates whenever a player's score changes
    PlayerScoreUpdates { player_id: PlayerId, sender: SenderWithExpiration },

    /// A special variant to close the program, and attempt to shut down the computer
    Shutdown,

    /// A variant to request to start a puzzle
    StartPuzzle { player_id: PlayerId, unique_puzzle: UniquePuzzle },

    /// A variant to check if a player id exists
    VerifyPlayer { player_id: PlayerId },

    /// A variant to verify this player id for a given game and puzzle
    VerifyCurrentPlayer { player_id: PlayerId, unique_puzzle: UniquePuzzle },
}

// Implement Request features
impl Request {
    /// A helper method to quickly extract the player_id from a request, if provided.
    /// 
    pub fn get_player_id(&self) -> Result<PlayerId> {
        match self {
            // Clone the internal player Id
            Request::CreatePlayer { player_id } => Ok(player_id.clone()),
            Request::CueEvent { player_id, .. } => Ok(player_id.clone()),
            Request::PlayerScores { player_id } => Ok(player_id.clone()),
            Request::PlayerScoreUpdates { player_id, .. } => Ok(player_id.clone()),
            Request::StartPuzzle { player_id, .. } => Ok(player_id.clone()),
            Request::VerifyPlayer { player_id } => Ok(player_id.clone()),
            Request::VerifyCurrentPlayer { player_id, .. } => Ok(player_id.clone()),

            // No player ID for remaining variants
            _ => Err(anyhow!("Request does not contain a player ID.")),
        }
    }
}


/// Helper struct to share a websocket with its expiration time
/// (JWT standard expiration)
///
#[derive(Clone, Debug)]
pub struct SenderWithExpiration {
    pub socket: mpsc::Sender<Result<Message, warp::Error>>, // the sender for the websocket
    pub expiration: u64, // the expiration time of the websocket, in UNIX Epoch seconds. 0 for no expiration
}

// Implement from<AllScores> for Message for websocket messages
impl From<AllScores> for Result<Message, warp::Error> {
    fn from(all_scores: AllScores) -> Self {
        // Try to serialize the update
        match serde_json::to_string(&all_scores) {
            Ok(string) => Ok(Message::text(string)),

            // On failure, return an empty string (unable to convert the error)
            _ => Ok(Message::text("")),
        }
    }
}


/// A struct to cover all replies
///
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reply {
    pub is_valid: bool,
    pub data: ReplyData,
}

/// An enum to cover all reply data typers
///
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReplyData {
    /// A variant for replies with a message
    #[serde(rename_all = "camelCase")]
    Message(String),

    /// A variant for replies with player scores
    #[serde(rename_all = "camelCase")]
    Scores(AllScores),
}

// Implement key features of the web reply
impl Reply {
    /// A function to return a new, successful web reply
    ///
    pub fn success() -> Reply {
        Reply {
            is_valid: true,
            data: ReplyData::Message("Request completed.".into()),
        }
    }

    /// A function to return a new, failed web reply
    ///
    pub fn failure<S>(reason: S) -> Reply
    where
        S: Into<String>,
    {
        Reply {
            is_valid: true,
            data: ReplyData::Message(reason.into()),
        }
    }

    /// A method to check if the reply is a success
    ///
    pub fn is_success(&self) -> bool {
        self.is_valid
    }
}

/// The stucture and methods to send updates from Minerva
///
#[derive(Clone, Debug)]
pub struct MinervaSend {
    send: mpsc::Sender<GameUpdate>, // the mpsc sending line for game updates
}

// Implement the key features of the minerva send struct
impl MinervaSend {
    /// A function to create a new MinervaSend
    ///
    /// The function returns the the MinervaSend structure and the system
    /// receive channel which will return the provided updates.
    ///
    pub fn new() -> (Self, mpsc::Receiver<GameUpdate>) {
        // Create the new channel
        let (send, receive) = mpsc::channel(512);

        // Create and return both new items
        (Self { send }, receive)
    }

    /// A method to send a game update. This method fails silently.
    ///
    pub async fn send(&self, game_id: GameId, update: MinervaUpdate) {
        self.send
            .send(GameUpdate { game_id, update })
            .await
            .unwrap_or(());
    }
}

/// A structure for carrying updates about a game
///
pub struct GameUpdate {
    pub game_id: GameId, // the game that this Minerva instance is running
    pub update: MinervaUpdate, // the update
}


/// An enum type to receive updates from Minerva. These updates contain only
/// the minimal information needed to follow operations as they progress.
///
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MinervaUpdate {
    /// A variant to receive a current event id
    #[serde(rename_all = "camelCase")]
    CurrentEvent {
        event: ItemId, // current event id
    },

    /// A variant to receive the current scene and state of all statuses
    #[serde(rename_all = "camelCase")]
    CurrentSceneAndStatus {
        current_scene: ItemId,
        current_status: CurrentStatus,
    },

    /// A variant indicating a change in the current scene
    #[serde(rename_all = "camelCase")]
    UpdateScene { current_scene: ItemId },

    /// A variant to update the state of a partiular status
    #[serde(rename_all = "camelCase")]
    UpdateStatus {
        status_id: ItemId, // the status to update
        new_state: ItemId, // the new state of the status
    },
}

/// A type to share the current status of the game
///
pub type CurrentStatus = FnvHashMap<u32, u32>;
