mod api;
mod auth;
mod cli;
mod commands;
mod config;
mod db;
mod keystore;
mod quic;
mod relay;
mod rendezvous_server;
mod session;
mod state;
mod ws;

use std::process::ExitCode;

use clap::Parser;
use cli::{Cli, Command};

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Command::Init { domain, passphrase_env } => {
            commands::cmd_init(domain, passphrase_env)
        }

        Command::Start {
            daemon, mode, domain, api_port, listen_port,
            rendezvous, relay, passphrase_env, log_level, config_path,
        } => {
            commands::cmd_start(
                daemon, mode, domain, api_port, listen_port,
                rendezvous, relay, passphrase_env, log_level, config_path,
            )
        }

        Command::Stop => commands::cmd_stop(cli.api_url.as_deref()),
        Command::Status => commands::cmd_status(cli.api_url.as_deref(), cli.json),
        Command::Identity { action } => commands::cmd_identity(action),
        Command::Peers { action } => commands::cmd_peers(action, cli.api_url.as_deref(), cli.json),
        Command::Rooms { action } => commands::cmd_rooms(action, cli.api_url.as_deref(), cli.json),
        Command::Rendezvous { action } => commands::cmd_rendezvous(action),
        Command::Logs { level, since, no_follow } => commands::cmd_logs(level, since, no_follow),
        Command::Config { action } => commands::cmd_config(action),
    }
}
