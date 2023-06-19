use std::{
    fs,
    sync::{Arc, Mutex},
};

use densky_core::{
    http::{HttpLeaf, HttpTree},
    views::ViewLeaf,
    walker::{WalkerContainer, WalkerLeaf, WalkerTree},
};

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

pub fn process_http(http_tree: Arc<Mutex<WalkerTree>>, container: &mut WalkerContainer) {
    let mut http_tree = http_tree.lock().unwrap();

    let output = match HttpTree::generate_file(&mut http_tree, container) {
        Ok(o) => o,
        Err(e) => panic!("{:?}", e),
    };
    let output_path = &http_tree.output_path;
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
        process_http(container.get_tree(child).unwrap(), container);
    }
}

pub fn process_view(view: ViewLeaf) -> Option<()> {
    let output = view.generate_file()?;

    let output_path = view.output_path();
    println!("{}", output_path.display());
    let _ = fs::create_dir_all(output_path.parent().unwrap());
    fs::write(output_path, output.0).unwrap();

    Some(())
}
