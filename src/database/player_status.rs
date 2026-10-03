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
// GNU General Public License for more detail.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

//! This module implements the connection to a Redis backup server to maintain
//! a live backup of the player state. This module maintains the player state 
//! locally if a Redis server is not connected, but data will not persist across
//! reboots.

// Import crate definitions
use crate::definitions::*;

// Import standard library features
use std::collections::HashMap;
use std::collections::hash_map::Entry::Occupied;

// Imprt redis client library
use redis::AsyncCommands;

// Import JWT features
use jsonwebtoken as jwt;

/// A structure which holds a reference to the Redis server (if it exists) and
/// syncronizes local data to and from the server.
///
/// # Notes
///
/// When created, the handler will attempt to connect to the requested
/// redis server. If the player handler cannot make the connection, the status
/// will be maintained locally instead.
///
pub enum PlayerHandler {
    /// A variant with a connection to the Redis server
    Connected {
        identifier: Identifier, // the optional identifier for this instance
        connection: redis::aio::MultiplexedConnection, // the Redis connection, if it exists
        player_listeners: HashMap<PlayerId, SenderWithExpiration>, // a map of player ids and sending lines to update the user
    },

    /// A variant without any connection to the Redis server
    Disconnected {
        player_map: PlayerMap, // the local copy of the player status
        player_listeners: HashMap<PlayerId, SenderWithExpiration>, // a map of player ids and sending lines to update the user
    }, // identifier is irrelevant as the data is internal
}

// Use the two variants internally
use PlayerHandler::{Connected, Disconnected};

// Implement key features for the player handler
impl PlayerHandler {
    /// A function to create and return a new player handler.
    ///
    /// # Errors
    ///
    /// This function will raise an error if it is unable to connect but a
    /// Redis server was provided.
    ///
    pub async fn new(identifier: Identifier, server_location: Option<String>) -> Self {
        // If a server location was specified
        if let Some(location) = server_location {
            // Try to connect to the Redis server
            if let Ok(client) = redis::Client::open(location.as_str()) {
                // Try to get a copy of the Redis connection
                if let Ok(mut connection) = client.get_multiplexed_async_connection().await {
                    // Try to set the snapshot settings
                    if redis::cmd("CONFIG").arg("SET").arg("save").arg("60 1").exec_async(&mut connection).await.is_err() {
                        // Warn that it wasn't possible to set the snapshot settings
                        error!("Unable to set Redis snapshot settings.");
                    }

                    // Return the new player handler
                    return Connected {
                        identifier,
                        connection,
                        player_listeners: HashMap::default(),
                    };

                // Indicate that there was a failure to connect to the server
                } else {
                    error!("Unable to connect to player server: {}.", location);
                }

            // Indicate that there was a failure to connect to the server
            } else {
                error!("Unable to connect to player server: {}.", location);
            }
        }

        // If a location was not specified or the connection failed,
        // return without a redis connection
        Disconnected {
            player_map: PlayerMap::default(),
            player_listeners: HashMap::default(),
        }
    }

    /// A method to indicate whether the player handler is connected to
    /// the Redis database
    /// 
    #[allow(dead_code)]
    pub fn is_connected(&self) -> bool {
        match self {
            Connected { .. } => true,
            Disconnected { .. } => false,
        }
    }

    /// A method to add a player to the database. This method does nothing
    /// if the player already exists.
    /// 
    /// # Errors
    ///
    /// This function will return an Err if it was unable to communicate
    /// with the Redis database.
    /// 
    pub async fn create_player(&mut self, player_id: PlayerId) -> Result<()> {
        // Match the connection type
        match self {
            // Verify that the player is in the Redis database
            &mut Connected { identifier, ref mut connection, .. } => {
                // See if the player exists
                match connection.get::<String, String>(format!("jupiter:{}:{}", identifier, player_id)).await {
                    Ok(_) => Ok(()),

                    // If not, add them
                    Err(_) => {
                        // Try to serialize the new player detail
                        let player_string = serde_yaml::to_string(&PlayerDetail::default())?;

                        // Try to add the player
                        connection.set::<String, String, bool>(format!("jupiter:{}:{}", identifier, player_id),player_string).await.context("Unable to create player.")?;

                        // Indicate success
                        Ok(())
                    }
                }
            },

            // Verify that the player is in the local database
            Disconnected { player_map, .. } => {
                // If the player doesn't exist, create a new empty one
                player_map.entry(player_id).or_default();
                
                // Indicate success
                Ok(())
            },
        }
    }

    /// A method to verify that player is in the database.
    /// 
    /// # Errors
    ///
    /// This function will return an Err if it was unable to communicate
    /// with the Redis database.
    /// 
    pub async fn verify_player(&mut self, player_id: &PlayerId) -> Result<()> {
        // Match the connection type
        match self {
            // Verify that the player is in the Redis database
            &mut Connected { identifier, ref mut connection, .. } => {
                // Try to read the player from the database
                connection.get::<String, String>(format!("jupiter:{}:{}", identifier, player_id)).await.context("Player Id does not exist.")?;

                // Indicate success
                Ok(())
            },

            // Verify that the player is in the local database
            Disconnected { player_map, .. } => {
                // If the player doesn't exist
                if !player_map.contains_key(&player_id) {
                    // Return an error
                    return Err(anyhow!("Player Id does not exist."));
                }
                
                // Indicate success
                Ok(())
            },
        }
    }

    /// A method to add a listening line for changes to a player
    ///
    /// # Errors
    /// 
    /// This ethod will return an error if the player does not exist
    /// 
    /// # Note
    /// 
    /// Only one user can be subscribed to a player at a time. Adding a new
    /// subscriber replaces the old one.
    /// 
    pub async fn add_listener(&mut self, player_id: PlayerId, sender: SenderWithExpiration) -> Result<()> {
        // Verify that the player exists
        self.verify_player(&player_id).await?;

        // Add the listening line to the corresponding hashmap
        match self {
            Connected { player_listeners, .. } => player_listeners.insert(player_id, sender),
            Disconnected { player_listeners, .. } => player_listeners.insert(player_id, sender),
        };

        // Indicate success
        Ok(())
    }

    /// A method to set or update the player name in player detail
    /// 
    /// # Errors
    ///
    /// This function will return an Err if it was unable to communicate
    /// with the Redis database or if the player does not exist.
    ///
    /// FIXME These functions are lots of boilerplate and could probably
    /// be easily condensed into a macro or other syntactic sugar
    /// 
    #[allow(dead_code)]
    pub async fn set_name(&mut self, player_id: PlayerId, name: String) -> Result<()> {
        // Match the connection type
        match self {
            // Verify that the player is in the Redis database
            &mut Connected { identifier, ref mut connection, .. } => {
                // Try to read the player from the database
                let mut player_string = connection.get::<String, String>(format!("jupiter:{}:{}", identifier, player_id)).await.context("Player Id does not exist.")?;

                // Attempt to serialize the string
                let mut detail = serde_yaml::from_str::<PlayerDetail>(&player_string).context("Unable to parse player detail.")?;

                // Update the detail
                detail.set_name(name);

                // Try to serialize the new player detail
                player_string = serde_yaml::to_string(&detail)?;

                // Try to set the new detail
                connection.set::<String, String, bool>(format!("jupiter:{}:{}", identifier, player_id),player_string).await.context("Unable to update player.")?;

                // Indicate success
                Ok(())
            },

            // Verify that the player is in the local database
            &mut Disconnected { ref mut player_map, .. } => {
                // Try to modify the existing player
                match player_map.entry(player_id.clone()) {
                    Occupied(mut detail) => {
                        // Set or update the score
                        detail.get_mut().set_name(name);

                        // Indicate success
                        Ok(())
                    }

                    // The player does not exist
                    _ => {
                        return Err(anyhow!("Player Id does not exist."));
                    }
                }
            },
        }
    }

    /// A method to set or update the player email in player detail
    /// 
    /// # Errors
    ///
    /// This function will return an Err if it was unable to communicate
    /// with the Redis database or if the player does not exist.
    ///
    /// FIXME These functions are lots of boilerplate and could probably
    /// be easily condensed into a macro or other syntactic sugar
    /// 
    #[allow(dead_code)]
    pub async fn set_email(&mut self, player_id: PlayerId, email: String) -> Result<()> {
        // Match the connection type
        match self {
            // Verify that the player is in the Redis database
            &mut Connected { identifier, ref mut connection, .. } => {
                // Try to read the player from the database
                let mut player_string = connection.get::<String, String>(format!("jupiter:{}:{}", identifier, player_id)).await.context("Player Id does not exist.")?;

                // Attempt to serialize the string
                let mut detail = serde_yaml::from_str::<PlayerDetail>(&player_string).context("Unable to parse player detail.")?;

                // Update the detail
                detail.set_email(email);

                // Try to serialize the new player detail
                player_string = serde_yaml::to_string(&detail)?;

                // Try to set the new detail
                connection.set::<String, String, bool>(format!("jupiter:{}:{}", identifier, player_id),player_string).await.context("Unable to update player.")?;

                // Indicate success
                Ok(())
            },

            // Verify that the player is in the local database
            &mut Disconnected { ref mut player_map, .. } => {
                // Try to modify the existing player
                match player_map.entry(player_id.clone()) {
                    Occupied(mut detail) => {
                        // Set or update the score
                        detail.get_mut().set_email(email);
                        
                        // Indicate success
                        Ok(())
                    }

                    // The player does not exist
                    _ => {
                        return Err(anyhow!("Player Id does not exist."));
                    }
                }
            },
        }
    }

    /// A method to set or update the current puzzle in player detail
    /// 
    /// # Errors
    ///
    /// This function will return an Err if it was unable to communicate
    /// with the Redis database or if the player does not exist.
    ///
    /// FIXME These functions are lots of boilerplate and could probably
    /// be easily condensed into a macro or other syntactic sugar
    /// 
    #[allow(dead_code)]
    pub async fn set_current_puzzle(&mut self, player_id: PlayerId, current_puzzle: Option<UniquePuzzle>) -> Result<()> {
        // Match the connection type
        match self {
            // Verify that the player is in the Redis database
            &mut Connected { identifier, ref mut connection, .. } => {
                // Try to read the player from the database
                let mut player_string = connection.get::<String, String>(format!("jupiter:{}:{}", identifier, player_id)).await.context("Player Id does not exist.")?;

                // Attempt to serialize the string
                let mut detail = serde_yaml::from_str::<PlayerDetail>(&player_string).context("Unable to parse player detail.")?;

                // Update the detail
                detail.set_current_puzzle(current_puzzle);

                // Try to serialize the new player detail
                player_string = serde_yaml::to_string(&detail)?;

                // Try to set the new detail
                connection.set::<String, String, bool>(format!("jupiter:{}:{}", identifier, player_id),player_string).await.context("Unable to update player.")?;

                // Indicate success
                Ok(())
            },

            // Verify that the player is in the local database
            &mut Disconnected { ref mut player_map, .. } => {
                // Try to modify the existing player
                match player_map.entry(player_id.clone()) {
                    Occupied(mut detail) => {
                        // Set or update the score
                        detail.get_mut().set_current_puzzle(current_puzzle);
                        
                        // Indicate success
                        Ok(())
                    }

                    // The player does not exist
                    _ => {
                        return Err(anyhow!("Player Id does not exist."));
                    }
                }
            },
        }
    }

    /// A method to set or update a puzzle score in the player detail
    /// 
    /// # Errors
    ///
    /// This function will return an Err if it was unable to communicate
    /// with the Redis database or if the player does not exist.
    ///
    /// FIXME These functions are lots of boilerplate and could probably
    /// be easily condensed into a macro or other syntactic sugar
    /// 
    pub async fn set_score(&mut self, player_id: PlayerId, unique_puzzle: UniquePuzzle, score: Score) -> Result<()> {
        // Match the connection type
        match self {
            // Verify that the player is in the Redis database
            &mut Connected { identifier, ref mut connection, .. } => {
                // Try to read the player from the database
                let mut player_string = connection.get::<String, String>(format!("jupiter:{}:{}", identifier, player_id)).await.context("Player Id does not exist.")?;

                // Attempt to serialize the string
                let mut detail = serde_yaml::from_str::<PlayerDetail>(&player_string).context("Unable to parse player detail.")?;

                // Update the detail
                detail.set_score(unique_puzzle, score);

                // Try to serialize the new player detail
                player_string = serde_yaml::to_string(&detail)?;

                // Try to set the new detail
                connection.set::<String, String, bool>(format!("jupiter:{}:{}", identifier, player_id),player_string).await.context("Unable to update player.")?;

                // Update any listeners
                self.send_score_updates(&player_id, detail.get_all_scores());

                // Indicate success
                Ok(())
            },

            // Verify that the player is in the local database
            &mut Disconnected { ref mut player_map, .. } => {
                // Save the new detail for future use
                let scores;

                // Try to modify the existing player
                match player_map.entry(player_id.clone()) {
                    Occupied(mut detail) => {
                        // Set or update the score
                        detail.get_mut().set_score(unique_puzzle, score);
                        
                        // Save the new detail
                        scores = detail.get().get_all_scores();
                    }

                    // The player does not exist
                    _ => {
                        return Err(anyhow!("Player Id does not exist."));
                    }
                }

                // Update any listeners
                self.send_score_updates(&player_id, scores);

                // Indicate success
                Ok(())
            },
        }
    }

    /// A method to set or update all scores for a game in the player detail
    /// 
    /// # Errors
    ///
    /// This function will return an Err if it was unable to communicate
    /// with the Redis database or if the player does not exist.
    ///
    /// FIXME These functions are lots of boilerplate and could probably
    /// be easily condensed into a macro or other syntactic sugar
    /// 
    #[allow(dead_code)]
    pub async fn set_scores(&mut self, player_id: PlayerId, game_id: GameId, game_scores: GameScores) -> Result<()> {
        // Match the connection type
        match self {
            // Verify that the player is in the Redis database
            &mut Connected { identifier, ref mut connection, .. } => {
                // Try to read the player from the database
                let mut player_string = connection.get::<String, String>(format!("jupiter:{}:{}", identifier, player_id)).await.context("Player Id does not exist.")?;

                // Attempt to serialize the string
                let mut detail = serde_yaml::from_str::<PlayerDetail>(&player_string).context("Unable to parse player detail.")?;

                // Update the detail
                detail.set_scores(game_id, game_scores);

                // Try to serialize the new player detail
                player_string = serde_yaml::to_string(&detail)?;

                // Try to set the new detail
                connection.set::<String, String, bool>(format!("jupiter:{}:{}", identifier, player_id),player_string).await.context("Unable to update player.")?;

                // Update any listeners
                self.send_score_updates(&player_id, detail.get_all_scores());

                // Indicate success
                Ok(())
            },

            // Verify that the player is in the local database
            &mut Disconnected { ref mut player_map, .. } => {
                // Save the new detail for future use
                let scores;

                // Try to modify the existing player
                match player_map.entry(player_id.clone()) {
                    Occupied(mut detail) => {
                        // Set or update the score
                        detail.get_mut().set_scores(game_id, game_scores);
                        
                        // Save the new detail
                        scores = detail.get().get_all_scores();
                    }

                    // The player does not exist
                    _ => {
                        return Err(anyhow!("Player Id does not exist."));
                    }
                }

                // Update any listeners
                self.send_score_updates(&player_id, scores);

                // Indicate success
                Ok(())
            },
        }
    }

    /// A method to clear personally identifying info from player detail
    /// 
    /// # Errors
    ///
    /// This function will return an Err if it was unable to communicate
    /// with the Redis database or if the player does not exist.
    ///
    /// FIXME These functions are lots of boilerplate and could probably
    /// be easily condensed into a macro or other syntactic sugar
    /// 
    #[allow(dead_code)]
    pub async fn clear_pii(&mut self, player_id: PlayerId) -> Result<()> {
        // Match the connection type
        match self {
            // Verify that the player is in the Redis database
            &mut Connected { identifier, ref mut connection, .. } => {
                // Try to read the player from the database
                let mut player_string = connection.get::<String, String>(format!("jupiter:{}:{}", identifier, player_id)).await.context("Player Id does not exist.")?;

                // Attempt to serialize the string
                let mut detail = serde_yaml::from_str::<PlayerDetail>(&player_string).context("Unable to parse player detail.")?;

                // Update the detail
                detail.clear_pii();

                // Try to serialize the new player detail
                player_string = serde_yaml::to_string(&detail)?;

                // Try to set the new detail
                connection.set::<String, String, bool>(format!("jupiter:{}:{}", identifier, player_id),player_string).await.context("Unable to update player.")?;

                // Indicate success
                Ok(())
            },

            // Verify that the player is in the local database
            &mut Disconnected { ref mut player_map, .. } => {
                // Try to modify the existing player
                match player_map.entry(player_id.clone()) {
                    Occupied(mut detail) => {
                        // Set or update the score
                        detail.get_mut().clear_pii();
                        
                        // Indicate success
                        Ok(())
                    }

                    // The player does not exist
                    _ => {
                        return Err(anyhow!("Player Id does not exist."));
                    }
                }
            },
        }
    }

    /// A method to get the player's name
    /// 
    /// # Errors
    ///
    /// This function will return an Err if it was unable to communicate
    /// with the Redis database or if the player does not exist.
    ///
    /// FIXME These functions are lots of boilerplate and could probably
    /// be easily condensed into a macro or other syntactic sugar
    /// 
    #[allow(dead_code)]
    pub async fn get_name(&mut self, player_id: PlayerId) -> Result<Option<String>> {
        // Match the connection type
        match self {
            // Verify that the player is in the Redis database
            &mut Connected { identifier, ref mut connection, .. } => {
                // Try to read the player from the database
                let player_string = connection.get::<String, String>(format!("jupiter:{}:{}", identifier, player_id)).await.context("Player Id does not exist.")?;

                // Attempt to serialize the string
                let detail = serde_yaml::from_str::<PlayerDetail>(&player_string).context("Unable to parse player detail.")?;

                // Return the player name
                Ok(detail.get_name())
            },

            // Verify that the player is in the local database
            &mut Disconnected { ref mut player_map, .. } => {
                // Try to retrive the player
                match player_map.get(&player_id) {
                    Some(detail) => {
                        // Return the player name
                        Ok(detail.get_name())
                    }

                    // The player does not exist
                    _ => {
                        Err(anyhow!("Player Id does not exist."))
                    }
                }
            },
        }
    }

    /// A method to get the player's email
    /// 
    /// # Errors
    ///
    /// This function will return an Err if it was unable to communicate
    /// with the Redis database or if the player does not exist.
    ///
    /// FIXME These functions are lots of boilerplate and could probably
    /// be easily condensed into a macro or other syntactic sugar
    /// 
    #[allow(dead_code)]
    pub async fn get_email(&mut self, player_id: PlayerId) -> Result<Option<String>> {
        // Match the connection type
        match self {
            // Verify that the player is in the Redis database
            &mut Connected { identifier, ref mut connection, .. } => {
                // Try to read the player from the database
                let player_string = connection.get::<String, String>(format!("jupiter:{}:{}", identifier, player_id)).await.context("Player Id does not exist.")?;

                // Attempt to serialize the string
                let detail = serde_yaml::from_str::<PlayerDetail>(&player_string).context("Unable to parse player detail.")?;

                // Return the player name
                Ok(detail.get_email())
            },

            // Verify that the player is in the local database
            &mut Disconnected { ref mut player_map, .. } => {
                // Try to retrive the player
                match player_map.get(&player_id) {
                    Some(detail) => {
                        // Return the player name
                        Ok(detail.get_email())
                    }

                    // The player does not exist
                    _ => {
                        Err(anyhow!("Player Id does not exist."))
                    }
                }
            },
        }
    }

    /// A method to get the player's current puzzle
    /// 
    /// # Errors
    ///
    /// This function will return an Err if it was unable to communicate
    /// with the Redis database or if the player does not exist.
    ///
    /// FIXME These functions are lots of boilerplate and could probably
    /// be easily condensed into a macro or other syntactic sugar
    /// 
    #[allow(dead_code)]
    pub async fn get_current_puzzle(&mut self, player_id: PlayerId) -> Result<Option<UniquePuzzle>> {
        // Match the connection type
        match self {
            // Verify that the player is in the Redis database
            &mut Connected { identifier, ref mut connection, .. } => {
                // Try to read the player from the database
                let player_string = connection.get::<String, String>(format!("jupiter:{}:{}", identifier, player_id)).await.context("Player Id does not exist.")?;

                // Attempt to serialize the string
                let detail = serde_yaml::from_str::<PlayerDetail>(&player_string).context("Unable to parse player detail.")?;

                // Return the player name
                Ok(detail.get_current_puzzle())
            },

            // Verify that the player is in the local database
            &mut Disconnected { ref mut player_map, .. } => {
                // Try to retrive the player
                match player_map.get(&player_id) {
                    Some(detail) => {
                        // Return the player name
                        Ok(detail.get_current_puzzle())
                    }

                    // The player does not exist
                    _ => {
                        Err(anyhow!("Player Id does not exist."))
                    }
                }
            },
        }
    }

    /// A method to get the player's scores
    /// 
    /// # Errors
    ///
    /// This function will return an Err if it was unable to communicate
    /// with the Redis database or if the player does not exist.
    ///
    /// FIXME These functions are lots of boilerplate and could probably
    /// be easily condensed into a macro or other syntactic sugar
    /// 
    pub async fn get_all_scores(&mut self, player_id: PlayerId) -> Result<AllScores> {
        // Match the connection type
        match self {
            // Verify that the player is in the Redis database
            &mut Connected { identifier, ref mut connection, .. } => {
                // Try to read the player from the database
                let player_string = connection.get::<String, String>(format!("jupiter:{}:{}", identifier, player_id)).await.context("Player Id does not exist.")?;

                // Attempt to serialize the string
                let detail = serde_yaml::from_str::<PlayerDetail>(&player_string).context("Unable to parse player detail.")?;

                // Return the player name
                Ok(detail.get_all_scores())
            },

            // Verify that the player is in the local database
            &mut Disconnected { ref mut player_map, .. } => {
                // Try to retrive the player
                match player_map.get(&player_id) {
                    Some(detail) => {
                        // Return the player name
                        Ok(detail.get_all_scores())
                    }

                    // The player does not exist
                    _ => {
                        Err(anyhow!("Player Id does not exist."))
                    }
                }
            },
        }
    }

    /// A method to get all the player's detail
    /// 
    /// # Errors
    ///
    /// This function will return an Err if it was unable to communicate
    /// with the Redis database or if the player does not exist.
    ///
    /// FIXME These functions are lots of boilerplate and could probably
    /// be easily condensed into a macro or other syntactic sugar
    /// 
    #[allow(dead_code)]
    pub async fn get_detail(&mut self, player_id: PlayerId) -> Result<PlayerDetail> {
        // Match the connection type
        match self {
            // Verify that the player is in the Redis database
            &mut Connected { identifier, ref mut connection, .. } => {
                // Try to read the player from the database
                let player_string = connection.get::<String, String>(format!("jupiter:{}:{}", identifier, player_id)).await.context("Player Id does not exist.")?;

                // Attempt to serialize the string
                let detail = serde_yaml::from_str::<PlayerDetail>(&player_string).context("Unable to parse player detail.")?;

                // Return the player name
                Ok(detail.get_detail())
            },

            // Verify that the player is in the local database
            &mut Disconnected { ref mut player_map, .. } => {
                // Try to retrive the player
                match player_map.get(&player_id) {
                    Some(detail) => {
                        // Return the player name
                        Ok(detail.get_detail())
                    }

                    // The player does not exist
                    _ => {
                        Err(anyhow!("Player Id does not exist."))
                    }
                }
            },
        }
    }

    /// A helper method to send score update to any subscribed users for a
    /// particular player. This method failes silently.
    /// 
    fn send_score_updates(&mut self, player_id: &PlayerId, new_scores: AllScores) {
        // Switch by connection type
        match self {
            Connected { player_listeners, .. } => {
                // Check to see if there is a listener for this player id
                if let Occupied(listener) = player_listeners.entry(player_id.clone()) {
                    // Check if the listener has expired
                    let expiration = listener.get().expiration;
                    if expiration == 0 && expiration > jwt::get_current_timestamp() {
                        // If not, send the score update
                        let _ = listener.get().socket.try_send(new_scores.into());
                    
                    // If so, remove it
                    } else {
                        listener.remove();
                    }
                }
            }
            Disconnected { player_listeners, .. } => {
                // Check to see if there is a listener for this player id
                if let Occupied(listener) = player_listeners.entry(player_id.clone()) {
                    // Check if the listener has expired
                    let expiration = listener.get().expiration;
                    if expiration == 0 && expiration > jwt::get_current_timestamp() {
                        // If not, send the score update
                        let _ = listener.get().socket.try_send(new_scores.into());
                    
                    // If so, remove it
                    } else {
                        listener.remove();
                    }
                }
            }
        }
    }
}

// Tests of the status module
#[cfg(test)]
mod tests {
    use super::*;

    // Test the connected version of the player handler
    #[tokio::test]
    async fn player_connected() {
        // Create the player handler
        let mut player_handler = PlayerHandler::new(
            Identifier { id: None },
            Some("redis://127.0.0.1:6379".into()),
        ).await;

        // Verify that is connected successfully
        assert!(player_handler.is_connected());

        // Try adding two new players
        let player1 = PlayerId::new("player1").unwrap();
        let player2 = PlayerId::new("player2").unwrap();
        assert!(player_handler.create_player(player1.clone()).await.is_ok());
        assert!(player_handler.create_player(player2.clone()).await.is_ok());

        // Verify that they exist but other players don't
        let player3 = PlayerId::new("player3").unwrap();
        assert!(player_handler.verify_player(&player1).await.is_ok());
        assert!(player_handler.verify_player(&player2).await.is_ok());
        assert!(player_handler.verify_player(&player3).await.is_err());

        // Try setting a player name
        player_handler.set_name(player1.clone(), "My Name".to_string()).await.unwrap();
        assert!(player_handler.set_name(player1.clone(), "My Name".to_string()).await.is_ok());
        assert_eq!(player_handler.get_name(player1.clone()).await.unwrap(), Some("My Name".to_string()));

        // Try setting the current puzzle
        let puzzle = UniquePuzzle { game_id: GameId::new("game1").unwrap(), puzzle_id: PuzzleId::new_unchecked(100) };
        assert!(player_handler.set_current_puzzle(player1.clone(), Some(puzzle.clone())).await.is_ok());
        assert_eq!(player_handler.get_current_puzzle(player1.clone()).await.unwrap(), Some(puzzle.clone()));

        // Try setting a particular score
        assert!(player_handler.set_score(player1.clone(), puzzle.clone(), 300).await.is_ok());
        assert_eq!(player_handler.get_all_scores(player1.clone()).await.unwrap().scores.get(&puzzle.game_id).unwrap().get(&puzzle.puzzle_id), Some(&300));
    }

    // Test the disconnected version of the player handler
    #[tokio::test]
    async fn player_disconnected() {
        // Create the player handler
        let mut player_handler = PlayerHandler::new(
            Identifier { id: None },
            None,
        ).await;

        // Verify that is not connected
        assert!(!player_handler.is_connected());

        // Try adding two new players
        let player1 = PlayerId::new("player1").unwrap();
        let player2 = PlayerId::new("player2").unwrap();
        assert!(player_handler.create_player(player1.clone()).await.is_ok());
        assert!(player_handler.create_player(player2.clone()).await.is_ok());

        // Verify that they exist but other players don't
        let player3 = PlayerId::new("player3").unwrap();
        assert!(player_handler.verify_player(&player1).await.is_ok());
        assert!(player_handler.verify_player(&player2).await.is_ok());
        assert!(player_handler.verify_player(&player3).await.is_err());

        // Try setting a player name
        assert!(player_handler.set_name(player1.clone(), "My Name".to_string()).await.is_ok());
        assert_eq!(player_handler.get_name(player1.clone()).await.unwrap(), Some("My Name".to_string()));

        // Try setting the current puzzle
        let puzzle = UniquePuzzle { game_id: GameId::new("game1").unwrap(), puzzle_id: PuzzleId::new_unchecked(100) };
        assert!(player_handler.set_current_puzzle(player1.clone(), Some(puzzle.clone())).await.is_ok());
        assert_eq!(player_handler.get_current_puzzle(player1.clone()).await.unwrap(), Some(puzzle.clone()));

        // Try setting a particular score
        assert!(player_handler.set_score(player1.clone(), puzzle.clone(), 300).await.is_ok());
        assert_eq!(player_handler.get_all_scores(player1.clone()).await.unwrap().scores.get(&puzzle.game_id).unwrap().get(&puzzle.puzzle_id), Some(&300));
    }
}
