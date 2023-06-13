use std::{path::PathBuf, thread, time::Duration};

use clap::{arg, value_parser, ArgMatches, Command, ValueHint};
use densky_core::utils::join_paths;

use crate::watcher::watch;

pub struct DevCommand;

impl DevCommand {
    pub fn command() -> Command {
        Command::new("dev").arg(
            arg!([folder] "Proyect folder")
                .default_value(".")
                .value_hint(ValueHint::DirPath)
                .value_parser(value_parser!(PathBuf)),
        )
    }

    pub fn process(matches: &ArgMatches) {
        let folder = matches.get_one::<PathBuf>("folder").unwrap();
        let cwd = std::env::current_dir().unwrap();
        let target_path: PathBuf = join_paths(folder, cwd).into();

        println!("Running on {}", target_path.display());
        watch(&target_path);

        thread::sleep(Duration::from_secs(20));

        // for event in watcher.iter() {
        //     println!("------\n {event:#?}");
        // }
    }

    pub fn send_update() {
        // TODO: print good error
        ureq::post("http://localhost:/$/dev")
            .send_string("SEND")
            .unwrap();
    }
}
