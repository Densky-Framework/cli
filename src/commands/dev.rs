use std::ffi::OsStr;
use std::{path::PathBuf, thread, time::Duration};
use std::{
    process,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use clap::{arg, value_parser, ArgMatches, Command, ValueHint};
use densky_core::manifest::Manifest;
use densky_core::views::view_discover;
use densky_core::{
    http::http_discover,
    utils::{join_paths, Fmt},
    CompileContext,
};

use crate::compiler::{process_view, write_aux_files};
use crate::progress;
use crate::watcher::{PollWatcher, WatchKind};

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

        let watching_path = target_path.clone();
        let mut watching_poll = PollWatcher::new(watching_path).unwrap();

        let target_path_main = target_path.clone();

        let compile_context = CompileContext {
            output_dir: join_paths(".densky", &target_path_main),
            routes_path: join_paths("src/routes", &target_path_main),
            views_path: join_paths("src/views", &target_path_main),
            static_path: join_paths("src/static", &target_path_main),
            verbose: true,
            static_prefix: "static/".to_owned(),
        };

        let progress = progress::create_spinner(Some("Discovering"));

        match write_aux_files(&compile_context) {
            Ok(_) => (),
            Err(e) => {
                eprintln!("Error on first build: {e}");
                return;
            }
        };
        progress.tick();

        let (http_container, http_tree) = http_discover(&compile_context);
        progress.tick();

        Manifest::update(&http_container, &compile_context).unwrap();
        progress.tick();

        let views = view_discover(&compile_context);

        progress.finish();
        for view in views {
            process_view(view);
        }

        println!(
            "\x1B[2J\x1B[1;1H{}\n",
            Fmt(|f| http_tree.lock().unwrap().display(f, &http_container))
        );

        let mut deno = process::Command::new("deno")
            .args(["run", "-A"])
            .arg(format!("{}/.densky/dev.ts", target_path.display()))
            .spawn()
            .expect("deno command failed to run");

        let term = Arc::new(AtomicBool::new(false));
        let sigint =
            signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&term)).unwrap();

        '_loop: loop {
            DevCommand::handle_update(&compile_context, &mut watching_poll);

            // wait to interrupt
            if term.load(Ordering::Relaxed) {
                // TODO: Check memory leaks on this line
                assert!(signal_hook::low_level::unregister(sigint));
                let _ = deno.kill(); // Err(): Command wasn't running
                return;
            }

            thread::sleep(Duration::from_millis(200));
        }
    }

    fn handle_update(compile_context: &CompileContext, watching_poll: &mut PollWatcher) {
        let event = watching_poll.poll();
        if event.len() != 0 {
            let (http_container, http_tree) = http_discover(&compile_context);
            let views = view_discover(&compile_context);

            println!(
                "\x1B[2J\x1B[1;1H{}\n",
                Fmt(|f| http_tree.lock().unwrap().display(f, &http_container))
            );

            for view in views {
                process_view(view);
            }

            match Manifest::update(&http_container, &compile_context) {
                Ok(_) => {}
                Err(err) => {
                    eprintln!("Error updating manifest: {err}")
                }
            }

            DevCommand::send_update(event.iter().map(|e| (e.kind.clone(), &e.path)));
        }
    }

    pub fn send_update<I, P>(files: I)
    where
        I: Iterator<Item = (WatchKind, P)>,
        P: AsRef<OsStr>,
    {
        let mut files_json = "[".to_owned();
        for file in files {
            use WatchKind::*;
            let kind = match file.0 {
                Create => "create",
                Remove => "remove",
                Modify => "modify",
            };
            files_json += "[\"";
            files_json += kind;
            files_json += "\",\"";
            files_json += file.1.as_ref().to_str().unwrap();
            files_json += "\"],";
        }
        files_json.pop();
        files_json += "]";
        // TODO: print good error
        let res = ureq::post("http://localhost:8000/$/dev")
            .set("Content-Type", "application/json")
            .send_string(&files_json);

        if let Err(err) = res {
            match err {
                ureq::Error::Status(_, _) => (),
                ureq::Error::Transport(err) => println!("[Dev Error] {}", err),
            }
        }
    }
}
