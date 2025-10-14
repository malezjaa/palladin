use clap::builder::styling::{AnsiColor, Effects};
use clap::builder::Styles;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "Palladin")]
#[command(about = "Build tool for web applications")]
#[command(long_about = "Palladin: A modern build tool for web applications")]
#[command(version)]
#[command(author)]
#[command(styles = get_styles())]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run the development server
    Dev {
        /// Host address to bind the server to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Port to run the server on
        #[arg(short, long, default_value = "8080")]
        port: u16,

        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
}

fn get_styles() -> Styles {
    Styles::styled()
        .usage(AnsiColor::Yellow.on_default() | Effects::BOLD)
        .header(AnsiColor::Green.on_default() | Effects::BOLD)
        .literal(AnsiColor::Cyan.on_default())
        .invalid(AnsiColor::Red.on_default() | Effects::BOLD)
        .error(AnsiColor::Red.on_default() | Effects::BOLD)
        .valid(AnsiColor::Green.on_default())
        .placeholder(AnsiColor::White.on_default())
}
