/*
 * Copyright (c) 2022, Dragon's Zone Project. All rights reserved.
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in
 * all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NON-INFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;

use libc::c_int;
use signal_hook::flag;
use signal_hook::iterator::exfiltrator::WithOrigin;
use signal_hook::iterator::{Handle, SignalsInfo};

pub struct UnixSignalHook {
    rx: Receiver<c_int>,
    handle: Handle,
    signal_accept: Arc<AtomicBool>,
}

impl UnixSignalHook {
    pub fn signals(&self) -> Vec<c_int> {
        let mut watch_id: Vec<c_int> = vec![];
        if let Ok(item) = self.rx.try_recv() {
            watch_id.push(item);
        };
        watch_id
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
