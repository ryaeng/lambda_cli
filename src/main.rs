use clap::{Parser, Subcommand};
use reqwest::blocking::Client;
use reqwest::header::AUTHORIZATION;
use std::env;
use dotenv::dotenv;

mod instances;
mod types;

use instances::{find_and_start_instance, list_available_instance_types, list_running_instances, launch_instances, terminate_instances};

/// Simple program to interact with Lambda Labs GPU cloud
#[derive(Parser)]
#[command(name = "lambda")]
#[command(about = "A command-line tool for Lambda Labs cloud GPU API", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Commands releated to instances
    Instances {
        #[command(subcommand)]
        subcommand: InstanceCommands
    },
    /// Commands related to instance types
    InstanceTypes {
        #[command(subcommand)]
        subcommand: InstanceTypeCommands,
    },
    /// Continuously find and start a GPU instance when it becomes available
    Find {
        #[arg(short, long)]
        gpu: String,
        #[arg(short, long, default_value = "")]
        ssh: String,
        #[arg(short, long, default_value_t = 10)]
        sec: u64,
    },
}

#[derive(Subcommand)]
enum InstanceCommands {
    /// Launch instances
    Launch {
        #[arg(short, long)]
        gpu: String,
        #[arg(short, long)]
        ssh: String,
    },
    /// List running instances
    List,
    /// Terminate instances
    Terminate {
        #[arg(short, long)]
        gpu: String,
    },
}

#[derive(Subcommand)]
enum InstanceTypeCommands {
    /// List all available instance types
    List,
}

fn main() {
    dotenv().ok();
    let api_key = env::var("LAMBDA_API_KEY").expect("LAMBDA_API_KEY must be set");
    let client = Client::new();

    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Instances { subcommand }) => match subcommand {
            InstanceCommands::Launch { gpu, ssh } => {
                launch_instances(&client, &api_key, &gpu, &ssh);
            }
            InstanceCommands::List => {
                list_running_instances(&client, &api_key);
            }
            InstanceCommands::Terminate { gpu } => {
                terminate_instances(&client, &api_key, gpu);
            }
        },
        Some(Commands::InstanceTypes { subcommand }) => match subcommand {
            InstanceTypeCommands::List => {
                list_available_instance_types(&client, &api_key);
            }
        },
        Some(Commands::Find { gpu, ssh, sec }) => {
            find_and_start_instance(&client, &api_key, gpu, ssh, *sec);
        }
        None => {
            validate_api_key(&client, &api_key);
        }
    }
}

fn validate_api_key(client: &Client, api_key: &str) {
    let url = "https://cloud.lambdalabs.com/api/v1/instances";
    let response = client.get(url)
        .header(AUTHORIZATION, format!("Bearer {}", api_key))
        .send()
        .expect("Failed to validate API key");

    if response.status().is_success() {
        println!("API key is valid");
    } else {
        println!("Failed to validate API key: {}", response.status());
    }
}

