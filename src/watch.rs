use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Duration;

use notify::{RecommendedWatcher, RecursiveMode, Watcher};

use crate::app::{InterruptError, InterruptFlag};
use crate::prelude::*;
use crate::project::Project;

type NotifyResult = notify::Result<notify::Event>;

pub struct Watch {
    watcher: RecommendedWatcher,
    evt_rx: Receiver<NotifyResult>,
    test_barrier: Option<Arc<Barrier>>,
}

#[derive(Debug)]
pub struct WatchControl {
    test_barrier: Arc<Barrier>,
}

impl Watch {
    pub fn new() -> Result<Self> {
        let (evt_tx, evt_rx) = channel();

        let watcher = notify::recommended_watcher(move |res: NotifyResult| {
            match res {
                Ok(evt) if evt.kind.is_access() => {} // Ignore access events
                other => {
                    let _ = evt_tx.send(other);
                }
            }
        })?;

        Ok(Watch {
            watcher,
            evt_rx,
            test_barrier: None,
        })
    }

    /// Create with the test sync flag on, for testing.
    pub fn with_test_sync() -> Result<(Self, WatchControl)> {
        let mut this = Self::new()?;

        let test_barrier = Arc::new(Barrier::new(2));
        let control = WatchControl {
            test_barrier: test_barrier.clone(),
        };

        this.test_barrier = Some(test_barrier);
        Ok((this, control))
    }

    pub fn watch(
        &mut self,
        project: &Project,
        interrupt: InterruptFlag,
    ) -> Result<Option<Vec<PathBuf>>> {
        self.watch_files(project)?;

        // Synchronize with test code, if any
        self.test_barrier.as_deref().map(Barrier::wait);

        let paths = match interrupt.channel_recv(&self.evt_rx) {
            Ok(Some(res)) => res.context("Error watching files")?.paths,
            Ok(None) => bail!("Internal error: Channel receive failed"),
            Err(InterruptError) => return Ok(None),
        };

        // Delaying mechanism - don't return back until we've
        // seen no event for a timeout's duration.
        loop {
            thread::sleep(Duration::from_millis(250));

            if self.evt_rx.try_recv().is_ok() {
                // Drain all immediately available evts
                while self.evt_rx.try_recv().is_ok() {}
            } else {
                break;
            }
        }

        self.unwatch_files(project);
        Ok(Some(paths))
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
    /// Wait until the `Watch` starts watching files in the current iteration
    /// as part of `.watch()`.
    ///
    /// **To be used in tests.** This only works when `Watch` is created with `test_sync = true`.
    pub fn wait_watching(&self) {
        self.test_barrier.wait();
    }
}
