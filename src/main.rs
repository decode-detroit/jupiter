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

//! The main module of the jupiter program which pulls from the other modules.

// Allow deeper recursion testing for web server
#![recursion_limit = "256"]

// Define program modules
#[macro_use]
mod definitions;
mod database;
mod web_interface;

// Import crate definitions
use crate::definitions::*;

// Import other structures into this module
use self::web_interface::WebInterface;
use self::system_interface::SystemInterface;

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
struct Jupiter;

// Implement the Jupiter functionality
impl Jupiter {
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

        // Return and file guard
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

        // Create the update send and receive
        let (update_send, update_recv) = PlayerSend::new();

        // Launch the system interface to monitor and handle events
        let (system_interface, web_send) = SystemInterface::new(
            update_send.clone(),
            arguments.config,
        )
        .await;

        // Launch the web interface (creates its own threads)
        WebInterface::launch(
            web_send,
            update_recv,
            arguments.address,
            arguments.cors_allowed_addr,
            arguments.cert_path,
            arguments.key_path,
            arguments.jwt_secret,
        )
        .await;

        // Block on the system interface
        system_interface.run().await // return the shutdown variable
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
