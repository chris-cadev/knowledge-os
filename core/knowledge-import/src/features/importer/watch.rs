use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use super::adapter::ImportError;

pub struct DirectoryWatcher {
    watcher: RecommendedWatcher,
    rx: mpsc::Receiver<Result<Event, notify::Error>>,
    watched_path: PathBuf,
}

impl DirectoryWatcher {
    pub fn new(path: &Path, recursive: bool) -> Result<Self, ImportError> {
        let (tx, rx) = mpsc::channel();

        let mut watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.send(res);
            },
            Config::default().with_poll_interval(Duration::from_secs(2)),
        )
        .map_err(|e| ImportError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        let mode = if recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };

        watcher
            .watch(path, mode)
            .map_err(|e| ImportError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        Ok(Self {
            watcher,
            rx,
            watched_path: path.to_path_buf(),
        })
    }

    pub fn next_event(&self) -> Result<Option<Event>, ImportError> {
        match self.rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Ok(event)) => Ok(Some(event)),
            Ok(Err(e)) => Err(ImportError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                e,
            ))),
            Err(mpsc::RecvTimeoutError::Timeout) => Ok(None),
            Err(mpsc::RecvTimeoutError::Disconnected) => Ok(None),
        }
    }

    pub fn get_events(&self, timeout_ms: u64) -> Result<Vec<Event>, ImportError> {
        let mut events = Vec::new();
        let start = std::time::Instant::now();
        loop {
            if start.elapsed().as_millis() as u64 > timeout_ms {
                break;
            }
            match self.rx.recv_timeout(Duration::from_millis(50)) {
                Ok(Ok(event)) => {
                    if matches!(
                        event.kind,
                        EventKind::Create(_) | EventKind::Modify(_)
                    ) {
                        events.push(event);
                    }
                }
                Ok(Err(_)) => {}
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if !events.is_empty() {
                        break;
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        Ok(events)
    }
}

impl Drop for DirectoryWatcher {
    fn drop(&mut self) {
        let _ = self.watcher.unwatch(&self.watched_path);
    }
}
