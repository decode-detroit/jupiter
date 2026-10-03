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

//! A module to load the configuration from a file and maintain the state
//! machine. This module handles any changes to the current state of the
//! program.

// Import crate definitions
use crate::definitions::*;

// Import tokio features
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// The configuration struct that is designed to allow simple
/// serialization and deserialization for the program configuration file.
/// This structure is saved in the external configuration file.
///
#[derive(Serialize, Deserialize)]
struct YamlConfig {
    version: String,        // a version tag to warn the user of incompatible versions
    identifier: Identifier, // unique identifier for the controller instance, if specified
    server_location: Option<String>, // the location of the backup server, if specified
    game_map: AllGames, // the definition of all games and the puzzles they contain
    minerva_controllers: MinervaControllers, // a list of game controllers
}

/// A structure to hold the whole configuration for current instantiation of the
/// program. This structure is passive and does not actively manage any state.
///
pub struct Config {
    identifier: Identifier, // unique identifier for the controller instance, if specified
    server_location: Option<String>, // the location of the backup server, if specified
    game_map: AllGames, // the definition of all games and the puzzles they contain
    minerva_controllers: MinervaControllers, // a list of game controllers
}

// Implement key features for the configuration
impl Config {
    /// A function to create a new config from a configuration file
    ///
    /// This function uses a file to fill out the game configuration. The
    /// the format of the configuration file is YAML (http://yaml.org/) and must
    /// match the structure of the private YamlConfig structure.
    ///
    /// # Errors
    ///
    /// This function will raise an error if it is unable to parse the
    /// configuration file.
    ///
    pub async fn from_config(
        mut config_file: File,
    ) -> Result<Config> {
        // Try to read from the configuration file
        let mut config_string = String::new();
        match config_file.read_to_string(&mut config_string).await {
            Ok(_) => (),
            Err(error) => {
                error!("Invalid configuration file: {}.", error);
                return Err(anyhow!("Invalid configuration file: {}", error));
            }
        }

        // Try to parse the configuration file
        let yaml_config: YamlConfig = match serde_yaml::from_str(&config_string) {
            Ok(config) => config,
            Err(error) => {
                error!("Unable to parse configuration file: {}.", error);
                return Err(anyhow!("Unable to parse configuration file: {}", error));
            }
        };

        // Check the version id and warn the user if they differ
        let version = env!("CARGO_PKG_VERSION");
        if yaml_config.version != version {
            warn!(
                "Version of configuration ({}) does not match software version ({}).",
                &yaml_config.version, version
            );
        }

        // Return the new configuration
        Ok(Config {
            identifier: yaml_config.identifier,
            server_location: yaml_config.server_location,
            game_map: yaml_config.game_map,
            minerva_controllers: yaml_config.minerva_controllers,
        })
    }

    /// A method to return the identifier
    ///
    pub fn get_identifier(&self) -> Identifier {
        self.identifier
    }

    /// A method to return the server location
    ///
    pub fn get_server_location(&self) -> Option<String> {
        self.server_location.clone()
    }

    /// A method to return a copy of the game map
    ///
    pub fn get_game_map(&self) -> AllGames {
        self.game_map.clone()
    }

    /// A method to return a copy of the Minerva controllers
    ///
    pub fn get_minerva_controllers(&self) -> MinervaControllers {
        self.minerva_controllers.clone()
    }

    /// A method to write the current configuration to a file.
    ///
    /// # Errors
    ///
    /// This function will raise an error if the current configuration is broken
    /// or the provided file was not usable. This usually indicates a problem
    /// the provided file type.
    ///
    #[allow(dead_code)]
    pub async fn to_config(&self, mut config_file: File) {
        // Create a YAML config from the elements
        let yaml_config = YamlConfig {
            version: env!("CARGO_PKG_VERSION").into(),
            identifier: self.get_identifier(),
            server_location: self.server_location.clone(),
            game_map: self.game_map.clone(),
            minerva_controllers: self.minerva_controllers.clone(),
        };

        // Try to parse the configuration
        let config_string = match serde_yaml::to_string(&yaml_config) {
            Ok(config_string) => config_string,
            Err(error) => {
                error!("Unable to parse current configuration: {}.", error);
                return;
            }
        };

        // Try to write the configuration to the file
        match config_file.write_all(config_string.as_bytes()).await {
            Ok(_) => (),
            Err(error) => {
                error!("Unable to write configuration file: {}.", error)
            }
        }
    }
}

// Tests of the scene module
#[cfg(test)]
mod tests {
    use fnv::FnvHashMap;

    use super::*;

    /* Generate example config 
    #[tokio::test]
    async fn example_config() {
        // Create the yaml config
        let mut score_map = FnvHashMap::default();
        score_map.insert(ItemId::new_unchecked(2000), 300);
        let mut game = Game::default();
        game.puzzles.insert(PuzzleId::new_unchecked(1000), Puzzle {
            current_state: ItemId::new_unchecked(1001),
            available_state: ItemId::new_unchecked(1002),
            starting_state: ItemId::new_unchecked(1003),
            starting_event: ItemId::new_unchecked(1004),
            score_map,
            current_player: None,
        });
        let mut game_map = AllGames::default();
        game_map.insert(GameId::new("game1").unwrap(), game);
        let config = Config {
            identifier: Identifier {
                id: Some(0),
            },
            server_location: Some("redis://127.0.0.1:6379".to_string()),
            game_map,
            minerva_controllers: vec![],
        };

        // Open a file
        let mut path = std::env::current_dir().unwrap();
        path.push("config_example.yaml");
        let config_file = File::create(&path).await.context("Unable to open configuration file.").unwrap();

        // Try to write to a file
        config.to_config(config_file).await;
    }*/

    // FIXME Define tests of this module
    #[tokio::test]
    async fn missing_tests() {
        // FIXME: Implement this
        unimplemented!();
    }
}
