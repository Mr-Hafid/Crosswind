mod airdrop;

use std::process::ExitCode;

use clap::{Parser, Subcommand};
use crosswind_core::{AwdlRoute, Platform};

#[derive(Parser)]
#[command(name = "crosswind", version, about = "AirDrop-compatible file transfer beyond Apple devices")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    #[command(about = "Show which AWDL routes this platform can try")]
    Doctor,
    #[command(about = "Receive files from AirDrop senders")]
    Receive(airdrop::ReceiveArgs),
    #[command(about = "List nearby AirDrop receivers")]
    Browse(airdrop::BrowseArgs),
    #[command(about = "Send files or links to an AirDrop receiver")]
    Send(airdrop::SendArgs),
}

fn run_doctor() -> ExitCode {
    let platform = Platform::current();
    println!("Platform: {}", platform.display_name());
    println!();
    println!("Candidate AWDL routes:");
    for route in AwdlRoute::candidates_for(platform) {
        println!("  - {:?}: {}", route, route.summary());
        println!("    status: {}", route.status().label());
    }
    ExitCode::SUCCESS
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Doctor => return run_doctor(),
        Command::Receive(args) => airdrop::receive(args).await,
        Command::Browse(args) => airdrop::browse(args).await,
        Command::Send(args) => airdrop::send(args).await,
    };
    result.unwrap_or_else(|error| {
        eprintln!("crosswind: {error}");
        ExitCode::FAILURE
    })
}
