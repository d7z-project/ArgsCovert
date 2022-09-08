use libc::c_int;
use signal_hook::flag;
use signal_hook::iterator::exfiltrator::WithOrigin;
use signal_hook::iterator::{Handle, SignalsInfo};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;

pub struct UnixSignalHook {
    rx: Receiver<c_int>,
    handle: Handle,
    signal_accept: Arc<AtomicBool>,
}

impl UnixSignalHook {
    pub fn signals(&self) -> Vec<c_int> {
        let mut vec1: Vec<c_int> = vec![];
        if let Ok(item) = self.rx.try_recv() {
            vec1.push(item);
        };
        vec1
    }
}

impl UnixSignalHook {
    pub fn close(&self) {
        self.handle.close();
        self.signal_accept.swap(true, Ordering::Release);
    }
    pub fn new(signals: Vec<c_int>) -> Self {
        let (tx, rx): (Sender<c_int>, Receiver<c_int>) = channel();
        let signal_accept = Arc::new(AtomicBool::new(false));
        for signal in &signals {
            flag::register_conditional_default(*signal, Arc::clone(&signal_accept)).unwrap();
        }
        let mut info = SignalsInfo::<WithOrigin>::new(Vec::clone(&signals)).unwrap();
        let handle = info.handle();
        thread::spawn(move || {
            for item in &mut info {
                tx.send(item.signal).unwrap();
            }
        });
        UnixSignalHook {
            rx,
            handle,
            signal_accept,
        }
    }
}
