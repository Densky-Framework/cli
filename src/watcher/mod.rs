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

// pub fn watch(cwd: &PathBuf) {
//     let files = walk_dir(&cwd);
//     let files = Arc::new(Mutex::new(files));
//
//     // let (tx, rx) = mpsc::channel::<WatchEvent>();
//
//     // let mut watcher = RecommendedWatcher::new(
//     //     move |res: Result<notify::Event>| match res {
//     //         Ok(raw_event) => {
//     //             println!("Change detected!");
//     //             let files = Arc::clone(&files);
//     //             let mut files = files.lock().unwrap();
//     //             let mut is_renaming = false;
//     //             // ignore it if is specific kinds
//     //             match raw_event.kind {
//     //                 EventKind::Access(_) => return,
//     //                 EventKind::Modify(ref k) => match k {
//     //                     ModifyKind::Data(_) => return,
//     //                     ModifyKind::Name(RenameMode::From | RenameMode::To) => return,
//     //                     ModifyKind::Name(RenameMode::Both) => is_renaming = true,
//     //                     _ => (),
//     //                 },
//     //                 _ => (),
//     //             };
//     //
//     //             let is_creating = raw_event.kind.is_create();
//     //
//     //             let mut paths: Option<PathBuf> = None;
//     //             let mut creating_file: Option<PathBuf> = None;
//     //             let mut removing_file: Option<PathBuf> = None;
//     //
//     //             for (idx, path) in raw_event.paths.iter().enumerate() {
//     //                 let is_nvim_cache = path.display().to_string().ends_with('~');
//     //                 let is_nvim_file = path.ends_with("4913");
//     //
//     //                 let will_pass = is_nvim_cache || is_nvim_file;
//     //
//     //                 if will_pass {
//     //                     return;
//     //                 }
//     //
//     //                 let path: PathBuf = normalize_path(path).into();
//     //
//     //                 let is_renaming_from = is_renaming && idx == 0;
//     //                 let is_renaming_to = is_renaming && idx == 1;
//     //
//     //                 let will_create = is_creating || is_renaming_to;
//     //                 let will_remove = raw_event.kind.is_remove() || is_renaming_from;
//     //
//     //                 let is_already_created = will_create && files.contains(&path);
//     //                 let not_exists = will_remove && !files.contains(&path);
//     //                 if is_already_created || not_exists {
//     //                     return;
//     //                 }
//     //
//     //                 if will_create {
//     //                     creating_file = Some(path.clone());
//     //                 }
//     //                 if will_remove {
//     //                     removing_file = Some(path.clone());
//     //                 }
//     //
//     //                 if idx == 0 {
//     //                     paths = Some(path);
//     //                 }
//     //             }
//     //
//     //             if let Some(ref creating_file) = creating_file {
//     //                 files.insert(creating_file.to_path_buf());
//     //             }
//     //             if let Some(removing_file) = removing_file {
//     //                 files.remove(&removing_file);
//     //             }
//     //
//     //             let event = WatchEvent {
//     //                 kind: match raw_event.kind {
//     //                     EventKind::Create(_) => WatchKind::Create,
//     //                     EventKind::Remove(_) => WatchKind::Remove,
//     //                     EventKind::Modify(ModifyKind::Name(_)) => {
//     //                         WatchKind::Rename(creating_file.unwrap())
//     //                     }
//     //                     EventKind::Modify(ModifyKind::Metadata(_)) => WatchKind::Modify,
//     //                     _ => unreachable!(),
//     //                 },
//     //                 path: if let Some(path) = paths { path } else { return },
//     //             };
//     //
//     //             // tx.send(event).unwrap();
//     //         }
//     //         Err(err) => println!("-----\n{:#?}", err),
//     //     },
//     //     Config::default(),
//     // )?;
//     let mut watcher = PollWatcher::new(
//         move |e| println!("{e:?}"),
//         Config::default().with_poll_interval(Duration::from_secs(1)),
//     )
//     .unwrap();
//     watcher.watch(&cwd, RecursiveMode::Recursive)?;
//
//     Ok(())
//     // Ok(rx)
// }
