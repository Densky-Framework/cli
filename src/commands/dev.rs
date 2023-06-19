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

use crate::compiler::{process_http, process_view};
use crate::watcher::PollWatcher;

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

        let mut deno = process::Command::new("deno")
            .args(["run", "-A"])
            .arg(format!("{}/main.ts", target_path.display()))
            .spawn()
            .expect("deno command failed to run");

        let main = thread::spawn(move || -> Option<()> {
            let compile_context = CompileContext {
                output_dir: join_paths(".densky", &target_path),
                routes_path: join_paths("src/routes", &target_path),
                views_path: join_paths("src/views", &target_path),
                static_path: join_paths("src/static", &target_path),
                verbose: true,
                static_prefix: "static/".to_owned(),
            };

            let (mut http_container, http_tree) = http_discover(&compile_context).ok()?;
            let views = view_discover(&compile_context);

            println!(
                "{}\n",
                Fmt(|f| http_tree.lock().unwrap().display(f, &http_container))
            );

            process_http(http_tree.clone(), &mut http_container);
            for view in views {
                process_view(view);
            }

            '_loop: loop {
                if let Ok(event) = watch_event_rx.recv_timeout(Duration::from_millis(10)) {
                    DevCommand::send_update(event.iter().map(|e| &e.path));
                    println!("[Debug] {event:#?}");
                }

                if let Ok(_) = shutdown_rx_main.recv_timeout(Duration::from_millis(1)) {
                    println!("[Main] Shutdown");
                    break;
                }
            }

            None
        });

        let term = Arc::new(AtomicBool::new(false));
        let sigint =
            signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&term)).unwrap();

        // wait to interrupt
        while !term.load(Ordering::Relaxed) {}

        // TODO: Check memory leaks on this line
        assert!(signal_hook::low_level::unregister(sigint));

        let _ = deno.kill();
        let _ = (shutdown_threads(), watching.join(), main.join());
    }

    pub fn send_update<I, P>(files: I)
    where
        I: Iterator<Item = P>,
        P: AsRef<OsStr>,
    {
        let mut files_json = "[".to_owned();
        for file in files {
            files_json += "\"";
            files_json += file.as_ref().to_str().unwrap();
            files_json += "\",";
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
