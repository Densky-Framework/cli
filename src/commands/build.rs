use std::path::PathBuf;

use clap::{arg, value_parser, ArgMatches, Command, ValueHint};
use densky_core::{http::http_discover, utils::join_paths, CompileContext};

use crate::{
    compiler::{process_http, write_aux_files},
    progress,
};

pub struct BuildCommand;

impl BuildCommand {
    pub fn command() -> Command {
        Command::new("build").arg(
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

        println!("Building {}", target_path.display());

        let compile_context = CompileContext {
            output_dir: join_paths(".densky", &target_path),
            routes_path: join_paths("src/routes", &target_path),
            views_path: join_paths("src/views", &target_path),
            static_path: join_paths("src/static", &target_path),
            verbose: true,
            static_prefix: "static/".to_owned(),
        };

        let progress = progress::create_spinner(Some("Discovering"));

        match write_aux_files(&compile_context) {
            Ok(_) => (),
            Err(_) => {
                // let _ = first_build_tx.send(false);
                return;
            }
        };
        progress.tick();

        let (mut http_container, http_tree) = http_discover(&compile_context);

        progress.finish();

        let progress = progress::create_bar(http_container.id_tree(), "Compiling");

        process_http(http_tree.clone(), &mut http_container, Some(progress));
    }
}
