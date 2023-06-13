use std::{
    cell::RefCell,
    fs,
    path::{Path, PathBuf},
    rc::Rc,
};

use ahash::HashMap;

use super::PollWatcher;

#[inline(always)]
pub fn walk_dir(
    cwd: &PathBuf,
) -> (
    HashMap<PathBuf, Rc<RefCell<PollWatcher>>>,
    HashMap<PathBuf, u64>,
) {
    let mut files = HashMap::default();
    let mut childs = HashMap::default();
    let dir = fs::read_dir(&cwd).unwrap();
    // let dir = RecursiveDirIterator::from_root(&cwd).unwrap();

    for entry in dir.filter_map(Result::ok) {
        let entry = entry.path();
        if !PollWatcher::is_valid_filename(&entry) {
            continue;
        }

        if Path::is_dir(&entry) {
            childs.insert(
                entry.clone(),
                Rc::new(RefCell::new(PollWatcher::new(entry))),
            );
        } else {
            let hash = PollWatcher::get_hash(&entry);
            files.insert(entry, hash);
        }
    }

    (childs, files)
}
