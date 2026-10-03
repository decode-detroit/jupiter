// Copyright (c) 2020-2021 Decode Detroit
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

//! A module to create the web interface to interface to connect the web UI
//! and endpoints to the program.

// Import crate definitions
use crate::definitions::*;

// Define private submodules
mod web_definitions;

// Import the web definitions
use self::web_definitions::*;

// Import Tokio and warp features
use tokio::fs::read;
use tokio::sync::{mpsc, oneshot};
use warp::ws::WebSocket;
use warp::{Filter, http};

// Import stream-related features
use async_stream::stream;
use futures_util::StreamExt;

// Import serde features
use serde::de::DeserializeOwned;

// Import JWT features
use jsonwebtoken as jwt;
use jwt::{decode, DecodingKey, EncodingKey, TokenData, Validation};

// Define validity period for new tokens
const TOKEN_DURATION: u64  = 43200; // 12 hours in seconds

/// A structure to contain the web interface and handle all updates to the
/// to the interface.
///
pub struct WebInterface;

// Implement key Web Interface functionality
impl WebInterface {
    /// A method to launch the web interface.
    /// The interface will listen for connections from the internet and send messages back.
    ///
    pub async fn launch(
        jupiter_send: JupiterSend,
        addr: String,
        cors_allowed_addr: Option<Vec<String>>,
        possible_cert_path: Option<String>,
        possible_key_path: Option<String>,
        possible_jwt_secret: Option<String>,
    ) {
        // Parse any provided addresses, or use defaults
        let possible_address = addr.parse::<std::net::SocketAddr>();

        // If the address is valid
        if let Ok(address) = possible_address {
            // Spin up a thread for processing requests
            let clone_send = jupiter_send.clone();
            tokio::spawn(async move {
                // Create the CORS filter to allow a specific origin
                let mut cors;
                if let Some(cors_addresses) = cors_allowed_addr {
                    // Create the header
                    cors = warp::cors();

                    // Add any specified address
                    for address in cors_addresses {
                        cors = cors.allow_origin(address.as_str());
                    }

                    // Add the authentication options
                    cors = cors.allow_headers(vec!["authorization"]);

                    // Specify relevant methods
                    cors = cors.allow_methods(vec!["GET", "POST"]);

                // Otherwise, default to allow any origin
                } else {
                    cors = warp::cors()
                        .allow_any_origin()
                        .allow_methods(vec!["GET", "POST"]);
                }

                // If a TLS certificate and private key were provided
                let mut is_using_tls = false;
                if let (Some(cert_path), Some(private_path)) =
                    (possible_cert_path, possible_key_path)
                {
                    // Try to load the certificate and private keys from the path
                    let possible_cert = read(cert_path).await;
                    let possible_private = read(private_path).await;

                    // If both files loaded successfully
                    if let (Ok(certificate), Ok(private_key)) = (possible_cert, possible_private) {
                        is_using_tls = true; // save successful TLS loading

                        // If a JWT secret was provided
                        if let Some(secret) = possible_jwt_secret {
                            // Create the JWT encoding and decoding keys
                            let encoding_key = EncodingKey::from_secret(secret.as_bytes());
                            let decoding_key = DecodingKey::from_secret(secret.as_bytes());

                            // Share the admin token to the terminal
                            info!(
                                "Admin Token: {}",
                                WebInterface::generate_admin_token(encoding_key.clone())
                            );

                            // Create the options response
                            let cors_options = warp::options().map(warp::reply).with(cors.clone());

                            // Create the admin close filter
                            let admin_close = warp::post()
                                .and(warp::path("close"))
                                .and(warp::path::end())
                                .and(WebInterface::with_clone(decoding_key.clone()))
                                .and(WebInterface::with_clone(clone_send.clone()))
                                .and(warp::header::<String>("authorization"))
                                .and_then(WebInterface::admin_close);

                            // Create the admin shutdown filter
                            let admin_shutdown = warp::post()
                                .and(warp::path("shutdown"))
                                .and(warp::path::end())
                                .and(WebInterface::with_clone(decoding_key.clone()))
                                .and(WebInterface::with_clone(clone_send.clone()))
                                .and(warp::header::<String>("authorization"))
                                .and_then(WebInterface::admin_shutdown);

                            // Create the admin create player filter
                            let admin_create_player = warp::post()
                                .and(warp::path("createPlayer"))
                                .and(WebInterface::with_clone(decoding_key.clone()))
                                .and(WebInterface::with_clone(encoding_key.clone()))
                                .and(WebInterface::with_clone(clone_send.clone()))
                                .and(warp::header::<String>("authorization"))
                                .and(WebInterface::with_json::<CreatePlayer>())
                                .and(warp::path::end())
                                .and_then(WebInterface::admin_create_player)
                                .with(cors.clone());

                            // Create the authenticated cue event filter
                            let cue_event = warp::post()
                                .and(warp::path("cueEvent"))
                                .and(WebInterface::with_clone(decoding_key.clone()))
                                .and(WebInterface::with_clone(clone_send.clone()))
                                .and(warp::header::<String>("authorization"))
                                .and(WebInterface::with_json::<CueEvent>())
                                .and(warp::path::end())
                                .and_then(WebInterface::auth_handle_request)
                                .with(cors.clone());

                            // Create the authenticated player scores filter
                            let player_scores = warp::get()
                                .and(warp::path("playerScores"))
                                .and(WebInterface::with_clone(decoding_key.clone()))
                                .and(WebInterface::with_clone(clone_send.clone()))
                                .and(warp::header::<String>("authorization"))
                                .and(WebInterface::with_json::<PlayerScores>())
                                .and(warp::path::end())
                                .and_then(WebInterface::auth_handle_request)
                                .with(cors.clone());

                            // Create the authenticated websocket filter for player updates
                            let player_updates = warp::path("playerUpdates")
                                .and(WebInterface::with_clone(decoding_key.clone()))
                                .and(WebInterface::with_clone(clone_send.clone()))
                                .and(warp::path::param::<String>())
                                .and(warp::path::param::<String>())
                                .and(warp::ws())
                                .map(|key, sender, player_id, token, ws: warp::ws::Ws| {
                                    // This will call the function if the handshake succeeds.
                                    ws.on_upgrade(move |socket| {
                                        WebInterface::auth_player_listener(
                                            key, sender, player_id, token,  socket,
                                        )
                                    })
                                });

                            // Create the authenticated start puzzle filter
                            let start_puzzle = warp::post()
                                .and(warp::path("startPuzzle"))
                                .and(WebInterface::with_clone(decoding_key.clone()))
                                .and(WebInterface::with_clone(clone_send.clone()))
                                .and(warp::header::<String>("authorization"))
                                .and(WebInterface::with_json::<StartPuzzle>())
                                .and(warp::path::end())
                                .and_then(WebInterface::auth_handle_request)
                                .with(cors.clone());

                            // Create the authenticated verify player filter
                            let verify_player = warp::get()
                                .and(warp::path("verifyPlayer"))
                                .and(WebInterface::with_clone(decoding_key.clone()))
                                .and(WebInterface::with_clone(clone_send.clone()))
                                .and(warp::header::<String>("authorization"))
                                .and(WebInterface::with_json::<VerifyPlayer>())
                                .and(warp::path::end())
                                .and_then(WebInterface::auth_handle_request)
                                .with(cors.clone());

                            // Create the authenticated verify player filter
                            let verify_current_player = warp::get()
                                .and(warp::path("verifyCurrentPlayer"))
                                .and(WebInterface::with_clone(decoding_key.clone()))
                                .and(WebInterface::with_clone(clone_send.clone()))
                                .and(warp::header::<String>("authorization"))
                                .and(WebInterface::with_json::<VerifyCurrentPlayer>())
                                .and(warp::path::end())
                                .and_then(WebInterface::auth_handle_request)
                                .with(cors.clone());

                            // Serve these routes
                            warp::serve(
                            cors_options
                                .or(admin_close)
                                .or(admin_shutdown)
                                .or(admin_create_player)
                                .or(cue_event)
                                .or(player_scores)
                                .or(player_updates)
                                .or(start_puzzle)
                                .or(verify_player)
                                .or(verify_current_player)
                            )
                            .tls()
                            .cert(certificate)
                            .key(private_key)
                            .run(address)
                            .await;

                        // Use TLS only, no JWT
                        } else {
                            // Create the options response
                            let cors_options = warp::options().map(warp::reply).with(cors.clone());

                            // Create the create player filter
                            let create_player = warp::post()
                                .and(warp::path("createPlayer"))
                                .and(WebInterface::with_clone(clone_send.clone()))
                                .and(WebInterface::with_json::<CreatePlayer>())
                                .and(warp::path::end())
                                .and_then(WebInterface::handle_request)
                                .with(cors.clone());

                            // Create the cue event filter
                            let cue_event = warp::post()
                                .and(warp::path("cueEvent"))
                                .and(WebInterface::with_clone(clone_send.clone()))
                                .and(WebInterface::with_json::<CueEvent>())
                                .and(warp::path::end())
                                .and_then(WebInterface::handle_request)
                                .with(cors.clone());

                            // Create the player scores filter
                            let player_scores = warp::get()
                                .and(warp::path("playerScores"))
                                .and(WebInterface::with_clone(clone_send.clone()))
                                .and(WebInterface::with_json::<PlayerScores>())
                                .and(warp::path::end())
                                .and_then(WebInterface::handle_request)
                                .with(cors.clone());

                            // Create the websocket filter for player updates
                            let player_updates = warp::path("playerUpdates")
                                .and(WebInterface::with_clone(clone_send.clone()))
                                .and(warp::path::param::<String>())
                                .and(warp::ws())
                                .map(|sender, player_id, ws: warp::ws::Ws| {
                                    // This will call the function if the handshake succeeds.
                                    ws.on_upgrade(move |socket| {
                                        WebInterface::player_listener(
                                            sender, player_id, socket,
                                        )
                                    })
                                });

                            // Create the start puzzle filter
                            let start_puzzle = warp::post()
                                .and(warp::path("startPuzzle"))
                                .and(WebInterface::with_clone(clone_send.clone()))
                                .and(WebInterface::with_json::<StartPuzzle>())
                                .and(warp::path::end())
                                .and_then(WebInterface::handle_request)
                                .with(cors.clone());

                            // Create the verify player filter
                            let verify_player = warp::get()
                                .and(warp::path("verifyPlayer"))
                                .and(WebInterface::with_clone(clone_send.clone()))
                                .and(WebInterface::with_json::<VerifyPlayer>())
                                .and(warp::path::end())
                                .and_then(WebInterface::handle_request)
                                .with(cors.clone());

                            // Create the verify player filter
                            let verify_current_player = warp::get()
                                .and(warp::path("verifyCurrentPlayer"))
                                .and(WebInterface::with_clone(clone_send.clone()))
                                .and(WebInterface::with_json::<VerifyCurrentPlayer>())
                                .and(warp::path::end())
                                .and_then(WebInterface::handle_request)
                                .with(cors.clone());

                            // Serve these routes
                            warp::serve(
                            cors_options
                                .or(create_player)
                                .or(cue_event)
                                .or(player_scores)
                                .or(player_updates)
                                .or(start_puzzle)
                                .or(verify_player)
                                .or(verify_current_player)
                            )
                            .tls()
                            .cert(certificate)
                            .key(private_key)
                            .run(address)
                            .await;
                        }

                    // Fallback to insecure implementation
                    } else {
                        // Throw an error first
                        error!("Unable to serve with TLS: Public or private key file not found.");
                    }
                }

                // Default to no security
                if !is_using_tls {
                    // Create the create player filter
                    let create_player = warp::post()
                        .and(warp::path("createPlayer"))
                        .and(WebInterface::with_clone(clone_send.clone()))
                        .and(WebInterface::with_json::<CreatePlayer>())
                        .and(warp::path::end())
                        .and_then(WebInterface::handle_request)
                        .with(cors.clone());

                    // Create the cue event filter
                    let cue_event = warp::post()
                        .and(warp::path("cueEvent"))
                        .and(WebInterface::with_clone(clone_send.clone()))
                        .and(WebInterface::with_json::<CueEvent>())
                        .and(warp::path::end())
                        .and_then(WebInterface::handle_request)
                        .with(cors.clone());

                    // Create the player scores filter
                    let player_scores = warp::get()
                        .and(warp::path("playerScores"))
                        .and(WebInterface::with_clone(clone_send.clone()))
                        .and(WebInterface::with_json::<PlayerScores>())
                        .and(warp::path::end())
                        .and_then(WebInterface::handle_request)
                        .with(cors.clone());

                    // Create the websocket filter for player updates
                    let player_updates = warp::path("playerUpdates")
                        .and(WebInterface::with_clone(clone_send.clone()))
                        .and(warp::path::param::<String>())
                        .and(warp::ws())
                        .map(|sender, player_id, ws: warp::ws::Ws| {
                            // This will call the function if the handshake succeeds.
                            ws.on_upgrade(move |socket| {
                                WebInterface::player_listener(
                                    sender, player_id, socket,
                                )
                            })
                        });

                    // Create the start puzzle filter
                    let start_puzzle = warp::post()
                        .and(warp::path("startPuzzle"))
                        .and(WebInterface::with_clone(clone_send.clone()))
                        .and(WebInterface::with_json::<StartPuzzle>())
                        .and(warp::path::end())
                        .and_then(WebInterface::handle_request)
                        .with(cors.clone());

                    // Create the verify player filter
                    let verify_player = warp::get()
                        .and(warp::path("verifyPlayer"))
                        .and(WebInterface::with_clone(clone_send.clone()))
                        .and(WebInterface::with_json::<VerifyPlayer>())
                        .and(warp::path::end())
                        .and_then(WebInterface::handle_request)
                        .with(cors.clone());

                    // Create the verify player filter
                    let verify_current_player = warp::get()
                        .and(warp::path("verifyCurrentPlayer"))
                        .and(WebInterface::with_clone(clone_send.clone()))
                        .and(WebInterface::with_json::<VerifyCurrentPlayer>())
                        .and(warp::path::end())
                        .and_then(WebInterface::handle_request)
                        .with(cors.clone());

                    // Serve these routes
                    warp::serve(
                        create_player
                        .or(cue_event)
                        .or(player_scores)
                        .or(player_updates)
                        .or(start_puzzle)
                        .or(verify_player)
                        .or(verify_current_player)
                    )
                    .run(address)
                    .await;
                }
            });
        }
    }

    /// A function to generate an administrator JWT authentication token
    ///
    /// # Note
    /// On failure, this function returns a description of the error
    /// instead of the token.
    ///
    fn generate_admin_token(encoding_key: EncodingKey) -> String {
        // Compose the claims for the token
        let claims = AuthClaims {
            iss: "Jupiter-Admin".into(),
            exp: 0, // Expiration is ignored
        };

        // Try to encode the token
        match jwt::encode(&jwt::Header::default(), &claims, &encoding_key) {
            Ok(token) => token,
            _ => "Unable to generate admin token.".into(),
        }
    }

    /// A function to close Jupiter
    /// (requires the admin token)
    ///
    async fn admin_close(
        key: DecodingKey,
        jupiter_send: JupiterSend,
        token: String,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        // Validate the token
        if let Err(reply) = WebInterface::validate::<AuthClaims>(token, "Jupiter-Admin", &key) {
            // Return early on failure
            return Ok(reply);
        };

        // Send the shutdown message and wait for the reply
        let (reply_to, rx) = oneshot::channel();
        jupiter_send.send(reply_to, Request::Close).await;

        // Wait for the reply
        if let Ok(reply) = rx.await {
            // If the reply is a success
            if reply.is_success() {
                Ok(warp::reply::with_status(
                    warp::reply::json(&reply),
                    http::StatusCode::OK,
                ))

            // Otherwise, note the error
            } else {
                Ok(warp::reply::with_status(
                    warp::reply::json(&reply),
                    http::StatusCode::BAD_REQUEST,
                ))
            }

        // Otherwise, note the error
        } else {
            Ok(warp::reply::with_status(
                warp::reply::json(&Reply::failure("Unable to process request.")),
                http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }

    /// A function to shut down the computer
    /// (requires the admin token)
    ///
    async fn admin_shutdown(
        key: DecodingKey,
        jupiter_send: JupiterSend,
        token: String,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        // Validate the token
        if let Err(reply) = WebInterface::validate::<AuthClaims>(token, "Jupiter-Admin", &key) {
            // Return early on failure
            return Ok(reply);
        };

        // Send the shutdown message and wait for the reply
        let (reply_to, rx) = oneshot::channel();
        jupiter_send.send(reply_to, Request::Shutdown).await;

        // Wait for the reply
        if let Ok(reply) = rx.await {
            // If the reply is a success
            if reply.is_success() {
                Ok(warp::reply::with_status(
                    warp::reply::json(&reply),
                    http::StatusCode::OK,
                ))

            // Otherwise, note the error
            } else {
                Ok(warp::reply::with_status(
                    warp::reply::json(&reply),
                    http::StatusCode::BAD_REQUEST,
                ))
            }

        // Otherwise, note the error
        } else {
            Ok(warp::reply::with_status(
                warp::reply::json(&Reply::failure("Unable to process request.")),
                http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }

    /// A function to create a player (if nedded) with the provided id,
    /// and generate a JWT authentication token associated with the player
    ///
    async fn admin_create_player(
        decoding_key: DecodingKey,
        encoding_key: EncodingKey,
        jupiter_send: JupiterSend,
        token: String,
        possible_request: CreatePlayer,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        // Validate the token
        if let Err(reply) = WebInterface::validate::<AuthClaims>(token, "Jupiter-Admin", &decoding_key) {
            // Return early on failure
            return Ok(reply);
        };
        
        // Compose the claims for the new token
        let claims = PlayerClaims {
            iss: "Jupiter".into(),
            plyr: possible_request.player_id.clone(),
            exp: jwt::get_current_timestamp() + TOKEN_DURATION,
        };

        // Try to create the new player request
        let request = match possible_request.try_into() {
            Ok(request) => request,
            _ => {
                return Ok(warp::reply::with_status(
                    warp::reply::json(&Reply::failure("Invalid player id.")),
                    http::StatusCode::BAD_REQUEST,
                ));
            }
        };

        // Create a oneshot channel to verify creation of the player
        let (reply_to, rx) = oneshot::channel();

        // Send the message and wait for the reply
        jupiter_send.send(reply_to, request).await;

        // If we got a reply
        if let Ok(reply) = rx.await {
            // If the reply is a failure
            if !reply.is_success() {
                // Indicate a bad request
                return Ok(warp::reply::with_status(
                    warp::reply::json(&Reply::failure("Invalid player id.")),
                    http::StatusCode::BAD_REQUEST,
                ));
            }
        
        // Otherwise, note the error
        } else {
            return Ok(warp::reply::with_status(
                warp::reply::json(&Reply::failure("Unable to process request.")),
                http::StatusCode::INTERNAL_SERVER_ERROR,
            ));
        }

        // Encode the token
        let token = match jwt::encode(&jwt::Header::default(), &claims, &encoding_key) {
            Ok(token) => token,
            Err(_) => {
                return Ok(warp::reply::with_status(
                    warp::reply::json(&Reply::failure("Unable to generate new token.")),
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                ));
            }
        };

        // Return the successful token
        Ok(warp::reply::with_status(
            warp::reply::json(&Reply {
                is_valid: true,
                data: ReplyData::Message(token),
            }),
            http::StatusCode::OK,
        ))
    }

    /// A function to process requests from authorized users
    ///
    async fn auth_handle_request(
        key: DecodingKey,
        jupiter_send: JupiterSend,
        token: String,
        possible_request: impl TryInto<Request>,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        // Validate the token and player
        let request = match WebInterface::validate_player_request(token, "Jupiter", &key, possible_request) {
            // Return the data on success
            Ok(data) => data,

            // Return early on failure
            Err(reply) => return Ok(reply),
        };

        // Send the message and wait for the reply
        let (reply_to, rx) = oneshot::channel();
        jupiter_send.send(reply_to, request).await;

        // Wait for the reply
        if let Ok(reply) = rx.await {
            // If the reply is a success
            if reply.is_success() {
                Ok(warp::reply::with_status(
                    warp::reply::json(&reply),
                    http::StatusCode::OK,
                ))

            // Otherwise, note the error
            } else {
                Ok(warp::reply::with_status(
                    warp::reply::json(&reply),
                    http::StatusCode::BAD_REQUEST,
                ))
            }

        // Otherwise, note the error
        } else {
            Ok(warp::reply::with_status(
                warp::reply::json(&Reply::failure("Unable to process request.")),
                http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }

    /// A function to check authentication and then add a new websocket for
    /// player status
    ///
    async fn auth_player_listener(
        key: DecodingKey,
        jupiter_send: JupiterSend,
        possible_player: String,
        token: String,
        socket: WebSocket,
    ) {
        // Validate the token
        let token_data = match WebInterface::validate::<PlayerClaims>(token, "Jupiter", &key) {
            // Return the data on success
            Ok(data) => data,

            // Return early on failure without connecting the socket
            _ => return,
        };

        // Create a player id from the token string
        let token_player_id = match PlayerId::new(&token_data.claims.plyr) {
            Some(id) => id,
            
            // Return early on failure without connecting the socket
            _ => return,
        };

        // Create a player id from the request
        let player_id = match PlayerId::new(&possible_player) {
            Some(id) => id,
            
            // Return early on failure without connecting the socket
            _ => return,
        };

        // If the player ids don't match, return without connecting
        if player_id != token_player_id {
            return;
        }

        // Split the socket into a sender and receiver
        let (ws_tx, mut ws_rx) = socket.split();

        // Use an bounded channel to handle buffering and flushing of messages
        let (tx, mut rx) = mpsc::channel(512);
        let stream = stream! {
            while let Some(item) = rx.recv().await {
                yield item;
            }
        };

        // Forward messages until the line is dropped
        tokio::spawn(
            // Forward received messages
            stream.forward(ws_tx),
        );

        // Create a oneshot channel
        let (reply_to, oneshot_rx) = oneshot::channel();

        // Send the message and wait for the reply
        jupiter_send.send(reply_to, Request::PlayerScoreUpdates {
            player_id,
            sender: SenderWithExpiration {
                socket: tx,
                expiration: token_data.claims.exp,
            }
        }).await;

        // Wait for a sucessful reply
        if let Ok(reply) = oneshot_rx.await && reply.is_success() {
            // Wait for the line to be dropped (ignore incoming messages)
            while ws_rx.next().await.is_some() {}
        }
    }

    /// A function to handle incoming requests
    ///
    async fn handle_request(
        jupiter_send: JupiterSend,
        possible_request: impl TryInto<Request>,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        // Try to create the request from the provided information
        let request = match possible_request.try_into() {
            Ok(request) => request,
            _ => return Ok(warp::reply::with_status(
                    warp::reply::json(&Reply::failure("Invalid request.")),
                    http::StatusCode::BAD_REQUEST,
                )),
        };

        // Send the message and wait for the reply
        let (reply_to, rx) = oneshot::channel();
        jupiter_send.send(reply_to, request).await;

        // Wait for the reply
        if let Ok(reply) = rx.await {
            // If the reply is a success
            if reply.is_success() {
                Ok(warp::reply::with_status(
                    warp::reply::json(&reply),
                    http::StatusCode::OK,
                ))

            // Otherwise, note the error
            } else {
                Ok(warp::reply::with_status(
                    warp::reply::json(&reply),
                    http::StatusCode::BAD_REQUEST,
                ))
            }

        // Otherwise, note the error
        } else {
            Ok(warp::reply::with_status(
                warp::reply::json(&Reply::failure("Unable to process request.")),
                http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }

    /// A function to add a new player listener
    ///
    async fn player_listener(jupiter_send: JupiterSend, possible_player: String, socket: WebSocket) {
        // Create a player id from the possible id
        let player_id = match PlayerId::new(&possible_player) {
            Some(id) => id,
            
            // Return early on failure without connecting the socket
            _ => return,
        };

        // Split the socket into a sender and receiver
        let (ws_tx, mut ws_rx) = socket.split();

        // Use an unbounded channel to handle buffering and flushing of messages
        let (tx, mut rx) = mpsc::channel(512);
        let stream = stream! {
            while let Some(item) = rx.recv().await {
                yield item;
            }
        };

        // Forward messages until the line is dropped
        tokio::spawn(
            // Forward received messages
            stream.forward(ws_tx),
        );

        // Create a oneshot channel
        let (reply_to, oneshot_rx) = oneshot::channel();

        // Send the message and wait for the reply
        jupiter_send.send(reply_to, Request::PlayerScoreUpdates {
            player_id,
            sender: SenderWithExpiration {
                socket: tx,
                expiration: 0,
            }
        }).await;

        // Wait for a sucessful reply
        if let Ok(reply) = oneshot_rx.await && reply.is_success() {
            // Wait for the line to be dropped (ignore incoming messages)
            while ws_rx.next().await.is_some() {}
        }
    }

    // A helper function to extract a helper type from the body of the message
    fn with_json<T>() -> impl Filter<Extract = (T,), Error = warp::Rejection> + Clone
    where
        T: Send + DeserializeOwned,
    {
        // When accepting a body, we want a JSON body (reject large payloads)
        warp::body::content_length_limit(1024 * 16).and(warp::body::json())
    }

    // A helper function to add the web send to the filter
    fn with_clone<T>(
        item: T,
    ) -> impl Filter<Extract = (T,), Error = std::convert::Infallible> + Clone
    where
        T: Send + Clone,
    {
        warp::any().map(move || item.clone())
    }

    // A helper function to validate the provided token against the provided
    // claims and return a "not authorized" reply if the token fails.
    fn validate<T>(token: impl AsRef<[u8]>, issuer: &str, key: &DecodingKey) -> Result<TokenData<T>, warp::reply::WithStatus<warp::reply::Json>>
    where
        T: DeserializeOwned,
    {
        // Create the validation requirements for the token
        let mut validation = Validation::default();
        validation.set_issuer(&[issuer]);
        
        // Decode and validate the token
        return match decode::<T>(token, key, &validation) {
            // Return the decoded data
            Ok(data) => Ok(data),

            // Return an authentication error
            _ => Err(warp::reply::with_status(
                    warp::reply::json(&Reply::failure("Authorization token is invalid.")),
                    http::StatusCode::FORBIDDEN,
                )),
        };
    }

    // A helper function to valiatate the provided player id matches the id
    // provided with the token. Returns the request, if successful.
    fn validate_player_request(token: impl AsRef<[u8]>, issuer: &str, key: &DecodingKey, possible_request: impl TryInto<Request>) -> Result<Request, warp::reply::WithStatus<warp::reply::Json>> {
        // Validate the token
        let token_data = WebInterface::validate::<PlayerClaims>(token, issuer, key)?;
        
        // Create a player id from the token string
        let token_player_id = match PlayerId::new(&token_data.claims.plyr) {
            Some(id) => id,
            None => {
                return Err(warp::reply::with_status(
                    warp::reply::json(&Reply::failure("Authorization token is invalid.")),
                    http::StatusCode::FORBIDDEN,
                ));
            }
        };

        // Try to create the request from the provided information
        let request = match possible_request.try_into() {
            Ok(request) => request,
            _ => return Err(warp::reply::with_status(
                    warp::reply::json(&Reply::failure("Invalid request.")),
                    http::StatusCode::BAD_REQUEST,
                )),
        };

        // If the two IDs match
        if let Ok(player_id) = request.get_player_id() && player_id == token_player_id {
            // Return the request
            Ok(request)
        
        // Otherwise, return an error
        } else {
            Err(warp::reply::with_status(
                warp::reply::json(&Reply::failure("Player ID is invalid.")),
                http::StatusCode::BAD_REQUEST,
            ))
        }
    }
}
