use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Duration;

use notify::{RecommendedWatcher, RecursiveMode, Watcher};

use crate::prelude::*;
use crate::project::Project;

#[derive(Debug)]
pub enum WatchEvent {
    Change(Vec<PathBuf>),
    Cancel,
    Error(Error),
}

impl WatchEvent {
    fn is_change(&self) -> bool {
        matches!(self, Self::Change(..))
    }
}

pub struct Watch {
    watcher: RecommendedWatcher,
    evt_rx: Receiver<WatchEvent>,
    test_barrier: Arc<Barrier>,
}

#[derive(Debug)]
pub struct WatchControl {
    evt_tx: Sender<WatchEvent>,
    test_barrier: Arc<Barrier>,
}

impl Watch {
    pub fn new(test_sync: bool) -> Result<(Self, WatchControl)> {
        let (evt_tx, evt_rx) = channel();
        let evt_tx2 = evt_tx.clone();

        let watcher = notify::recommended_watcher(move |res| {
            if let Some(evt) = Self::event_map(res) {
                let _ = evt_tx.send(evt);
            }
        })?;

        let test_barrier = Arc::new(Barrier::new(if test_sync { 2 } else { 1 }));
        let watch = Watch {
            watcher,
            evt_rx,
            test_barrier: test_barrier.clone(),
        };
        let control = WatchControl {
            evt_tx: evt_tx2,
            test_barrier,
        };

        Ok((watch, control))
    }

    const DELAY_MS: u64 = 500;

    fn event_map(res: notify::Result<notify::Event>) -> Option<WatchEvent> {
        let evt = match res.context("Error watching files") {
            Ok(evt) => evt,
            Err(err) => return Some(WatchEvent::Error(err)),
        };

        if evt.kind.is_access() {
            None
        } else {
            Some(WatchEvent::Change(evt.paths))
        }
    }

    pub fn watch(&mut self, project: &Project) -> Result<WatchEvent> {
        self.watch_files(project)?;

        // Synchronize with test code, if any
        self.test_barrier.wait();

        let evt = self
            .evt_rx
            .recv()
            .expect("Internal error: Channel receive failed");

        // Delaying mechanism - don't return back until we've
        // seen no event for a timeout's duration.
        if evt.is_change() {
            loop {
                thread::sleep(Duration::from_millis(Self::DELAY_MS));

                if self.evt_rx.try_recv().is_ok() {
                    // Drain all immediately available evts
                    while self.evt_rx.try_recv().is_ok() {}
                } else {
                    break;
                }
            }
        }

        self.unwatch_files(project);
        Ok(evt)
    }

    fn watch_files(&mut self, project: &Project) -> Result<()> {
        project.watch_paths().try_for_each(|path| {
            self.watcher
                .watch(path, RecursiveMode::NonRecursive)
                .context("Error watching files")
        })
    }

    fn unwatch_files(&mut self, project: &Project) {
        for path in project.watch_paths() {
            let _ = self.watcher.unwatch(path);
        }
    }
}

impl WatchControl {
    /// Tell the watch loop to break.
    pub fn cancel(&self) {
        let _ = self.evt_tx.send(WatchEvent::Cancel);
    }

    /// Wait until the `Watch` starts watching files in the current iteration
    /// as part of `.watch()`.
    /// This only works when `Watch` is created with `test_sync = true`.
    pub fn wait_watching(&self) {
        self.test_barrier.wait();
    }
}
