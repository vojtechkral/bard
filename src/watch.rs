use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::Duration;

use anyhow::{Context, Error, Result};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};

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
    rx: Receiver<WatchEvent>,
}

#[derive(Debug)]
pub struct WatchCancellation(Sender<WatchEvent>);

impl Watch {
    pub fn new() -> Result<(Watch, WatchCancellation)> {
        let (tx, rx) = channel();
        let tx2 = tx.clone();

        let watcher = notify::recommended_watcher(move |res| {
            if let Some(evt) = Self::event_map(res) {
                let _ = tx.send(evt);
            }
        })?;

        let watch = Watch { watcher, rx };
        let cancellation = WatchCancellation(tx2);
        Ok((watch, cancellation))
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

        let evt = self
            .rx
            .recv()
            .expect("Internal error: Channel receive failed");

        // Delaying mechanism - don't return back until we've
        // seen no event for a timeout's duration.
        if evt.is_change() {
            loop {
                thread::sleep(Duration::from_millis(Self::DELAY_MS));

                if self.rx.try_recv().is_ok() {
                    // Drain all immediately available evts
                    while self.rx.try_recv().is_ok() {}
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
                .watch(path.as_std_path(), RecursiveMode::NonRecursive)
                .context("Error watching files")
        })
    }

    fn unwatch_files(&mut self, project: &Project) {
        for path in project.watch_paths() {
            let _ = self.watcher.unwatch(path.as_std_path());
        }
    }
}

impl WatchCancellation {
    pub fn cancel(&self) {
        let _ = self.0.send(WatchEvent::Cancel);
    }
}
