mod backend;
mod control;
mod hardware;
mod ui;
use crate::control::ControlWorker;
use std::sync::{Arc, Mutex};

fn main() {
    let controls = Arc::new(Mutex::new(Vec::<Arc<ControlWorker>>::new()));
    hardware::detect_controls(controls.clone());
    if let Err(e) = ui::run(controls) {
        eprintln!("Error: {}", e);
    }
}
