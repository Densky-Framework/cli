mod poll;
mod utils;

use std::{
    path::PathBuf,
    thread::{self, JoinHandle},
    time::Duration,
};

pub use poll::*;

use ahash::RandomState;
use once_cell::sync::Lazy;

static MAIN_HASHER: Lazy<RandomState> = Lazy::new(|| RandomState::with_seed(29384));

pub fn watch(cwd: &PathBuf) -> JoinHandle<()> {
    let cwd = cwd.to_path_buf();
    thread::spawn(move || {
        let mut poll = PollWatcher::new(cwd);
        poll.scheduling_poll(Duration::from_secs(1), |x| println!("{x:#?}"))
    })
}
