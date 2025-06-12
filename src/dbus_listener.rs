use crate::cli::Config;
use crate::error::AppError;
use crate::retry::{RetryConfig, retry_operation, retry_operation_with_config};
use crate::{error_message_processing, error_not_found, report_error};
use futures_lite::stream::StreamExt;
use log::debug;
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
        // Use default retry configuration for connection
        let connection = retry_operation_with_config(
            || async { self.establish_connection().await },
            "D-Bus connection",
            RetryConfig::default(),
        )
        .await?;

        // --- PHASE 1: Initial State Query ---
        if let Some(status_config) = match self.config.parse_status() {
            Ok(config) => config,
            Err(e) => {
                debug!("error: Failed to parse status configuration: {}", e);
                return Err(error_not_found!("Invalid status format: {}", e));
            }
        } {
            // Wrap the property query in retry for service availability
            let initial_state_result = retry_operation_with_config(
                || async {
                    let proxy = Proxy::new(
                        &connection,
                        status_config.service.as_str(),
                        status_config.object_path.as_str(),
                        status_config.interface.as_str(),
                    )
                    .await?;

                    let value = proxy
                        .get_property::<zbus::zvariant::Value>(&status_config.property)
                        .await?;

                    Ok::<_, AppError>(value)
                },
                "initial property query",
                RetryConfig::default(),
            )
            .await;

            // Handle the result after retries
            match initial_state_result {
                Ok(value) => {
                    // Process and print the value (stdout flushing is handled internally)
                    self.config.type_handler.process_and_print(&value);
                }
                Err(e) => {
                    // If it's a service unavailable error after all retries, exit with proper error code
                    if matches!(e, AppError::ServiceUnavailable(_, _)) {
                        return Err(e);
                    }
                    // For other errors, just log a warning rather than failing completely
                    debug!(
                        "warn: Could not get initial property '{}' after retries: {}",
                        status_config.property, e
                    );
                }
            }
        }

        // --- PHASE 2: Signal Listening ---
        let mut stream = retry_operation(
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

                    // Only exit if this is a permanent connection error
                    if matches!(app_error, AppError::BadGateway(_, _)) {
                        return Err(app_error);
                    }
                    // Otherwise continue listening for new messages
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
                        debug!("error: Failed to connect to both session and system bus");
                        debug!("error: Session bus error: {}", e);
                        debug!("error: System bus error: {}", system_err);
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
        debug!("Processing message with signature: {:?}", body.signature());

        // Use the new unified process_message method from TypeHandler
        match self.config.type_handler.process_message(message) {
            Ok(_) => Ok(()),
            Err(e) => {
                debug!("error: {}", e);
                Err(error_message_processing!(
                    "Failed to process message with signature: {:?}: {}",
                    body.signature(),
                    e
                ))
            }
        }
    }
}
