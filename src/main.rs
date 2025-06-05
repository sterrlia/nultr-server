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
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{}=debug,tower_http=debug", env!("CARGO_CRATE_NAME")).into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    dotenv::dotenv().ok();

    let cli = Cli::parse();
    match cli.command {
        Some(command) => cli::perform(command).await,
        None => server::serve().await,
    }
}
