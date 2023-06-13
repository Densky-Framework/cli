use std::cell::RefCell;
use std::os::unix::prelude::MetadataExt;
use std::rc::Rc;
use std::{fs, path::PathBuf, thread, time::Duration};

use ahash::{HashMap, HashSet};

use super::utils::walk_dir;
use super::MAIN_HASHER;

#[derive(Debug)]
pub enum WatchKind {
    Create,
    Remove,
    Modify,
    Rename(PathBuf),
}

static MEGABYTE: u64 = 1000000;
static FILE_SIZE_THRESHOLD: u64 = MEGABYTE * 20;

#[derive(Debug)]
pub struct WatchEvent {
    pub kind: WatchKind,
    pub is_dir: bool,
    pub path: PathBuf,
}

#[derive(Debug)]
pub struct PollWatcher {
    folder: PathBuf,
    childs: RefCell<HashMap<PathBuf, Rc<RefCell<PollWatcher>>>>,
    files: RefCell<HashMap<PathBuf, u64>>,
}

impl PollWatcher {
    pub fn new(folder: PathBuf) -> PollWatcher {
        let (childs, files) = walk_dir(&folder);
        PollWatcher {
            folder,
            files: RefCell::new(files),
            childs: RefCell::new(childs),
        }
    }

    pub fn get_hash_with_size(path: &PathBuf, size: u64) -> u64 {
        if size >= FILE_SIZE_THRESHOLD {
            size
        } else {
            let c = match fs::read_to_string(&path) {
                Ok(c) => c,
                Err(_) => return 0,
            };
            MAIN_HASHER.hash_one(c)
        }
    }

    pub fn get_hash(path: &PathBuf) -> u64 {
        let size = match fs::metadata(&path) {
            Ok(m) => m.size(),
            Err(_) => 0,
        };

        Self::get_hash_with_size(path, size)
    }

    pub fn poll(&mut self) -> Vec<WatchEvent> {
        let a = fs::read_dir(&self.folder).unwrap();
        let a = a.filter_map(Result::ok);

        let dirs = self.childs.borrow();
        let mut removed_dirs: HashSet<PathBuf> = HashSet::default();
        for d in dirs.keys() {
            removed_dirs.insert(d.clone());
        }
        drop(dirs);
        let files = self.files.borrow();
        let mut removed_files: HashSet<PathBuf> = HashSet::default();
        for f in files.keys() {
            removed_files.insert(f.clone());
        }
        drop(files);
        let mut events = Vec::new();
        for entry in a {
            let path = entry.path();
            if !Self::is_valid_filename(&path) {
                continue;
            }

            let (is_dir, size) = match entry.metadata() {
                Err(_) => (false, 0),
                Ok(m) => (m.is_dir(), m.size()),
            };

            if is_dir {
                removed_dirs.remove(&path);
                if let Some(child) = self.childs.borrow().get(&path) {
                    let mut diff = child.borrow_mut().poll();
                    events.append(&mut diff);
                } else {
                    self.childs.borrow_mut().insert(
                        path.clone(),
                        Rc::new(RefCell::new(PollWatcher::new(path.clone()))),
                    );
                    events.push(WatchEvent {
                        kind: WatchKind::Create,
                        is_dir: true,
                        path,
                    });
                }

                continue;
            }

            let file_hash = Self::get_hash_with_size(&path, size);

            removed_files.remove(&path);

            let mut files = self.files.borrow_mut();
            if let Some(old_hash) = files.get(&path) {
                if *old_hash != file_hash {
                    files.insert(path.clone(), file_hash);
                    events.push(WatchEvent {
                        kind: WatchKind::Modify,
                        is_dir: false,
                        path,
                    });
                }
            } else {
                files.insert(path.clone(), file_hash);
                events.push(WatchEvent {
                    kind: WatchKind::Create,
                    is_dir: false,
                    path,
                });
            }
        }

        for entry in removed_files {
            self.files.borrow_mut().remove(&entry);
            events.push(WatchEvent {
                kind: WatchKind::Remove,
                is_dir: false,
                path: entry.into(),
            });
        }
        for entry in removed_dirs {
            self.childs.borrow_mut().remove(&entry);
            events.push(WatchEvent {
                kind: WatchKind::Remove,
                is_dir: true,
                path: entry.into(),
            })
        }

        events
    }

    pub fn is_valid_filename(path: &PathBuf) -> bool {
        let is_nvim_cache = path.display().to_string().ends_with('~');
        let is_nvim_file = path.ends_with("4913");
        let is_output = path
            .components()
            .find(|c| c.as_os_str() == ".densky")
            .is_some();

        !is_nvim_cache && !is_nvim_file && !is_output
    }

    pub fn scheduling_poll<F>(&mut self, interval: Duration, mut f: F)
    where
        F: FnMut(Vec<WatchEvent>),
    {
        loop {
            let r = self.poll();
            if r.len() != 0 {
                f(r);
            }
            thread::sleep(interval);
        }
    }
}
