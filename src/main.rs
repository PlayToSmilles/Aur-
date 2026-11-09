#![windows_subsystem = "windows"]

mod document;
mod html;
mod platform;
mod render;

use std::{path::PathBuf, process::ExitCode};

fn document_path() -> Result<PathBuf, String> {
    let mut args = std::env::args_os().skip(1);
    if let Some(path) = args.next() {
        if args.next().is_some() { return Err("Uso: Arua.exe [caminho-do-documento.html]".into()); }
        return Ok(PathBuf::from(path));
    }
    // The build is intentionally not an installer: default sample is in the
    // source checkout. An explicit path makes the executable independent of it.
    Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/mark_i.html"))
}

fn main() -> ExitCode {
    let result = document_path()
        .and_then(|path| html::load(&path).map_err(|e| e.to_string()))
        .and_then(|document| platform::run(document).map_err(|e| format!("Falha na janela ou renderização: {e}")));
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => { platform::show_error(&error); ExitCode::FAILURE },
    }
}
