use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=../ico/icon.ico");

    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let icon_path = manifest_dir.join("../ico/icon.ico");
    if !icon_path.is_file() {
        panic!("missing Windows icon at {}", icon_path.display());
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("out dir"));
    let resource_script = out_dir.join("printcountpay-icon.rc");
    let resource_object = out_dir.join("printcountpay-icon.res");

    fs::write(
        &resource_script,
        format!("1 ICON \"{}\"\n", windows_resource_path(&icon_path)),
    )
    .expect("write resource script");

    compile_windows_resource(&resource_script, &resource_object);

    println!(
        "cargo:rustc-link-arg-bin=printcountpay-app={}",
        resource_object.display()
    );
}

fn windows_resource_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn compile_windows_resource(resource_script: &Path, resource_object: &Path) {
    let script = resource_script
        .to_str()
        .expect("resource script path should be UTF-8");
    let object = resource_object
        .to_str()
        .expect("resource object path should be UTF-8");

    for compiler in ["llvm-rc.exe", "rc.exe"] {
        let status = Command::new(compiler)
            .args(["/nologo", "/fo", object, script])
            .status();

        match status {
            Ok(status) if status.success() => return,
            Ok(_) => continue,
            Err(_) => continue,
        }
    }

    panic!("failed to compile Windows resources with llvm-rc.exe or rc.exe");
}
