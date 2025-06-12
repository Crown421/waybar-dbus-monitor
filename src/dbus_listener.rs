use crate::cli::TypeHandler;
use futures_lite::stream::StreamExt;
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
            Err(e) => {
                debug!("Failed to connect to session bus: {}", e);
                debug!("Trying system bus");
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
                    if let Err(e) = self.process_message(&message) {
                        error!("Error processing message: {}", e);
                        // Continue listening rather than crashing on a single message error
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

    /// Process a single D-Bus message and print the result
    fn process_message(&self, message: &zbus::Message) -> Result<(), Box<dyn Error>> {
        let body = message.body();

        // Try to deserialize as a single Value - this handles most cases
        match body.deserialize::<(zbus::zvariant::Value,)>() {
            Ok((value,)) => {
                if let Some(output) = self.type_handler.process(&value) {
                    println!("{}", output);
                    Ok(())
                } else {
                    Err(format!("Failed to process signal value: {:?}", value).into())
                }
            }
            Err(e) => {
                error!("Failed to deserialize message body: {}", e);
                debug!("Message signature: {:?}", message.body().signature());
                debug!("Raw body: {:?}", body);
                Err(e.into())
            }
        }
    }
}
