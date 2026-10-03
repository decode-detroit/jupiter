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

//! The main module of the jupiter program which pulls from the other modules
//! and handles communicaiton between the modules.

// Allow deeper recursion testing for web server
#![recursion_limit = "256"]

// Define program modules
#[macro_use]
mod definitions;
mod database;
mod minerva_interface;
mod web_interface;

// Import crate definitions
use crate::definitions::*;

// Import other structures into this module
use self::database::Database;
use self::minerva_interface::MinervaHandler;
use self::web_interface::WebInterface;

// Import Tokio features
use tokio::sync::mpsc;

// Import anyhow features
#[macro_use]
extern crate anyhow;

// Import tracing features
use tracing::Level;
use tracing_subscriber::filter::{LevelFilter, filter_fn};
use tracing_subscriber::prelude::*;

// Import clap features
use clap::Parser;

// Import single instance features
use single_instance::SingleInstance;

// Import shutdown features
use system_shutdown::shutdown;

/// Struct to hold the optional arguments for Minerva
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Arguments {
    /// Relative path to a configuration file
    #[arg(short, long, default_value = DEFAULT_FILE)]
    config: String,

    /// Flag to allow for multiple instances
    #[arg(short = 'm', long, default_value = "false")]
    allow_multiple: bool,

    /// Address for the web interface
    #[arg(long, default_value = DEFAULT_ADDRESS)]
    address: String,

    /// CORS allowed addresses
    #[arg(long, default_value = None)] // default is to allow any origin
    cors_allowed_addr: Option<Vec<String>>,

    /// TLS certificate location for the address
    #[arg(long, default_value = None)]
    cert_path: Option<String>,

    /// TLS private key location for the address
    #[arg(long, default_value = None)]
    key_path: Option<String>,

    /// JWT secret for the limited access address
    #[arg(long, default_value = None)]
    jwt_secret: Option<String>,

    /// Flag to set the log level
    #[arg(short, long, default_value = DEFAULT_LOGLEVEL)]
    log_level: String,
}

/// The Jupiter structure to contain the program launching and overall
/// communication code.
///
struct Jupiter {
    jupiter_recv: mpsc::Receiver<WebRequest>,
    minerva_recv: mpsc::Receiver<GameUpdate>,
    database: Database,
    minerva_handler: MinervaHandler,
}

// Implement the Jupiter functionality
impl Jupiter {
    /// A method to return a new instance of Jupiter
    /// 
    async fn new(config_file: &str) -> Result<(Self, JupiterSend)> {
        // Create the jupiter request send and receive
        let (jupiter_send, jupiter_recv) = JupiterSend::new();

        // Create the minerva update send and receive
        let (minerva_send, minerva_recv) = MinervaSend::new();

        // Create the puzzle, game and player database from the configuration
        let (database, minerva_controllers) = Database::new(config_file).await?;

        // Create the Minerva connections, as needed
        let minerva_handler = MinervaHandler::new(minerva_controllers, minerva_send).await;

        // Return the new instance with other communication elements
        Ok((Self { jupiter_recv, minerva_recv, database, minerva_handler }, jupiter_send ))
    }
    
    /// A function to setup the logging configuration
    ///
    fn setup_logging(log_string: String) -> tracing_appender::non_blocking::WorkerGuard {
        // Try to convert the string to a log level
        let log_level = match log_string.as_str() {
            "Trace" => LevelFilter::TRACE,
            "Debug" => LevelFilter::DEBUG,
            "Info" => LevelFilter::INFO,
            "Warn" => LevelFilter::WARN,
            "Error" => LevelFilter::ERROR,

            // Otherwise, print a nice error
            _ => {
                println!(
                    "Unable to parse parameter for option 'logLevel'. Options are Trace, Debug, Info, Warn, and Error."
                );
                LevelFilter::INFO
            }
        };

        // Create the stdout layer
        let stdout_layer = tracing_subscriber::fmt::layer()
            .with_target(false)
            .with_filter(log_level);

        // Create a user interface layer FIXME collect warning and error messages to display to the user
        //let buffer = Mutex::new(Writer::new(Arc::new(Vec::new()))); //Arc::new()
        //let buf_writer = buffer.clone().make_writer();
        //let user_layer = tracing_subscriber::fmt::layer().with_writer(buffer).with_ansi(false).with_target(false).with_filter(log_level);

        // Create the log file
        let file_appender = tracing_appender::rolling::daily(LOG_FOLDER, ERROR_LOG);
        let (non_blocking, file_guard) = tracing_appender::non_blocking(file_appender);

        // Create the log file filter
        let file_filter = {
            filter_fn(|metadata| metadata.target() == ERROR_LOG || metadata.level() == &Level::ERROR)
        };

        // Create the log file layer
        let file_layer = tracing_subscriber::fmt::layer()
            .with_writer(non_blocking)
            .with_ansi(false)
            .with_target(false)
            .with_filter(file_filter);

        // Initialize tracing
        tracing_subscriber::registry()
            .with(stdout_layer)
            .with(file_layer)
            .init();

        // Return a file guard
        file_guard
    }

    /// A function to build and run the main program
    ///
    /// This function returns true if the user requested the computer to shut down.
    ///
    async fn run(arguments: Arguments) -> bool {
        // Initialize logging (guard is held until the end of run())
        #[cfg(not(feature = "tokio_console"))]
        let _guard = Jupiter::setup_logging(arguments.log_level);

        // Create the new Jupiter instance
        let (mut jupiter, jupiter_send) = match Jupiter::new(&arguments.config).await {
            Ok(tuple) => tuple,
            Err(err) => {
                error!("Unable to initialize Jupiter: {}", err);
                return false;
            }
        };

        // Launch the web interface (creates its own threads)
        WebInterface::launch(
            jupiter_send,
            arguments.address,
            arguments.cors_allowed_addr,
            arguments.cert_path,
            arguments.key_path,
            arguments.jwt_secret,
        )
        .await;

        // Loop indefinitely waiting for the program to be closed by the user
        let is_shutdown;
        loop {
            // Repeat endlessly until run_once reaches close
            if let Err(shutdown) = jupiter.run_once().await {
                is_shutdown = shutdown;
                break;
            }
        }

        // Drop all associated data in jupiter
        drop(jupiter);

        // Return the shutdown variable
        is_shutdown
    }

    /// A method to run one iteration of the program to update the user
    /// and process any outstanding requests.
    ///
    /// This method returns Ok(()) if the program should continue running, and
    /// an Error if it should close. If Error(true), the user has requested the
    /// computer shut down as well.
    ///
    async fn run_once(&mut self) -> Result<(), bool> {
        // Check for updates on any line
        tokio::select! {
            // Update from Minerva
            Some(update) = self.minerva_recv.recv() => {
                // Match the update type
                match update.update {
                    // Possibly update the state of a puzzle
                    MinervaUpdate::UpdateStatus { status_id, new_state } => {
                        // Compose the unique puzzle
                        let unique_puzzle = UniquePuzzle {
                            game_id: update.game_id,
                            puzzle_id: status_id, // may not actually be a puzzle
                        };

                        // Try to update the state of that puzzle
                        self.database.change_state(unique_puzzle, new_state).await;
                    },

                    // Ignore all others
                    _ => (),
                }
            }

            // Requests from the web
            Some(request) = self.jupiter_recv.recv() => {
                // Match the request type
                match request.request {
                    // Execute the close request
                    Request::Close => {
                        request.reply_to.send(Reply::success()).unwrap_or(());
                        return Err(false); // exit the loop, but don't shutdown
                    }

                    // Create a new player id (or do nothing, if the player exists)
                    Request::CreatePlayer { player_id } => {
                        match self.database.create_player(player_id).await {
                            Ok(()) => request.reply_to.send(Reply::success()).unwrap_or(()),
                            Err(err) => request.reply_to.send(Reply::failure(format!("{}", err))).unwrap_or(()),
                        }
                    }

                    // Cue an event for the selected game and puzzle
                    Request::CueEvent { player_id, unique_puzzle, event_id } => {
                        // Verify the current player
                        if let Err(err) = self.database.verify_current_player(&unique_puzzle, &player_id).await {
                            request.reply_to.send(Reply::failure(format!("{}", err))).unwrap_or(());
                        
                        // Otherwise, try to forward the event
                        } else {
                            if self.minerva_handler.cue_event(unique_puzzle.game_id, event_id).await.is_ok() {
                                request.reply_to.send(Reply::success()).unwrap_or(());
                            
                            // Notify the user of failure
                            } else {
                                // Notify of the failure
                                request.reply_to.send(Reply::failure(format!("Unable to contact Minerva."))).unwrap_or(());
                            }
                        }
                    }

                    // Get the current player scores
                    Request::PlayerScores { player_id } => {
                        match self.database.get_all_scores(player_id).await {
                            Ok(scores) => request.reply_to.send(Reply { is_valid: true, data: ReplyData::Scores(scores) }).unwrap_or(()),
                            Err(err) => request.reply_to.send(Reply::failure(format!("{}", err))).unwrap_or(()),
                        }
                    }

                    // Subscribe the player to any updates to this player scores
                    Request::PlayerScoreUpdates { player_id, sender } => {
                        match self.database.add_listener(player_id, sender).await {
                            Ok(()) => request.reply_to.send(Reply::success()).unwrap_or(()),
                            Err(err) => request.reply_to.send(Reply::failure(format!("{}", err))).unwrap_or(()),
                        }
                    }

                    // Execute the shutdown request
                    Request::Shutdown => {
                        request.reply_to.send(Reply::success()).unwrap_or(());
                        return Err(true); // exit the loop and shutdown
                    }

                    // Start the selected puzzle if it is available
                    Request::StartPuzzle { player_id, unique_puzzle } => {
                        match self.database.start_puzzle(unique_puzzle.clone(), player_id).await {
                            // If the puzzle was started by this player
                            Ok(starting_event) => {
                                // Update the puzzle state using the starting event
                                if self.minerva_handler.cue_event(unique_puzzle.game_id.clone(), starting_event).await.is_ok() {
                                    request.reply_to.send(Reply::success()).unwrap_or(());
                                
                                // On failure, reset the puzzle
                                } else {
                                    // Ignore the result
                                    let _ = self.database.reset_puzzle(unique_puzzle.clone());

                                    // Notify of the failure
                                    request.reply_to.send(Reply::failure(format!("Unable to contact Minerva."))).unwrap_or(());
                                }
                                
                            },
                            Err(err) => request.reply_to.send(Reply::failure(format!("{}", err))).unwrap_or(()),
                        }
                    }

                    // Verify that the specified player exists
                    Request::VerifyPlayer { player_id } => {
                        match self.database.verify_player(player_id).await {
                            Ok(()) => request.reply_to.send(Reply::success()).unwrap_or(()),
                            Err(err) => request.reply_to.send(Reply::failure(format!("{}", err))).unwrap_or(()),
                        }
                    }

                    // Verify that the specified player is the current player
                    Request::VerifyCurrentPlayer { player_id, unique_puzzle } => {
                        match self.database.verify_current_player(&unique_puzzle, &player_id).await {
                            Ok(()) => request.reply_to.send(Reply::success()).unwrap_or(()),
                            Err(err) => request.reply_to.send(Reply::failure(format!("{}", err))).unwrap_or(()),
                        }
                    }
                }
            }
        }

        // In most cases, indicate to continue normally
        Ok(())
    }
}

/// The main function of the program, simplified to as high a level as possible.
///
#[tokio::main]
async fn main() {
    // Get the commandline arguments
    let arguments = Arguments::parse();

    // Start the console subscriber
    #[cfg(feature = "tokio_console")]
    console_subscriber::init();

    // Create a single instance marker
    let is_shutdown;
    if let Ok(instance) = SingleInstance::new("jupiter") {
        // If not allowing multiple instances and this isn't the only instance
        if !arguments.allow_multiple && !instance.is_single() {
            println!("Jupiter is already running. Exiting ...");
            return;
        }

        // Create the program and run until directed otherwise
        is_shutdown = Jupiter::run(arguments).await;

    // If unable to create the marker, warn the user
    } else {
        println!("Unable to verify if this is the only instance of Jupiter.");

        // Create the program and run until directed otherwise
        is_shutdown = Jupiter::run(arguments).await;
    }

    // Check to see if a shutdown was requested
    if is_shutdown {
        match shutdown() {
            Ok(_) => println!("Shutting down computer ..."),
            Err(error) => eprintln!("Unable to shut down computer: {}", error),
        }
    }
}
