use crate::cli::Config;
use crate::error::AppError;
use crate::retry::retry_dbus_operation;
use crate::{error_message_processing, error_not_found, error_service_unavailable, report_error};
use futures_lite::stream::StreamExt;
use log::{debug, error, warn};
use std::io::Write;
use zbus::{Connection, MatchRule, MessageStream, Proxy};

pub struct DBusListener {
    pub config: Config,
}

impl DBusListener {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// Establish connection and listen for D-Bus signals with retry logic
    pub async fn listen(&self) -> Result<(), AppError> {
        let connection = retry_dbus_operation(
            || async { self.establish_connection().await },
            "D-Bus connection",
        )
        .await?;

        // --- PHASE 1: Initial State Query ---
        if let Some(status_config) = match self.config.parse_status() {
            Ok(config) => config,
            Err(e) => {
                error!("Failed to parse status configuration: {}", e);
                return Err(error_not_found!("Invalid status format: {}", e));
            }
        } {
            let proxy = Proxy::new(
                &connection,
                status_config.service.as_str(),
                status_config.object_path.as_str(),
                status_config.interface.as_str(),
            )
            .await?;

            match proxy
                .get_property::<zbus::zvariant::Value>(&status_config.property)
                .await
            {
                Ok(value) => {
                    if let Some(output) = self.config.type_handler.process(&value) {
                        println!("{}", output);
                        // Flush stdout
                        if let Err(e) = std::io::stdout().flush() {
                            error!("Failed to flush stdout: {}", e);
                        }
                    }
                }
                Err(e) => {
                    // Check if this is a "not found" type error for interface/service availability
                    if e.to_string().contains("not found")
                        || e.to_string().contains("NotFound")
                        || e.to_string().contains("ServiceUnknown")
                        || e.to_string().contains("UnknownObject")
                    {
                        return Err(error_service_unavailable!(
                            "D-Bus interface '{}' or monitor '{}' not available: {}",
                            self.config.interface,
                            self.config.monitor,
                            e
                        ));
                    } else {
                        warn!(
                            "Warning: Could not get initial property '{}': {}",
                            status_config.property, e
                        );
                    }
                }
            }
        }

        // --- PHASE 2: Signal Listening ---
        let mut stream = retry_dbus_operation(
            || async { self.setup_message_stream(&connection).await },
            "D-Bus message stream setup",
        )
        .await?;

        debug!("Listening for D-Bus signals...");

        // Main listening loop - now we only receive messages that match our criteria
        while let Some(msg) = stream.next().await {
            match msg {
                Ok(message) => {
                    if let Err(e) = self.process_message(&message) {
                        // Print error code to stdout for waybar and log error
                        report_error!(e, "Error processing message");
                        // Continue listening rather than crashing on a single message error
                    }
                }
                Err(e) => {
                    let app_error = AppError::from(e);
                    report_error!(app_error, "Error receiving message");
                    return Err(app_error);
                }
            }
        }

        Ok(())
    }

    /// Establish D-Bus connection with fallback from session to system bus
    async fn establish_connection(&self) -> Result<Connection, AppError> {
        // Try to connect to session bus first, fallback to system bus
        match Connection::session().await {
            Ok(conn) => {
                debug!("Connected to session bus");
                Ok(conn)
            }
            Err(e) => {
                debug!("Failed to connect to session bus: {}", e);
                debug!("Trying system bus");

                match Connection::system().await {
                    Ok(conn) => {
                        debug!("Connected to system bus");
                        Ok(conn)
                    }
                    Err(system_err) => {
                        error!("Failed to connect to both session and system bus");
                        error!("Session bus error: {}", e);
                        error!("System bus error: {}", system_err);
                        Err(AppError::connection_failed(system_err))
                    }
                }
            }
        }
    }

    /// Setup message stream for the specific signal
    async fn setup_message_stream(
        &self,
        connection: &Connection,
    ) -> Result<MessageStream, AppError> {
        // Create a match rule for the specific signal
        let match_rule: MatchRule<'_> = MatchRule::builder()
            .msg_type(zbus::message::Type::Signal)
            .interface(self.config.interface.as_str())
            .map_err(|e| error_not_found!("Invalid interface '{}': {}", self.config.interface, e))?
            .member(self.config.monitor.as_str())
            .map_err(|e| error_not_found!("Invalid monitor '{}': {}", self.config.monitor, e))?
            .build();

        debug!(
            "Adding match rule for interface: {}, monitor: {}",
            self.config.interface, self.config.monitor
        );

        // Create a filtered message stream for our match rule
        // This automatically registers the rule with the bus
        let stream = MessageStream::for_match_rule(match_rule, connection, None)
            .await
            .map_err(AppError::from)?;

        Ok(stream)
    }

    /// Process a single D-Bus message and print the result
    fn process_message(&self, message: &zbus::Message) -> Result<(), AppError> {
        let body = message.body();

        // Try to deserialize as a single Value - this handles most cases
        match body.deserialize::<(zbus::zvariant::Value,)>() {
            Ok((value,)) => {
                if let Some(output) = self.config.type_handler.process(&value) {
                    println!("{}", output);
                    Ok(())
                } else {
                    Err(error_message_processing!(
                        "Failed to process signal value: {:?}",
                        value
                    ))
                }
            }
            Err(e) => {
                error!("Failed to deserialize message body: {}", e);
                debug!("Message signature: {:?}", message.body().signature());
                debug!("Raw body: {:?}", body);
                Err(error_message_processing!(
                    "Failed to deserialize message: {}",
                    e
                ))
            }
        }
    }
}
