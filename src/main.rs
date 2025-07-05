mod auth;
mod cli;
mod config;
mod db;
mod http;
mod server;
mod state;
mod ws;

use clap::Parser;
use cli::Cli;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::ERROR)
        .init();

    dotenv::dotenv().ok();

    let cli = Cli::parse();
    match cli.command {
        Some(command) => cli::perform(command).await,
        None => server::serve().await,
    }
}
