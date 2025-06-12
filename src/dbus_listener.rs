use futures_util::stream::StreamExt;
use std::error::Error;
use zbus::{Connection, MatchRule, MessageStream};

pub struct DBusListener {
    pub interface: String,
    pub member: String,
}

impl DBusListener {
    pub fn new(interface: String, member: String) -> Self {
        Self { interface, member }
    }

    /// Establish connection and listen for D-Bus signals
    pub async fn listen(&self) -> Result<(), Box<dyn Error>> {
        // Try to connect to session bus first, fallback to system bus
        let connection = match Connection::session().await {
            Ok(conn) => {
                eprintln!("Connected to session bus");
                conn
            }
            Err(_) => {
                eprintln!("Failed to connect to session bus, trying system bus");
                Connection::system().await?
            }
        };

        // Create a match rule for the specific signal
        let match_rule = MatchRule::builder()
            .msg_type(zbus::message::Type::Signal)
            .interface(self.interface.as_str())?
            .member(self.member.as_str())?
            .build();

        eprintln!(
            "Adding match rule for interface: {}, member: {}",
            self.interface, self.member
        );

        // Create a filtered message stream for our match rule
        // This automatically registers the rule with the bus
        let mut stream = MessageStream::for_match_rule(match_rule, &connection, None).await?;

        eprintln!("Listening for D-Bus signals...");

        // Main listening loop - now we only receive messages that match our criteria
        while let Some(msg) = stream.next().await {
            match msg {
                Ok(message) => {
                    let body = message.body();

                    // Print the matched signal to stdout
                    println!("Received signal: {:?}", body);

                    // Debug info
                    eprintln!("Raw body: {:?}", body);
                }
                Err(e) => {
                    eprintln!("Error receiving message: {}", e);
                    return Err(e.into());
                }
            }
        }

        Ok(())
    }
}
