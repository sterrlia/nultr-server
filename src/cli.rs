use clap::{Parser, Subcommand};
use rand::{Rng, distr::Alphanumeric};
use sea_orm::ActiveValue::Set;

use crate::{db::entity::users, state};

#[derive(Parser)]
#[command(name = "manager")]
#[command(about = "The only way to add users", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    AddUser { username: String },
    DeleteUser { username: String }
}

pub async fn perform(command: Command) {
    let state = state::CliState::default();

    if let Err(error) = try_perform(state, command).await {
        let error_message = error.to_string();

        println!("Error occurred: {error_message}")
    }
}

pub async fn try_perform(state: state::CliState, command: Command) -> anyhow::Result<()> {
    match command {
        Command::AddUser { username } => {
            let password: String = rand::rng()
                .sample_iter(&Alphanumeric)
                .take(7)
                .map(char::from)
                .collect();

            let password_hash = state.password_hasher.hash_password(password.as_str());

            let user_model = users::ActiveModel {
                username: Set(username),
                password_hash: Set(password_hash),
                ..Default::default()
            };

            state.user_repository.insert(user_model).await?;

            println!("User created. Generated password: {password}");

            Ok(())
        }
        Command::DeleteUser { username } => {
            let user_result = state
                .user_repository
                .get_by_username(username)
                .await?;

            if let Some(user) = user_result {
                state.user_repository.delete(user).await?;
                println!("User deleted");
            } else {
                println!("User not found");
            }

            Ok(())
        }
    }
}
