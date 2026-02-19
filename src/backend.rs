use once_cell::sync::Lazy;
use std::env;
use std::path::Path;
use std::process::Command;

// Check for tools at startup
pub static DDC_BIN: Lazy<Option<String>> = Lazy::new(|| {
    if which("ddcutil").is_ok() {
        Some("ddcutil".to_string())
    } else {
        None
    }
});

pub static BRIGHTNESS_BIN: Lazy<Option<String>> = Lazy::new(|| {
    if which("brightnessctl").is_ok() {
        Some("brightnessctl".to_string())
    } else {
        None
    }
});

fn which(program: &str) -> std::result::Result<std::path::PathBuf, String> {
    if let Ok(path) = env::var("PATH") {
        for p in path.split(':') {
            let p_str = format!("{}/{}", p, program);
            if Path::new(&p_str).exists() {
                return Ok(std::path::PathBuf::from(p_str));
            }
        }
    }
    Err(format!("{} not found", program))
}

pub trait Backend: Send + Sync {
    fn get(&self) -> i32;
    fn set(&self, val: i32);
    fn box_clone(&self) -> Box<dyn Backend>;
}

impl Clone for Box<dyn Backend> {
    fn clone(&self) -> Box<dyn Backend> {
        self.box_clone()
    }
}

#[derive(Clone)]
pub struct BrightnessctlBackend {
    device: String,
}

impl BrightnessctlBackend {
    pub fn new(device: String) -> Self {
        Self { device }
    }
}

impl Backend for BrightnessctlBackend {
    fn box_clone(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }

    fn get(&self) -> i32 {
        if BRIGHTNESS_BIN.is_none() {
            return -1;
        }
        let bin = BRIGHTNESS_BIN.as_ref().unwrap();

        let output_curr = Command::new(bin).args(["-d", &self.device, "g"]).output();

        let output_max = Command::new(bin).args(["-d", &self.device, "m"]).output();

        if let (Ok(curr), Ok(max)) = (output_curr, output_max) {
            let c = String::from_utf8_lossy(&curr.stdout)
                .trim()
                .parse::<f32>()
                .unwrap_or(0.0);
            let m = String::from_utf8_lossy(&max.stdout)
                .trim()
                .parse::<f32>()
                .unwrap_or(1.0);
            if m == 0.0 {
                return -1;
            }
            return ((c / m) * 100.0) as i32;
        }
        -1
    }

    fn set(&self, val: i32) {
        if let Some(bin) = BRIGHTNESS_BIN.as_ref() {
            let _ = Command::new(bin)
                .args(["-d", &self.device, "s", &format!("{}%", val)])
                .output();
        }
    }
}

#[derive(Clone)]
pub struct DDCBackend {
    bus: String,
    code: String,
}

impl DDCBackend {
    pub fn new(bus: String, code: u8) -> Self {
        Self {
            bus,
            code: code.to_string(),
        }
    }
}

impl Backend for DDCBackend {
    fn box_clone(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }

    fn get(&self) -> i32 {
        if DDC_BIN.is_none() {
            return -1;
        }
        let bin = DDC_BIN.as_ref().unwrap();

        // ddcutil getvcp 10 --brief --bus 1 --sleep-multiplier .1
        let output = Command::new(bin)
            .args([
                "getvcp",
                &self.code,
                "--brief",
                "--bus",
                &self.bus,
                "--sleep-multiplier",
                "0.1",
            ])
            .output();

        if let Ok(res) = output {
            let s = String::from_utf8_lossy(&res.stdout);
            // Parse "VCP 10 C 100"
            if let Some(caps) = regex::Regex::new(r"C\s+(\d+)").unwrap().captures(&s) {
                if let Some(m) = caps.get(1) {
                    return m.as_str().parse().unwrap_or(-1);
                }
            }
        }
        -1
    }

    fn set(&self, val: i32) {
        if let Some(bin) = DDC_BIN.as_ref() {
            let _ = Command::new(bin)
                .args([
                    "setvcp",
                    &self.code,
                    &val.to_string(),
                    "--bus",
                    &self.bus,
                    "--noverify",
                ])
                .output();
        }
    }
}
