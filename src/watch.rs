use notify::*;
use std::{path::Path, time::Duration};

pub fn wath<P: AsRef<Path>, F: Fn()>(paths: &[P], work: F) -> anyhow::Result<()> {
    let (tx, rx) = std::sync::mpsc::channel();

    let config = Config::default().with_poll_interval(Duration::from_secs(1));
    let mut watcher = PollWatcher::new(tx, config).unwrap();

    for path in paths {
        watcher.watch(path.as_ref(), RecursiveMode::NonRecursive)?;
    }

    for _ in rx {
        work()
    }

    Ok(())
}
