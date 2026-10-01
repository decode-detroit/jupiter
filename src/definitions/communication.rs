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

/// The stucture and methods to send updates to the player(s).
///
#[derive(Clone, Debug)]
pub struct PlayerSend {
    update_send: mpsc::Sender<Update>, // the line to pass updates to the player(s)
}

// Implement the key features of player send
impl PlayerSend {
    /// A function to create a new PlayerSend
    ///
    /// The function returns the PlayerSend structure and the interface
    /// receive channel which will deliver the provided updates.
    ///
    pub fn new() -> (Self, mpsc::Receiver<Update>) {
        // Create one or two new channels
        let (update_send, update_recv) = mpsc::channel(512);

        // Create and return the new items
        (PlayerSend { update_send }, update_recv)
    }

    /// A method to send a player update. This method fails silently.
    ///
    pub async fn send(&self, update: Update) {
        self.update_send.send(update).await.unwrap_or(());
    }
}

/// An enum type to provide updates to the players
///
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Update {
    /// A variant to provide the current scene and state of all statuses
    #[serde(rename_all = "camelCase")]
    CurrentSceneAndStatus {
        current_scene: ItemId,
        current_status: CurrentStatus,
    },

    /// A variant to post a current event to the status bar
    #[serde(rename_all = "camelCase")]
    Notify { message: String },

    /// A variant to indicate that the entire interface should be refreshed
    #[serde(rename_all = "camelCase")]
    RefreshAll,

    /// A variant indicating the current scene should be refreshed with
    /// the new scene.
    #[serde(rename_all = "camelCase")]
    UpdateScene { current_scene: ItemId },

    /// A variant to update the state of a partiular status.
    #[serde(rename_all = "camelCase")]
    UpdateStatus {
        status_id: ItemPair, // the status to update
        new_state: ItemPair, // the new state of the status
    },

    /// A variant indicating that the system notifications should be updated.
    #[serde(rename_all = "camelCase")]
    UpdateNotifications { notifications: Vec<String> },

    /// A variant indicating that the event timeline should be updated.
    #[serde(rename_all = "camelCase")]
    UpdateTimeline { events: Vec<UpcomingEvent> },
}

// Implement from<Update> for Message)
impl From<Update> for Result<Message, warp::Error> {
    fn from(update: Update) -> Self {
        // Try to serialize the update
        match serde_json::to_string(&update) {
            Ok(string) => Ok(Message::text(string)),

            // On failure, return an empty string (unable to convert the error)
            _ => Ok(Message::text("")),
        }
    }
}

/// The stucture and methods to send requests to jupiter
///
#[derive(Clone, Debug)]
pub struct JupiterSend {
    web_send: mpsc::Sender<WebRequest>, // the mpsc sending line to pass player requests
}

// Implement the key features of the web send struct
impl JupiterSend {
    /// A function to create a new WebSend
    ///
    /// The function returns the the Web Sent structure and the system
    /// receive channel which will return the provided updates.
    ///
    pub fn new() -> (Self, mpsc::Receiver<WebRequest>) {
        // Create the new channel
        let (web_send, receive) = mpsc::channel(512);

        // Create and return both new items
        (JupiterSend { web_send }, receive)
    }

    /// A method to send a web request. This method fails silently.
    ///
    pub async fn send(&self, reply_to: oneshot::Sender<Reply>, request: Request) {
        self.web_send
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Request {
    /// A special variant to close the program.
    Close,

    /// A variant to create (or connect to an existing) player id
    CreatePlayer { player_id: PlayerId },

    /// A variant to cue an event for a particular puzzle
    CueEvent { player_id: PlayerId, unique_puzzle: UniquePuzzle, event_id: ItemId },

    /// A variant to get the status of a player
    PlayerStatus { player_id: PlayerId },

    /// A special variant to close the program, and attempt to shut down the computer
    Shutdown,

    /// A variant to request to start a puzzle
    StartPuzzle { player_id: PlayerId, unique_puzzle: UniquePuzzle },

    /// A variant to check if a player id exists
    VerifyPlayer { player_id: PlayerId },

    /// A variant to verify this player id for a given game and puzzle
    VerifyCurrentPlayer { player_id: PlayerId, unique_puzzle: UniquePuzzle },
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
    /// A variant for 
    Placeholder,

    /// A variant for replies with a message
    #[serde(rename_all = "camelCase")]
    Message(String),
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
