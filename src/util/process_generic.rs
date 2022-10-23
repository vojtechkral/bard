#![cfg(not(unix))]

use std::io;
use std::process::{ChildStderr, ChildStdout};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

use super::BinaryLines;

type LineSender = Sender<io::Result<Vec<u8>>>;
type LineReceiver = Receiver<io::Result<Vec<u8>>>;

fn read_thread<R>(read: R, sender: LineSender) -> JoinHandle<()>
where
    R: io::Read + Send + 'static,
{
    thread::spawn(move || {
        let mut lines = BinaryLines::new(read);
        while let Some(res) = lines.next() {
            if sender.send(res).is_err() {
                return;
            }
        }
    })
}

pub struct ProcessLines {
    rx: LineReceiver,
}

impl ProcessLines {
    pub fn new(stdout: ChildStdout, stderr: ChildStderr) -> Self {
        let (tx, rx) = mpsc::channel();
        read_thread(stdout, tx.clone());
        read_thread(stderr, tx);
        Self { rx }
    }

    pub fn read_line(&mut self) -> io::Result<Option<Vec<u8>>> {
        self.rx.recv().ok().transpose()
    }
}
