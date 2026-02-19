use crate::backend::Backend;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub struct ControlWorker {
    pub name: String,
    #[allow(dead_code)]
    backend: Box<dyn Backend>,
    current: Arc<AtomicI32>,
    target: Arc<AtomicI32>,
    running: Arc<AtomicBool>,
}

impl Drop for ControlWorker {
    fn drop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

impl ControlWorker {
    pub fn new(name: String, backend: Box<dyn Backend>) -> Self {
        let current = Arc::new(AtomicI32::new(-1));
        let target = Arc::new(AtomicI32::new(-1));
        let running = Arc::new(AtomicBool::new(true));

        let c_current = current.clone();
        let c_target = target.clone();
        let c_running = running.clone();
        let c_backend = backend.clone();

        thread::spawn(move || {
            // initial read
            let raw_val = c_backend.get();
            if raw_val != -1 {
                // Round to nearest 5
                let val = ((raw_val + 2) / 5) * 5;
                c_current.store(val, Ordering::SeqCst);
                c_target.store(val, Ordering::SeqCst);
            }

            // loop
            while c_running.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(50));

                let t = c_target.load(Ordering::SeqCst);
                let c = c_current.load(Ordering::SeqCst);

                if t != -1 && t != c {
                    c_backend.set(t);
                    c_current.store(t, Ordering::SeqCst);
                }
            }
        });

        Self {
            name,
            backend,
            current,
            target,
            running,
        }
    }

    pub fn set_target(&self, val: i32) {
        let v = val.clamp(0, 100);
        self.target.store(v, Ordering::SeqCst);
    }

    pub fn get_value(&self) -> i32 {
        self.target.load(Ordering::SeqCst)
    }

    pub fn is_ready(&self) -> bool {
        self.current.load(Ordering::SeqCst) != -1
    }
}
