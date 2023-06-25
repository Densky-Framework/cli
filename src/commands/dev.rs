use std::ffi::OsStr;
use std::{path::PathBuf, thread, time::Duration};
use std::{
    process,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SendError},
        Arc,
    },
};

use clap::{arg, value_parser, ArgMatches, Command, ValueHint};
use densky_core::views::view_discover;
use densky_core::{
    http::http_discover,
    utils::{join_paths, Fmt},
    CompileContext,
};
use indicatif::{ProgressBar, ProgressStyle};

use crate::compiler::{process_http, process_view, write_aux_files};
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

        let (shutdown_tx, shutdown_rx, shutdown_rx_main) = spmc_channel();
        let shutdown_threads = move || shutdown_tx(());
        let (watch_event_tx, watch_event_rx) = mpsc::channel();

        let watching_path = target_path.clone();
        let watching = thread::spawn(move || {
            let mut poll = PollWatcher::new(watching_path).unwrap();
            poll.scheduling_poll(Duration::from_millis(500), shutdown_rx, |x| {
                watch_event_tx.send(x).unwrap();
            })
        });

        let (first_build_tx, first_build_rx) = mpsc::sync_channel(1);
        let target_path_main = target_path.clone();
        let main = thread::spawn(move || -> Option<()> {
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
                Err(_) => {
                    let _ = first_build_tx.send(false);
                    return None;
                }
            };

            let (mut http_container, http_tree) = http_discover(&compile_context).ok()?;
            progress.tick();
            let views = view_discover(&compile_context);

            progress.finish();

            // let progress = ProgressBar::new(http_container.id_tree() as u64)
            let progress = progress::create_bar(http_container.id_tree(), "Compiling");

            process_http(
                http_tree.clone(),
                &mut http_container,
                Some(progress.clone()),
            );
            progress.finish();
            for view in views {
                process_view(view);
            }
            let _ = first_build_tx.send(true);

            println!(
                "\x1B[2J\x1B[1;1H{}\n",
                Fmt(|f| http_tree.lock().unwrap().display(f, &http_container))
            );

            '_loop: loop {
                if let Ok(event) = watch_event_rx.recv_timeout(Duration::from_millis(10)) {
                    densky_core::utils::new_import_hash();
                    let progress = progress::create_bar(http_container.id_tree(), "Compiling");

                    write_aux_files(&compile_context).unwrap();

                    process_http(
                        http_tree.clone(),
                        &mut http_container,
                        Some(progress.clone()),
                    );
                    progress.finish();
                    // for view in views {
                    //     process_view(view);
                    // }
                    DevCommand::send_update(event.iter().map(|e| (e.kind.clone(), &e.path)));
                }

                if let Ok(_) = shutdown_rx_main.recv_timeout(Duration::from_millis(1)) {
                    println!("[Main] Shutdown");
                    break;
                }
            }

            None
        });

        let shutdown_threads = move || {
            (
                shutdown_threads().unwrap(),
                watching.join().unwrap(),
                main.join().unwrap(),
            )
        };

        let successful = first_build_rx.recv().unwrap();
        if !successful {
            let _ = shutdown_threads();
            return;
        }

        let mut deno = process::Command::new("deno")
            .args(["run", "-A"])
            .arg(format!("{}/.densky/dev.ts", target_path.display()))
            .spawn()
            .expect("deno command failed to run");

        let term = Arc::new(AtomicBool::new(false));
        let sigint =
            signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&term)).unwrap();

        // wait to interrupt
        while !term.load(Ordering::Relaxed) {}

        // TODO: Check memory leaks on this line
        assert!(signal_hook::low_level::unregister(sigint));

        let _ = deno.kill();
        let _ = shutdown_threads();
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

fn spmc_channel<T>() -> (
    impl FnOnce(T) -> Result<(), SendError<T>>,
    Receiver<T>,
    Receiver<T>,
)
where
    T: Clone,
{
    let (tx1, rx1) = mpsc::channel();
    let (tx2, rx2) = mpsc::channel();

    return (
        move |v| {
            tx1.send(v.clone())?;
            tx2.send(v.clone())?;

            Ok(())
        },
        rx1,
        rx2,
    );
}
