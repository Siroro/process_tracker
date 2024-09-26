use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::Duration;
use eframe::egui;
use serde::Deserialize;
use wmi::*;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename = "__InstanceCreationEvent")]
#[serde(rename_all = "PascalCase")]
struct NewProcessEvent {
    target_instance: Process,
}

#[derive(Deserialize, Debug, Clone)]
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

struct ProcessMonitorApp {
    processes: Vec<Process>,
    receiver: Receiver<Process>,
}

impl ProcessMonitorApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (sender, receiver) = channel();
        
        thread::spawn(move || {
            monitor_processes(sender);
        });

        Self {
            processes: Vec::new(),
            receiver,
        }
    }
}

impl eframe::App for ProcessMonitorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(process) = self.receiver.try_recv() {
            self.processes.push(process);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Process Monitor");
            
            egui::ScrollArea::vertical().show(ui, |ui| {
                for process in &self.processes {
                    ui.group(|ui| {
                        ui.label(format!("PID: {}", process.process_id));
                        ui.label(format!("Name: {}", process.name));
                        ui.label(format!("Executable: {:?}", process.executable_path.as_deref().unwrap_or("None")));
                        ui.label(format!("Parent PID: {:?}", process.parent_process_id.unwrap_or(0)));
                        ui.label(format!("Command Line: {:?}", process.command_line.as_deref().unwrap_or("None")));
                        if let Some(wmi_date) = &process.creation_date {
                            ui.label(format!("Created: {}", convert_wmi_date_time(wmi_date.clone())));
                        } else {
                            ui.label("Created: N/A");
                        }
                    });
                    ui.add_space(4.0);
                }
            });
        });

        ctx.request_repaint();
    }
}

fn monitor_processes(sender: Sender<Process>) {
    let mut filters = HashMap::<String, FilterValue>::new();
    filters.insert("TargetInstance".to_owned(), FilterValue::is_a::<Process>().unwrap());
    
    let wmi_con = WMIConnection::new(COMLibrary::new().unwrap()).unwrap();
    const MAX_RETRIES: usize = 1000;
    const RETRY_DELAY: Duration = Duration::from_secs(1);
    
    let mut iterator = None;
    
    for _ in 0..MAX_RETRIES {
        match wmi_con.filtered_notification::<NewProcessEvent>(&filters, Some(Duration::from_secs(1))) {
            Ok(iter) => {
                iterator = Some(iter);
                break;
            },
            Err(e) => {
                eprintln!("Failed to get filtered notification: {:?}", e);
                std::thread::sleep(RETRY_DELAY);
            }
        }
    }
    
    let iterator = match iterator {
        Some(iter) => iter,
        None => {
            eprintln!("Failed to get filtered notification after {} retries.", MAX_RETRIES);
            return;
        }
    };

    for result in iterator {
        if let Ok(event) = result {
            let _ = sender.send(event.target_instance);
        }
    }
}

fn convert_wmi_date_time(wmi_date: WMIDateTime) -> String {
    wmi_date.0.format("%Y-%m-%d %H:%M:%S %z").to_string()
}

fn main() -> Result<(), eframe::Error> {
    let options: eframe::NativeOptions = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([300.0, 220.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Process Monitor",
        options,
        Box::new(|_cc| Ok(Box::new(ProcessMonitorApp::new(_cc)))),
    )
}