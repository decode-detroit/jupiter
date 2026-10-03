// Copyright (c) 2024 Decode Detroit
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

//! A module to load and play video and audio files on this device

// Import crate definitions
use crate::definitions::*;

// Import standard library features
use std::collections::HashMap;
use std::collections::hash_map::Entry::{Occupied, Vacant};

// Import standard library features
use std::path::PathBuf;

// Import tokio elements
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::time::{Duration, sleep};

// Import reqwest and websocket elements
use reqwest::Client;
use reqwest_websocket::{Upgrade, Message};

// Import futures util features
use futures_util::StreamExt;

/// A structure to define a cue event for communicating with Minerva
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FullCueEvent {
    id: u32,
    secs: u64,
    nanos: u64,
}

/// A structure to hold and manage a Minerva thread
///
struct MinervaThread;

// Implement the MinervaThread Functions
impl MinervaThread {
    /* FIXME Reenable this will all Minerva options
    /// Spawn a copy of minerva and the monitoring thread
    async fn spawn(
        mut receiver: mpsc::Receiver<ItemId>,
        path: PathBuf,
        address: String,
        backup_location: Option<String>,
    ) {
        // Notify that the background process is starting
        info!("Starting Minerva Game controller ...");

        // Compose the arguments for the command
        let mut arguments = vec![
            "-p".into(),
            path.to_str().unwrap_or("").into(),
            "-a".into(),
            address.clone(),
        ];

        // Add the backup location if specified
        if let Some(location) = backup_location {
            arguments.push("-b".into());
            arguments.push(location);
        }

        // Create the child process
        let mut child = match Command::new("vulcan").args(&arguments).spawn() {
            // If the child process was created, return it
            Ok(child) => child,

            // Otherwise, try again in the local directory
            _ => {
                // Try looking in the local directory
                match Command::new("./vulcan").args(&arguments).spawn() {
                    // If the child process was created, return it
                    Ok(child) => child,

                    // Otherwise, warn of the error and return
                    _ => {
                        error!("Unable to start Vulcan DMX controller.");
                        return;
                    }
                }
            }
        };

        // Create a client for passing dmx information
        let client = match Client::builder().timeout(Duration::from_secs(10)).build() {
            // On error close the monitoring thread
            Err(_) => {
                error!("Unable to create Vulcan communication client.");
                return;
            }

            // Otherwise, continue
            Ok(client) => client,
        };

        // Wait a second for the server to start
        sleep(Duration::from_secs(1)).await;

        // Spawn a background thread to monitor the process
        tokio::spawn(async move {
            // Run indefinitely or until the process fails
            loop {
                // Wait for a message, the process to finish, or the sender to be poisoned
                tokio::select! {
                    // The process has finished
                    result = child.wait() => {
                        match result {
                            // Notify that the process has terminated
                            Ok(_) => error!("Vulcan DMX controller stopped."),

                            // If the process failed to run
                            _ => {
                                error!("Unable to run Vulcan DMX controller.");
                                break;
                            }
                        }
                    }

                    // A message was received (or the line dropped)
                    possible_fade = receiver.recv() => {
                        // If a fade was received
                        if let Some(fade) = possible_fade {
                            // Recompose the dmx fade into a helper
                            let helper: DmxFadeHelper = fade.into();

                            // Pass the dmx fade on to Vulcan
                            if let Err(err) = client.post(format!("http://{}/playFade", address)).json(&helper).send().await {
                                error!("Error with DMX Fade: {}", err);
                            };

                            // Start listening again for more messages
                            continue;

                        // Otherwise, the sending line has been dropped
                        } else {
                            // Notify of the closure
                            info!("Closing Vulcan DMX controller ...");

                            // Tell Vulcan to close
                            let _ = client.post(format!("http://{}/close", address)).send().await;

                            // Exit the loop and close the background thread
                            break;
                        }
                    }
                }

                // Wait several seconds to restart the server
                sleep(Duration::from_secs(2)).await;

                // Notify that the background process is restarting
                info!("Restarting Vulcan DMX contoller ...");

                // Start the process again
                child = match Command::new("vulcan").args(&arguments).spawn() {
                    // If the child process was created, return it
                    Ok(child) => child,

                    // Otherwise, try again in the local directory
                    _ => {
                        // Try looking in the local directory
                        match Command::new("./vulcan").args(&arguments).spawn() {
                            // If the child process was created, return it
                            Ok(child) => child,

                            // Otherwise, warn of the error and return
                            _ => {
                                error!("Unable to start Vulcan DMX controller.");
                                break;
                            }
                        }
                    }
                };

                // Wait a second for the server to start
                sleep(Duration::from_secs(1)).await;
            }
        });
    }*/

    /// Spawn the monitoring thread only
    async fn no_spawn(mut receiver: mpsc::Receiver<ItemId>, address: String, game_id: GameId, minerva_send: MinervaSend) {
        // Notify that the background process is starting
        info!("Connecting to Minerva Game controller ...");

        // Create a client for passing event information
        let client = match Client::builder().timeout(Duration::from_secs(10)).build() {
            // On error close the monitoring thread
            Err(_) => {
                error!("Unable to create Minerva communication client.");
                return;
            }

            // Otherwise, continue
            Ok(client) => client,
        };

        // Spawn a background thread to communicate
        let client_clone = client.clone();
        let address_clone = address.clone();
        tokio::spawn(async move {
            // Run indefinitely or until the line is closed
            loop {
                // If a event was received
                if let Some(event_id) = receiver.recv().await {
                    // Compose the full cue event
                    let cue_event = FullCueEvent {
                        id: event_id.id(),
                        secs: 0,
                        nanos: 0,
                    };

                    // Pass the dmx fade on to Minerva
                    if let Err(err) = client
                        .post(format!("http://{}/cueEvent", address))
                        .json(&cue_event)
                        .send()
                        .await
                    {
                        error!("Error with Cue Event: {}", err);
                    };

                    // Start listening again for more messages
                    continue;

                // Otherwise, the sending line has been dropped
                } else {
                    // Notify of the closure
                    info!("Disconnecting from Minerva Game controller ...");

                    // Exit the loop and close the background thread
                    break;
                }
            }
        });

        // Request a websocket to listen for status changes from Minerva
        let response = match client_clone.get(format!("http://{}/listen", address_clone)).upgrade().send().await {
            Ok(response) => response,
            Err(err) => {
                error!("Unable to listen to Minerva with id {}: {}", game_id, err);
                return;
            }
        };

        // Try to create the new websocket
        let mut websocket = match response.into_websocket().await {
            Ok(socket) => socket,
            Err(err) => {
                error!("Unable to listen to Minerva with id {}: {}", game_id, err);
                return;
            }
        };

        // Spawn another thread to receive, process, and forward updates
        tokio::spawn(async move {
            // Loop and forward messages until an error is encountered
            while let Some(result) =  websocket.next().await {
                if let Ok(message) = result {
                    // Interpret string messages
                    if let Message::Text(string) = message {
                        // Try to serialize the message into an update
                        if let Ok(update) = serde_json::from_str::<MinervaUpdate>(&string) {
                            // Send the update to Jupiter
                            minerva_send.send(game_id.clone(), update).await;
                        } else {
                            error!("Unable to parse message from Minerva id {}", game_id.clone());
                        }
                    
                    // Warn that we got an invalid message type
                    } else {
                        error!("Received an invalid message type from Minerva id {}", game_id.clone());
                    }

                // Otherwise, leave the loop
                } else {
                    error!("Lost connection to Minerva id {}", game_id);
                    break;
                }
            }   
        });
    }
}

/// A structure to hold and manipulate the connection to the minerva backend
///
struct MinervaInterface {
    sender: mpsc::Sender<ItemId>, // a line to pass events to the background thread. The line is poisoned when this structure is dropped
}

// Implement key functionality for the Minerva Interface structure
impl MinervaInterface {
    /// A function to create a new instance of the Minerva interface
    ///
    async fn new(minerva_params: MinervaParams, minerva_send: MinervaSend) -> Self {
        // Copy the specified address or use the default
        let address = minerva_params
            .address
            .clone()
            .unwrap_or(String::from("127.0.0.1:64636")); // TODO: Add a version for the secured address using jwt

        // Create a channel to notify the background thread to close
        let (sender, receiver) = mpsc::channel(512);

        // Spin out thread to monitor and restart vulcan, if requested
        /*if vulcan_params.spawn {
            VulcanThread::spawn(
                receiver,
                vulcan_params.path.unwrap_or_default(),
                address,
                backup_location,
            )
            .await;

        // Otherwise, just spin the background thread for communication
        } else {*/
        MinervaThread::no_spawn(receiver, address, minerva_params.game_id, minerva_send).await;
        //}

        // Return the complete module
        Self { sender }
    }

    /// A method to send a new event to the game controller
    ///
    /// This method passes the request to the background thread for processing.
    /// If the request fails, the error will be passed through the tracing library
    /// from the background thread.
    ///
    async fn cue_event(&self, event_id: ItemId) {
        // Send the dmx fade to the background thread
        self.sender.send(event_id).await.unwrap_or(());
    }
}

/// A structure to hold and manipulate one or more Minerva Interfaces
/// and access them by game id
///
pub struct MinervaHandler {
    interface_map: HashMap<GameId, Vec<MinervaInterface>>, // a map of minerva interfaces
}

// Implement key functionality for the Minerva handler structure
impl MinervaHandler {
    /// A function to create a new instance of the Minerva handler
    ///
    pub async fn new(mut minerva_controllers: MinervaControllers, minerva_send: MinervaSend) -> Self {
        // Create an empty game map
        let mut interface_map: HashMap<GameId, Vec<MinervaInterface>> = HashMap::default();

        // For each of the Minerva controllers, spin off a new interface
        for params in minerva_controllers.drain(..) {
            // Create a new interace for each one
            let game_id = params.game_id.clone();
            let interface = MinervaInterface::new(params, minerva_send.clone()).await;

            // Add it to the map
            match interface_map.entry(game_id) {
                // Add it to an existing list of interfaces
                Occupied(mut entry) => {
                    entry.get_mut().push(interface);
                }

                // Or create a new list and add it
                Vacant(entry) => {
                    entry.insert(vec![interface]);
                }
            }
        }

        // Return the complete module
        Self { interface_map }
    }

    /// A method to send a new event to the game controller
    ///
    /// # Errors
    /// 
    /// If the specified game id does not have a matching controller, this
    /// method will throw an error.
    /// 
    /// If the request fails to reach the controller, the error will be
    /// passed through the tracing library.
    ///
    pub async fn cue_event(&mut self, game_id: GameId, event_id: ItemId) -> Result<()> {
        // Verify that the game has at least one matching interface
        match self.interface_map.entry(game_id) {
            Occupied(interfaces) => {
                // Send the event to each interface
                for interface in interfaces.get().iter() {
                    interface.cue_event(event_id).await;
                }

                // Indicate success
                Ok(())
            }
            
            // Otherwise, throw an error
            _ => {
                return Err(anyhow!("Game does not have any matching interface."));
            }
        }
    }
}
