//! JSON protocol server implementation

use std::convert::TryFrom;
use std::net::SocketAddr;

use tokio::net::TcpListener;
use tokio::prelude::*;

use tokio::codec::Framed;

use crate::color;
use crate::hyperion::{Input, StateUpdate};
use crate::image::RawImage;
use crate::runtime::HostHandle;

/// Schema definitions as Serde serializable structures and enums
mod message;
use message::{HyperionMessage, HyperionResponse};

/// JSON protocol codec definition
mod codec;
use codec::*;

pub use message::{Effect, EffectDefinition};

#[allow(missing_docs)]
mod errors {
    use error_chain::error_chain;

    error_chain! {
        types {
            JsonServerError, JsonServerErrorKind, ResultExt;
        }

        foreign_links {
            Io(::std::io::Error);
        }
    }
}

pub use errors::*;

/// Binds the JSON protocol server implementation to the given address, returning a future to
/// process incoming requests.
///
/// # Parameters
///
/// * `address`: address (and port) of the endpoint to bind the JSON server to
/// * `host`: component host
/// * `tripwire`: handle to the cancellation future
///
/// # Errors
///
/// * When the server can't be bound to the given address
pub fn bind(
    address: &SocketAddr,
    host: HostHandle,
    tripwire: stream_cancel::Tripwire,
) -> Result<Box<dyn Future<Item = (), Error = std::io::Error> + Send>, JsonServerError> {
    let listener = TcpListener::bind(&address)?;

    let server = listener.incoming().for_each(move |socket| {
        debug!(
            "accepted new connection from {}",
            socket.peer_addr().unwrap()
        );

        let sender = host.get_service_input_sender();

        let framed = Framed::new(socket, JsonCodec::new());
        let (writer, reader) = framed.split();
        let host = host.clone();

        let action = reader
            .and_then(move |request| {
                trace!("processing request: {:?}", request);

                let reply = match request {
                    HyperionMessage::ClearAll => {
                        // Update state
                        sender
                            .unbounded_send(Input::user_input(StateUpdate::Clear, 0, None))
                            .unwrap();

                        HyperionResponse::success()
                    }
                    HyperionMessage::Clear { priority } => {
                        // Update state
                        sender
                            .unbounded_send(Input::user_input(StateUpdate::Clear, priority, None))
                            .unwrap();

                        HyperionResponse::success()
                    }
                    HyperionMessage::Color {
                        priority,
                        duration,
                        color,
                    } => {
                        let update = StateUpdate::SolidColor {
                            color: color::ColorPoint::from((
                                f32::from(color[0]) / 255.0,
                                f32::from(color[1]) / 255.0,
                                f32::from(color[2]) / 255.0,
                            )),
                        };

                        // Update state
                        sender
                            .unbounded_send(Input::user_input(update, priority, duration))
                            .unwrap();

                        HyperionResponse::success()
                    }
                    HyperionMessage::Image {
                        priority,
                        duration,
                        imagewidth,
                        imageheight,
                        imagedata,
                    } => {
                        // Try to convert sizes to unsigned fields
                        u32::try_from(imagewidth)
                            .and_then(|imagewidth| {
                                u32::try_from(imageheight)
                                    .map(|imageheight| (imagewidth, imageheight))
                            })
                            .map_err(|_| "invalid size".to_owned())
                            .and_then(|(imagewidth, imageheight)| {
                                // Try to create image from raw data and given size
                                RawImage::try_from((imagedata, imagewidth, imageheight))
                                    .map(|raw_image| {
                                        // Update state
                                        sender
                                            .unbounded_send(Input::user_input(
                                                StateUpdate::Image(raw_image),
                                                priority,
                                                duration,
                                            ))
                                            .unwrap();

                                        HyperionResponse::success()
                                    })
                                    .map_err(|error| error.to_string())
                            })
                            .unwrap_or_else(|error| HyperionResponse::ErrorResponse {
                                success: false,
                                error,
                            })
                    }
                    HyperionMessage::Effect {
                        priority,
                        duration,
                        effect,
                    } => {
                        // Update state
                        sender
                            .unbounded_send(Input::effect(priority, duration, effect))
                            .unwrap();

                        // TODO: Only send success if effect was found
                        HyperionResponse::success()
                    }
                    HyperionMessage::ServerInfo => {
                        let effects = host.get_effect_engine().get_definitions();

                        HyperionResponse::server_info(
                            hostname::get()
                                .map(|h| String::from(h.to_string_lossy()))
                                .unwrap_or_else(|_| "<unknown hostname>".to_owned()),
                            effects,
                            option_env!("HYPERION_VERSION_ID")
                                .unwrap_or("<unknown version>")
                                .to_owned(),
                        )
                    }
                    _ => HyperionResponse::error("not implemented".into()),
                };

                trace!("sending response: {:?}", reply);

                Ok(reply)
            })
            .forward(writer)
            .map(|_| {})
            .map_err(|e| {
                warn!("error while processing request: {}", e);
            })
            .select(tripwire.clone())
            .map(|_| ())
            .map_err(|_| {
                error!("server tripwire error");
            });

        tokio::spawn(action);

        Ok(())
    });

    info!("server listening on {}", address);

    Ok(Box::new(server))
}
