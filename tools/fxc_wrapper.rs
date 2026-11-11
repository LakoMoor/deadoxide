//! Work around fxc include resolution outside the workspace on restricted Windows hosts.
//! Invoked through GPUI_FXC_PATH; the real compiler path is in DEADOXIDE_REAL_FXC.
use std::{env, ffi::OsString, fs, io, path::PathBuf, process::Command};

fn run() -> io::Result<i32> {
    let compiler = env::var_os("DEADOXIDE_REAL_FXC")
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "DEADOXIDE_REAL_FXC is not set"))?;
    let mut args: Vec<OsString> = env::args_os().skip(1).collect();
    if let Some(last) = args.last_mut() {
        let input = PathBuf::from(&*last);
        if input
            .file_name()
            .is_some_and(|name| name == "color_text_raster.hlsl")
        {
            let source_dir = input
                .parent()
                .ok_or_else(|| io::Error::other("shader has no parent"))?;
            let dest_dir = PathBuf::from(
                env::var_os("OUT_DIR").ok_or_else(|| io::Error::other("OUT_DIR is not set"))?,
            )
            .join("fxc-local-include");
            fs::create_dir_all(&dest_dir)?;
            for name in ["color_text_raster.hlsl", "alpha_correction.hlsl"] {
                fs::copy(source_dir.join(name), dest_dir.join(name))?;
            }
            *last = dest_dir.join("color_text_raster.hlsl").into_os_string();
        }
    }
    let status = Command::new(compiler).args(args).status()?;
    Ok(status.code().unwrap_or(1))
}

fn main() {
    match run() {
        Ok(code) => std::process::exit(code),
        Err(err) => {
            eprintln!("fxc wrapper: {err}");
            std::process::exit(1);
        }
    }
}
