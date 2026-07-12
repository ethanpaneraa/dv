use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use crate::project;

pub struct Update {
    pub root: PathBuf,
    pub project: Option<project::Project>,
}

const POLL_INTERVAL: Duration = Duration::from_secs(2);

/// Spawns a background thread that re-diffs each of `roots` on a fixed interval and
/// sends the result back over a channel. Detached; runs for the process's lifetime
/// and exits on its own once the receiver is dropped (i.e. the app quits).
pub fn spawn(roots: Vec<PathBuf>, staged: bool) -> Receiver<Update> {
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || loop {
        std::thread::sleep(POLL_INTERVAL);
        for root in &roots {
            // A transient error (e.g. git briefly locked mid-commit) just means we
            // skip this root for this tick and try again next interval.
            if let Ok(project) = project::load(root, staged) {
                if tx
                    .send(Update {
                        root: root.clone(),
                        project,
                    })
                    .is_err()
                {
                    return;
                }
            }
        }
    });

    rx
}
