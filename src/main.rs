use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;
use wmi::*;

#[derive(Deserialize, Debug)]
#[serde(rename = "__InstanceCreationEvent")]
#[serde(rename_all = "PascalCase")]
struct NewProcessEvent {
    target_instance: Process,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "Win32_Process")]
#[serde(rename_all = "PascalCase")]
struct Process {
    process_id: u32,
    name: String,
    executable_path: Option<String>,
    parent_process_id: Option<u32>,
    command_line: Option<String>,
    creation_date: Option<WMIDateTime>,
}

fn main() {
    let mut filters = HashMap::<String, FilterValue>::new();

    filters.insert(
        "TargetInstance".to_owned(),
        FilterValue::is_a::<Process>().unwrap(),
    );
    let wmi_con = WMIConnection::new(COMLibrary::new().unwrap()).unwrap();
    const MAX_RETRIES: usize = 1000;
    const RETRY_DELAY: Duration = Duration::from_secs(1);

    let mut iterator = None;

    for _ in 0..MAX_RETRIES {
        match wmi_con
            .filtered_notification::<NewProcessEvent>(&filters, Some(Duration::from_secs(1)))
        {
            Ok(iter) => {
                iterator = Some(iter);
                break;
            }
            Err(e) => {
                eprintln!("Failed to get filtered notification: {:?}", e);
                std::thread::sleep(RETRY_DELAY);
            }
        }
    }

    // Check if the iterator was successfully created
    let iterator = match iterator {
        Some(iter) => iter,
        None => {
            eprintln!(
                "Failed to get filtered notification after {} retries.",
                MAX_RETRIES
            );
            return;
        }
    };
    println!("Monitoring new processes.");

    for result in iterator {
        let process = result.unwrap().target_instance;
        println!("============NEW PROCESS============");
        println!("PID:        {}", process.process_id);
        println!("Name:       {}", process.name);
        println!(
            "Executable: {:?}",
            process.executable_path.unwrap_or("None".to_owned())
        );
        println!(
            "Parent PID: {:?}",
            process.parent_process_id.unwrap_or(0.to_owned())
        );
        println!(
            "Command:    {:?}",
            process.command_line.unwrap_or("None".to_owned())
        );
        match process.creation_date {
            Some(wmi_date) => {
                let formatted_date = convert_wmi_date_time(wmi_date);
                println!("Created:    {}", formatted_date);
            }
            None => {
                println!("Created:    N/A");
            }
        }
    }
}

fn convert_wmi_date_time(wmi_date: WMIDateTime) -> String {
    let formatted_string: String = wmi_date.0.format("%Y-%m-%d %H:%M:%S %z").to_string();
    formatted_string
}
