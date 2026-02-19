use crate::backend::{BrightnessctlBackend, DDCBackend, DDC_BIN};
use crate::control::ControlWorker;
use regex::Regex;
use std::fs;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;

pub fn detect_controls(workers_mutex: Arc<Mutex<Vec<Arc<ControlWorker>>>>) {
    {
        let mut workers = workers_mutex.lock().unwrap();
        if let Ok(entries) = fs::read_dir("/sys/class/backlight") {
            for entry in entries.flatten() {
                if entry.path().join("brightness").exists() {
                    if let Some(name) = entry.file_name().to_str() {
                        let b = Box::new(BrightnessctlBackend::new(name.to_string()));
                        workers.push(Arc::new(ControlWorker::new(
                            "Internal Backlight".to_string(),
                            b,
                        )));
                    }
                }
            }
        }
    }
    if DDC_BIN.is_some() {
        let workers_clone = workers_mutex.clone();
        thread::spawn(move || {
            let re = Regex::new(r"/dev/i2c-(\d+)").unwrap();
            if let Ok(entries) = fs::read_dir("/dev") {
                for entry in entries.flatten() {
                    let path_str = entry.path().to_string_lossy().to_string();
                    if let Some(caps) = re.captures(&path_str) {
                        let bus_id = caps[1].to_string();
                        let bus_id_c = bus_id.clone();
                        let wc = workers_clone.clone();
                        thread::spawn(move || {
                            check_bus(bus_id_c, wc);
                        });
                    }
                }
            }
        });
    }
}

fn get_monitor_name(bus: &str) -> String {
    if let Some(bin) = DDC_BIN.as_ref() {
        if let Ok(output) = Command::new(bin).args(["detect", "--brief"]).output() {
            let s = String::from_utf8_lossy(&output.stdout);
            let mut found_bus = false;
            for line in s.lines() {
                if line.contains(&format!("/dev/i2c-{}", bus)) {
                    found_bus = true;
                }
                if found_bus && line.trim().starts_with("Monitor:") {
                    if let Some(caps) = Regex::new(r"Monitor:\s+([^:]+)").unwrap().captures(line) {
                        if let Some(m) = caps.get(1) {
                            return m.as_str().trim().to_string();
                        }
                    }
                    if let Some(caps) = Regex::new(r"Monitor:\s+(.*)").unwrap().captures(line) {
                        if let Some(m) = caps.get(1) {
                            return m.as_str().trim().to_string();
                        }
                    }
                    return format!("Display-{}", bus);
                }
                if line.trim().starts_with("Display") {
                    found_bus = false;
                }
            }
        }
    }
    format!("Display-{}", bus)
}

fn check_bus(bus: String, workers: Arc<Mutex<Vec<Arc<ControlWorker>>>>) {
    if DDC_BIN.is_none() {
        return;
    }
    let bin = DDC_BIN.as_ref().unwrap();

    let output = Command::new(bin)
        .args([
            "getvcp",
            "10",
            "--bus",
            &bus,
            "--brief",
            "--sleep-multiplier",
            "0.05",
        ])
        .output();

    if let Ok(o) = output {
        if o.status.success() {
            let name = get_monitor_name(&bus);
            let b1 = Box::new(DDCBackend::new(bus.clone(), 10));
            let w1 = Arc::new(ControlWorker::new(format!("{} Brightness", name), b1));
            let b2 = Box::new(DDCBackend::new(bus.clone(), 12));
            let w2 = Arc::new(ControlWorker::new(format!("{} Contrast", name), b2));
            let mut list = workers.lock().unwrap();
            list.push(w1);
            list.push(w2);
        }
    }
}
