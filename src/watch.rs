use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::Duration;

use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};

use crate::error::*;
use crate::project::Project;

type Result<T, E = ErrorNotify> = std::result::Result<T, E>;

pub struct Watch {
    watcher: RecommendedWatcher,
    rx: Receiver<DebouncedEvent>,
}

#[derive(Debug)]
pub struct WatchCancellation(Sender<DebouncedEvent>);

#[derive(Debug)]
pub enum WatchEvent {
    Path(PathBuf),
    Pathless,
    Cancel,
}

impl Watch {
    pub fn new() -> Result<(Watch, WatchCancellation)> {
        let (tx, rx) = channel();
        let tx2 = tx.clone();

        // Note: Set notify's duration to zero,
        // it doesn't do the thing we want, see below...
        let watcher = Watcher::new(tx2, Duration::new(0, 0))?;

        let watch = Watch { watcher, rx };
        let cancellation = WatchCancellation(tx);
        Ok((watch, cancellation))
    }

    const DELAY_MS: u64 = 500;

    fn watch_inner(&mut self) -> Result<WatchEvent> {
        let res = self
            .rx
            .recv()
            .expect("Internal error: Channel receive failed");
        let res = match res {
            DebouncedEvent::Error(notify::Error::Generic(ref err), None) if err.is_empty() => {
                return Ok(WatchEvent::Cancel)
            }
            DebouncedEvent::Error(err, None) => return Err(err.into()),
            DebouncedEvent::Error(err, Some(path)) => {
                return Err(ErrorNotify::NotifyPath { path, source: err })
            }

            DebouncedEvent::NoticeWrite(path)
            | DebouncedEvent::NoticeRemove(path)
            | DebouncedEvent::Create(path)
            | DebouncedEvent::Write(path)
            | DebouncedEvent::Chmod(path)
            | DebouncedEvent::Remove(path)
            | DebouncedEvent::Rename(_, path) => Ok(WatchEvent::Path(path)),

            DebouncedEvent::Rescan => Ok(WatchEvent::Pathless),
        };

        // Delaying mechanism - don't return back until we've
        // seen no event for a timeout's duration.
        // This is used instead of notify's delay, which just delays
        // individual events (which eventually still arrive),
        // but we want to ignore them instead...
        loop {
            thread::sleep(Duration::from_millis(Self::DELAY_MS));

            if self.rx.try_recv().is_ok() {
                // Drain all immediately available evts
                while let Ok(_) = self.rx.try_recv() {}
            } else {
                break;
            }
        }

        res
    }

    pub fn watch(&mut self, project: &Project) -> Result<WatchEvent> {
        self.watch_files(project, true)?;
        let res = self.watch_inner();
        let _ = self.watch_files(project, false);
        res
    }

    fn watch_files(&mut self, project: &Project, watch: bool) -> Result<()> {
        for path in project.watch_paths() {
            if watch {
                self.watcher
                    .watch(path, RecursiveMode::NonRecursive)
                    .map_err(|source| ErrorNotify::NotifyPath {
                        path: path.into(),
                        source,
                    })?;
            } else {
                let _ = self.watcher.unwatch(path);
            }
        }

        Ok(())
    }
}

impl WatchCancellation {
    pub fn cancel(&self) {
        // Bit of a hack: We inject cancellation as an empty generic error:
        let err = notify::Error::Generic(String::new());
        let _ = self.0.send(DebouncedEvent::Error(err, None));
    }
}
