use crate::cli::TypeHandler;
use futures_util::stream::StreamExt;
use log::{debug, error};
use std::error::Error;
use zbus::{Connection, MatchRule, MessageStream};

pub struct DBusListener {
    pub interface: String,
    pub member: String,
    pub type_handler: TypeHandler,
}

impl DBusListener {
    pub fn new(interface: String, member: String, type_handler: TypeHandler) -> Self {
        Self {
            interface,
            member,
            type_handler,
        }
    }

    /// Establish connection and listen for D-Bus signals
    pub async fn listen(&self) -> Result<(), Box<dyn Error>> {
        // Try to connect to session bus first, fallback to system bus
        let connection = match Connection::session().await {
            Ok(conn) => {
                debug!("Connected to session bus");
                conn
            }
            Err(_) => {
                debug!("Failed to connect to session bus, trying system bus");
                Connection::system().await?
            }
        };

        // Create a match rule for the specific signal
        let match_rule = MatchRule::builder()
            .msg_type(zbus::message::Type::Signal)
            .interface(self.interface.as_str())?
            .member(self.member.as_str())?
            .build();

        debug!(
            "Adding match rule for interface: {}, member: {}",
            self.interface, self.member
        );

        // Create a filtered message stream for our match rule
        // This automatically registers the rule with the bus
        let mut stream = MessageStream::for_match_rule(match_rule, &connection, None).await?;

        debug!("Listening for D-Bus signals...");

        // Main listening loop - now we only receive messages that match our criteria
        while let Some(msg) = stream.next().await {
            match msg {
                Ok(message) => {
                    let body = message.body();

                    // For boolean signals, try to deserialize directly as boolean first
                    if let Ok(bool_value) = body.deserialize::<(bool,)>() {
                        let value = zbus::zvariant::Value::Bool(bool_value.0);
                        match self.type_handler.process(&value) {
                            Some(output) => {
                                println!("{}", output);
                            }
                            None => {
                                error!("Failed to process boolean value: {}", bool_value.0);
                            }
                        }
                    } else {
                        // Fallback: try to get as generic Value tuple
                        match body.deserialize::<(zbus::zvariant::Value,)>() {
                            Ok((value,)) => match self.type_handler.process(&value) {
                                Some(output) => {
                                    println!("{}", output);
                                }
                                None => {
                                    error!("Failed to process signal value: {:?}", value);
                                }
                            },
                            Err(e) => {
                                error!("Failed to deserialize message body: {}", e);
                                debug!("Message signature: {:?}", message.body().signature());
                                debug!("Raw body: {:?}", body);
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Error receiving message: {}", e);
                    return Err(e.into());
                }
            }
        }

        Ok(())
    }
}
