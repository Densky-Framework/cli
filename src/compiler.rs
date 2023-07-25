use std::{
    fs, io,
    sync::{Arc, Mutex},
};

use densky_core::{
    http::{HttpLeaf, HttpTree},
    utils::{import_filename, join_paths},
    views::ViewLeaf,
    walker::{WalkerContainer, WalkerLeaf, WalkerTree},
    CompileContext,
};
use indicatif::ProgressBar;

pub fn write_aux_files(compile_context: &CompileContext) -> io::Result<()> {
    fs::create_dir_all(&compile_context.output_dir)?;

    let header = "// THIS FILE WAS GENERATED BY DENSKY-BACKEND (By Apika Luca)";
    // main.ts
    fs::write(join_paths("main.ts", &compile_context.output_dir), format!("{header}
import * as $Densky$ from \"densky/runtime.ts\";
import httpHandler from \"{http_main}\";

$Densky$.HTTPResponse.viewsPath = \"{}\";

export default async function requestHandler(req: $Densky$.HTTPRequest, conn: Deno.Conn): Promise<Response> {{
  return await httpHandler(req);
}}", join_paths("views", &compile_context.output_dir), http_main = import_filename("./http.main.ts")))?;

    // http.main.ts
    fs::write(join_paths("http.main.ts", &compile_context.output_dir), format!("{header}
import * as $Densky$ from \"densky/runtime.ts\";
import mainHandler from \"{http_index}\";

function toResponse (
  req: $Densky$.HTTPRequest,
  response: Response | $Densky$.HTTPError | Error | void
): Response {{
  if (response instanceof Error) 
    response = $Densky$.HTTPError.fromError(response);

  if (response instanceof $Densky$.HTTPError) 
    response = response.toResponse();

  if (response instanceof Response) 
    return new Response(response.body, {{
      status: response.status,
      statusText: response.statusText,
      headers: Object.fromEntries([...req.headers.entries(), ...response.headers.entries()]),
    }});

  throw new Error(\"Unreachable code\");
}}

export default async function requestHandler(req: $Densky$.HTTPRequest): Promise<Response> {{
  return toResponse(req, await mainHandler(req) ?? new $Densky$.HTTPError($Densky$.StatusCode.NOT_FOUND));
}}", http_index = import_filename("./http/_index.ts")))?;

    // dev.ts
    fs::write(
        join_paths("dev.ts", &compile_context.output_dir),
        format!(
            "{header}
import {{ DevServer }} from \"densky/dev.ts\";
import compileOptions from \"{config}\";

const server = new DevServer({{ port: 8000, verbose: true }}, compileOptions);

server.start();
",
            config = import_filename("../config.ts")
        ),
    )?;

    Ok(())
}

pub fn process_http_leaf(http_leaf: Arc<Mutex<WalkerLeaf>>) {
    let http_tree = http_leaf.lock().unwrap();
    let output = match HttpLeaf::generate_file(&http_tree) {
        Ok(o) => o,
        Err(e) => panic!("{:?}", e),
    };
    let output_path = &http_tree.output_path;
    let _ = fs::create_dir_all(output_path.parent().unwrap());
    fs::write(output_path, output).unwrap();
}

pub fn process_http(
    http_tree: Arc<Mutex<WalkerTree>>,
    container: &mut WalkerContainer,
    progress: Option<ProgressBar>,
) {
    let mut http_tree = http_tree.lock().unwrap();

    let output = match HttpTree::generate_file(&mut http_tree, container) {
        Ok(o) => o,
        Err(e) => panic!("{:?}", e),
    };
    let output_path = &http_tree.output_path;
    // println!("{}", output_path.display());
    let _ = fs::create_dir_all(output_path.parent().unwrap());
    fs::write(output_path, output).unwrap();

    let children = http_tree.children.clone();

    if let Some(fallback) = &http_tree.fallback {
        let fallback = container.get_leaf(*fallback).unwrap();
        process_http_leaf(fallback);
    }
    if let Some(middleware) = &http_tree.middleware {
        let middleware = container.get_leaf(*middleware).unwrap();
        process_http_leaf(middleware);
    }

    drop(http_tree);

    for child in children {
        process_http(
            container.get_tree(child).unwrap(),
            container,
            progress.clone(),
        );
        if let Some(ref progress_bar) = progress {
            progress_bar.inc(1);
        }
    }
}

pub fn process_view(view: ViewLeaf) -> Option<()> {
    let output = view.generate_file()?;

    let output_path = view.output_path();
    // println!("{}", output_path.display());
    let _ = fs::create_dir_all(output_path.parent().unwrap());
    fs::write(output_path, output.0).unwrap();

    Some(())
}
