use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use libc::c_int;
use signal_hook::flag;
use signal_hook::iterator::exfiltrator::WithOrigin;
use signal_hook::iterator::{Handle, SignalsInfo};

pub struct UnixSignalHook {
    pub rx: Receiver<usize>,
    pub handle: Handle,
}

impl UnixSignalHook {
    pub fn close(&self) {
        self.handle.close()
    }
    pub fn new(signals: Vec<c_int>) -> Self {
        let (tx, rx): (Sender<usize>, Receiver<usize>) = channel();
        let signal_accept = Arc::new(AtomicBool::new(false));
        for signal in &signals {
            flag::register_conditional_default(*signal, Arc::clone(&signal_accept)).unwrap();
        }
        let mut info = SignalsInfo::<WithOrigin>::new(Vec::clone(&signals)).unwrap();
        let handle = info.handle();
        thread::spawn(move || {
            for item in &mut info {
                tx.send(item.signal as usize).unwrap();
            }
        });
        UnixSignalHook {
            rx,
            handle,
        }
    }
}
