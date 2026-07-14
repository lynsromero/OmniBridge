use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let msys2_bin = PathBuf::from(r"C:\msys64\mingw64\bin");
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    let target_dir = manifest_dir.join("target").join(&profile);

    println!("cargo:rerun-if-env-changed=MSYS2_BIN");

    let exe_path = target_dir.join("omnibridge.exe");
    if !exe_path.exists() {
        return;
    }

    let ldd_output = Command::new("ldd")
        .arg(exe_path.to_str().unwrap())
        .env("PATH", format!("{};{}", msys2_bin.to_str().unwrap(), std::env::var("PATH").unwrap_or_default()))
        .output();

    if let Ok(output) = ldd_output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.contains("mingw64") || line.contains("msys64") {
                if let Some(pos) = line.find("=>") {
                    let dll_path = line[pos + 2..].trim();
                    if let Some(pos) = dll_path.find(" (") {
                        let dll_path = &dll_path[..pos];
                        if dll_path.starts_with('/') {
                            let dll_name = PathBuf::from(dll_path).file_name().unwrap().to_str().unwrap().to_string();
                            let src = msys2_bin.join(&dll_name);
                            let dst = target_dir.join(&dll_name);
                            if src.exists() && !dst.exists() {
                                let _ = fs::copy(&src, &dst);
                            }
                            println!("cargo:rerun-if-changed={}", dll_name);
                        }
                    }
                }
            }
        }
    }
}
