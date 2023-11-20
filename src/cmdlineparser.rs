use std::{path::PathBuf, env};

use clap::{Parser, Subcommand};

fn get_default_config_path() -> PathBuf {
    let mut path = env::current_exe().unwrap();
    path.pop();
    path.push("useless-rocket.yaml");
    path
}

#[derive(Parser)]
#[command(name="UselessRocket")]
#[command(author="Daniele Cruciani <daniele@smartango.com>")]
#[command(version="1.0")]
#[command(about ="Register for notification types and listen a socket to push them", long_about = None)]
struct Cli {
    name: Option<String>,
    
    #[arg(short, long, value_name = "FILE", default_value=get_default_config_path().into_os_string())]
    config: PathBuf,

    #[arg(short, long, value_name = "URI", default_value_t=env::var("RABBIT_URL").unwrap_or("amqp://rabbit_rabbit/".to_string()))]
    rabbiturl: String,
    
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,
    
    #[command(subcommand)]
    command: Option<Commands>
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Parse,
    Dry,
    Run
}

use crate::urconfig::{URConfig, UCommands};

pub fn parse() -> URConfig {
    let cli = Cli::parse();

    if let Some(name) = cli.name.as_deref() {
        println!("got a name: {name}");
    }

    let configfile = cli.config.to_string_lossy().to_string();
    let debug: u8 = match cli.debug {
        0 => 0_u8,
        1 => 1_u8,
        2 => 2_u8,
        3 => 3_u8,
        _ => 4_u8,
    };

    let command = match &cli.command {
        Some(Commands::Parse) => UCommands::Parse,
        Some(Commands::Dry) => UCommands::Dry,
        _ => UCommands::Run
    };

    URConfig {
        configfile,
        debug_level: debug,
        command,
        serviceconf: None,
    }
}