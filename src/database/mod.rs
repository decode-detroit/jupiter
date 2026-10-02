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

//! A module to manage all the data of the system, including backing up the
//! player data to a Redis database, if specified

// Define program modules
mod config;
mod player_status;
mod puzzle_status;

// Import crate definitions
use crate::definitions::*;

// Import structures into this module
use config::Config;
use player_status::PlayerHandler;
use puzzle_status::PuzzleHandler;

// Load standard library features
use std::env;

// Import Tokio features
use tokio::fs::File;
use tokio::sync::mpsc;

/// A struct to load and hold the player, game, and puzzle databases
pub struct Database {
    config: Config, // the configuration for this instance of Jupiter
    player_status: PlayerHandler, // handler to manage status of the players
    puzzle_status: PuzzleHandler, // handler to manage status of the puzzles
}

// Implement key features for the Database structure
impl Database {
    /// A method to create a new instance of the database by loading the config
    ///
    pub async fn new(config_file: &str) -> Result<(Self, MinervaControllers)> {
        // Try to load the configuration in the current directory, if it exists
        let mut path = env::current_dir()?;

        // Add the specified or default filename
        path.push(config_file); // FIXME only allows for relative filepaths

        // If the path doesn't exist, throw an error
        if !path.exists() {
            return Err(anyhow!("Configuration file not found."));
        }

        // Attempt to open the configuration file
        let config_file = File::open(path.clone()).await.context("Unable to open configuration file.")?;

        // Attempt to load the configuration from the file
        let config = Config::from_config(config_file).await?;

        // Create the new player status handler and load existing data
        let player_status = PlayerHandler::new(config.get_identifier(), config.get_server_location()).await;

        // Create the new puzzle status handler
        let puzzle_status = PuzzleHandler::new(config.get_game_map());

        // Return the completed database
        let minerva_controllers = config.get_minerva_controllers();
        Ok((Database {
            config,
            player_status,
            puzzle_status,
        },
        minerva_controllers))
    }

    /// A method to create a new player
    /// 
    pub async fn create_player(&mut self, player_id: PlayerId) -> Result<()> {
        // Forward to the player status handler
        self.player_status.create_player(player_id).await
    }

    /// A method to subscribe this user to any changes with a particular player
    /// 
    /// # Note
    /// 
    /// Only one user can be subscribed to a player at a time. Adding a new
    /// subscriber replaces the old one.
    /// 
    pub fn add_listener(&mut self, player_id: PlayerId, sender: mpsc::Sender<AllScores>) {
        // Pass the sender to the player status handler
        self.player_status.add_listener(player_id, sender)
    }

    /// A method to retrieve the current scores for this player
    /// 
    pub async fn get_all_scores(&mut self, player_id: PlayerId) -> Result<AllScores> {
        // Retrieve the player scores
        self.player_status.get_all_scores(player_id).await
    }

    /// A method to reset a particular puzzle. This method should
    /// only be used if you know the puzzle was available to start,
    /// but Minerva could not be reached.
    /// 
    pub async fn reset_puzzle(&mut self, unique_puzzle: UniquePuzzle) -> Result<()> {
        // Try to reset the puzzle to available
        self.puzzle_status.reset_puzzle(unique_puzzle)
    }

    /// A method to start a particular puzzle
    /// 
    pub async fn start_puzzle(&mut self, unique_puzzle: UniquePuzzle, player_id: PlayerId) -> Result<ItemId> {
        // Verify that the player is valid
        self.player_status.verify_player(&player_id).await?;

        // Try to make the player the current player
        self.puzzle_status.start_puzzle(unique_puzzle, player_id)
    }

    /// A method to verify that a particular player exists
    /// 
    pub async fn verify_player(&mut self, player_id: PlayerId) -> Result<()> {
        // Verify that the player is valid
        self.player_status.verify_player(&player_id).await
    }

    /// A method to verify that a player exists and is the current player
    /// for a pariticular puzzle
    /// 
    pub async fn verify_current_player(&mut self, unique_puzzle: &UniquePuzzle, player_id: &PlayerId) -> Result<()> {
        // Verify that the player is valid
        self.player_status.verify_player(player_id).await?;
        
        // Verify that this player is the current player
        self.puzzle_status.verify_current_player(unique_puzzle, player_id)?;
        
        // TODO Return a line tracking puzzle updates
        Ok(())
    }
}

