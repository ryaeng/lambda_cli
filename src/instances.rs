use chrono::Local;
use colored::Colorize;
use crossterm::{execute, terminal::{Clear, ClearType}, cursor::MoveTo};
use prettytable::{row, Table};
use reqwest::blocking::Client;
use reqwest::header::AUTHORIZATION;
use std::collections::HashMap;
use std::io::stdout;
use std::thread;
use std::time::{Duration, Instant};

use crate::types::{Instance, InstanceTypeResponse, LaunchResponse, ApiResponse};

pub fn find_and_start_instance(client: &Client, api_key: &str, gpu: &str, ssh: &str, sec: u64) {
    println!("Looking for available instances of type {}...", gpu);

    loop {
        let start_time = Instant::now();
        let check_time = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

        let mut table = Table::new();
        table.add_row(row!["Last Checked", "Status", "Next Check In (s)"]);

        if let Some(instance_type_response) = get_instance_type_response(client, api_key, gpu) {
            if !instance_type_response.regions_with_capacity_available.is_empty() {
                let regions: Vec<String> = instance_type_response.regions_with_capacity_available
                    .iter()
                    .map(|region| format!("{} ({})", region.name, region.description))
                    .collect();

                println!("Found available {} in region(s): {:?}", gpu, regions);
                launch_instances(client, api_key, gpu, ssh);
                break;
            }
        }
        
        let next_check_in = sec.saturating_sub(start_time.elapsed().as_secs());
        table.add_row(row![
            check_time,
            "No available instances found".red(),
            next_check_in.to_string().yellow()
        ]);

        // Clear the screen and print the updated table
        execute!(stdout(), Clear(ClearType::All), MoveTo(0, 0)).unwrap();
        table.printstd();

        thread::sleep(Duration::from_secs(next_check_in));
    }
}

fn get_instance_details(client: &Client, api_key: &str, instance_id: &str) -> Instance {
    let url = format!("https://cloud.lambdalabs.com/api/v1/instances/{}", instance_id);
    let response_result = client.get(&url)
        .header(AUTHORIZATION, format!("Bearer {}", api_key))
        .send();

    match response_result {
        Ok(response) => {
            let response_text = response.text().unwrap_or_else(|_| "Failed to read response text".to_string());
            match serde_json::from_str::<ApiResponse<Instance>>(&response_text) {
                Ok(parsed_response) => parsed_response.data,
                Err(e) => {
                    println!("Failed to parse response: {}\nResponse text: {}", e, response_text);
                    panic!("Failed to get instance details");
                }
            }
        }
        Err(e) => {
            println!("Request failed: {}", e);
            panic!("Failed to get instance details");
        }
    }
}

fn get_instance_type_response(client: &Client, api_key: &str, gpu: &str) -> Option<InstanceTypeResponse> {
    let url = "https://cloud.lambdalabs.com/api/v1/instance-types";
    let response: ApiResponse<HashMap<String, InstanceTypeResponse>> = client.get(url)
        .header(AUTHORIZATION, format!("Bearer {}", api_key))
        .send()
        .expect("Failed to get instance types")
        .json()
        .expect("Failed to parse response");

    response.data.get(gpu).cloned()
}

pub fn launch_instances(client: &Client, api_key: &str, gpu: &str, ssh: &str) {
    if let Some(instance_type_response) = get_instance_type_response(client, api_key, gpu) {
        let region_name = &instance_type_response.regions_with_capacity_available[0].name;

        let url = "https://cloud.lambdalabs.com/api/v1/instance-operations/launch";
        let payload = serde_json::json!({
            "region_name": region_name,
            "instance_type_name": gpu,
            "ssh_key_names": [ssh],
            "quantity": 1
        });

        let response_result = client.post(url)
            .header(AUTHORIZATION, format!("Bearer {}", api_key))
            .json(&payload)
            .send();

        match response_result {
            Ok(response) => {
                let response_text = response.text().unwrap_or_else(|_| "Failed to read response text".to_string());
                match serde_json::from_str::<ApiResponse<LaunchResponse>>(&response_text) {
                    Ok(parsed_response) => {
                        let instance_id = &parsed_response.data.instance_ids[0];
                        println!("Instance {} started in region {}. Waiting for it to become active...", instance_id, region_name);

                        std::thread::sleep(std::time::Duration::from_secs(120));

                        let instance = get_instance_details(client, api_key, instance_id);
                        match instance.ip {
                            Some(ip) => println!("Instance is active. SSH IP: {}", ip),
                            None => println!("Instance is active, but IP address is not available yet."),
                        }
                    }
                    Err(e) => {
                        println!("Failed to parse response: {}\nResponse text: {}", e, response_text);
                    }
                }
            }
            Err(e) => {
                println!("Request failed: {}", e);
            }
        }
    } else {
        println!("Instance type {} not found.", gpu);
    }
}

pub fn list_available_instance_types(client: &Client, api_key: &str) {
    let url = "https://cloud.lambdalabs.com/api/v1/instance-types";
    let response: ApiResponse<HashMap<String, InstanceTypeResponse>> = client.get(url)
        .header(AUTHORIZATION, format!("Bearer {}", api_key))
        .send()
        .expect("Failed to list instances")
        .json()
        .expect("Failed to parse response");

    let mut table = Table::new();
    table.add_row(row!["Instance Type", "Description", "Price (cents/hour)", "vCPUs", "Memory (GiB)", "Storage (GiB)", "Available Regions"]);

    for (key, instance_type_response) in response.data {
        if !instance_type_response.regions_with_capacity_available.is_empty() {
            let regions: Vec<String> = instance_type_response.regions_with_capacity_available
                .iter()
                .map(|region| format!("{} ({})", region.name, region.description))
                .collect();

            table.add_row(row![
                key.green(),
                instance_type_response.instance_type.description.clone(),
                instance_type_response.instance_type.price_cents_per_hour.to_string().yellow(),
                instance_type_response.instance_type.specs.vcpus.to_string(),
                instance_type_response.instance_type.specs.memory_gib.to_string(),
                instance_type_response.instance_type.specs.storage_gib.to_string(),
                regions.join(", ").blue()
            ]);
        }
    }

    table.printstd();
}

pub fn list_running_instances(client: &Client, api_key: &str) {
    let url = "https://cloud.lambdalabs.com/api/v1/instances";
    let response: ApiResponse<Vec<Instance>> = client.get(url)
        .header(AUTHORIZATION, format!("Bearer {}", api_key))
        .send()
        .expect("Failed to list running instances")
        .json()
        .expect("Failed to parse response");

    let mut table = Table::new();
    table.add_row(row!["Instance ID", "Type", "Status", "IP Address", "SSH Key Names"]);

    for instance in response.data {
        table.add_row(row![
            instance.id.unwrap_or_else(|| "N/A".to_string()).green(),
            instance.instance_type
                .as_ref()
                .map(|it| it.name.clone())
                .unwrap_or_else(|| "N/A".to_string())
                .cyan(),
            instance.status.unwrap_or_else(|| "N/A".to_string()).yellow(),
            instance.ip.unwrap_or_else(|| "N/A".to_string()).blue(),
            instance.ssh_key_names.unwrap_or_else(|| vec!["N/A".to_string()]).join(", ").purple()
        ]);
    }

    table.printstd();
}

pub fn terminate_instances(client: &Client, api_key: &str, gpu: &str) {
    let url = "https://cloud.lambdalabs.com/api/v1/instance-operations/terminate";
    let payload = serde_json::json!({
        "instance_ids": [gpu]
    });

    client.post(url)
        .header(AUTHORIZATION, format!("Bearer {}", api_key))
        .json(&payload)
        .send()
        .expect("Failed to stop instance");

    println!("Instance {} stopped", gpu);
}
