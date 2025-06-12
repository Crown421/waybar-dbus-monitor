mod cli;

use clap::Parser;

fn main() {
    let config = cli::Config::parse();

    println!("Parsed configuration:");
    println!("Interface: {}", config.interface);
    println!("Member: {}", config.member);
    println!("Type handler: {:?}", config.type_handler);

    match &config.type_handler {
        cli::TypeHandler::Boolean {
            return_true,
            return_false,
        } => {
            println!("Boolean handler configured:");
            println!("  Return on true: '{}'", return_true);
            println!("  Return on false: '{}'", return_false);
        }
    }
}
