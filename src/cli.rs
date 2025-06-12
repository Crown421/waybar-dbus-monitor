use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Config {
    /// D-Bus interface to monitor
    #[arg(long)]
    pub interface: String,

    /// D-Bus member (signal/method) to monitor
    #[arg(long)]
    pub member: String,

    /// Type handler for the monitored data
    #[command(subcommand)]
    pub type_handler: TypeHandler,
}

#[derive(Subcommand, Debug)]
pub enum TypeHandler {
    /// Monitor a boolean value
    Boolean {
        /// String to return when value is true
        #[arg(long, default_value = "true")]
        return_true: String,

        /// String to return when value is false
        #[arg(long, default_value = "false")]
        return_false: String,
    },
    // Future expansion:
    // String { ... },
    // Integer { ... },
}
