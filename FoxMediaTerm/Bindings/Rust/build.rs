use std::env;
use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::Command;

const PRESET_ENVIRONMENT: &str = "FOX_MEDIA_TERM_CMAKE_PRESET";
const CONFIGURATION_ENVIRONMENT: &str = "FOX_MEDIA_TERM_CMAKE_CONFIG";

fn main() {
    let host = required_environment("HOST");
    let target = required_environment("TARGET");
    assert_eq!(
        host, target,
        "Fox Media Term requires a native build; run Cargo on the target platform instead of cross-compiling"
    );

    let manifest_directory = PathBuf::from(required_environment("CARGO_MANIFEST_DIR"));
    let project_directory = manifest_directory.join("../..");
    let output_directory = PathBuf::from(required_environment("OUT_DIR"));
    let build_directory = output_directory.join("cmake-build");
    let install_directory = output_directory.join("cmake-install");
    let preset = env::var(PRESET_ENVIRONMENT).unwrap_or_else(|_| default_preset().to_owned());
    let configuration =
        env::var(CONFIGURATION_ENVIRONMENT).unwrap_or_else(|_| default_configuration().to_owned());

    run(
        Command::new("cmake")
            .current_dir(&project_directory)
            .arg("--preset")
            .arg(&preset)
            .arg("-B")
            .arg(&build_directory)
            .arg(format!(
                "-DCMAKE_INSTALL_PREFIX={}",
                install_directory.display()
            ))
            .arg("-DBUILD_TESTING=OFF")
            .arg("-DVCPKG_MANIFEST_FEATURES="),
        "configure Fox Media Term",
    );
    run(
        Command::new("cmake")
            .arg("--build")
            .arg(&build_directory)
            .arg("--config")
            .arg(&configuration)
            .arg("--target")
            .arg("install"),
        "build and install Fox Media Term",
    );

    let library_directory = install_directory.join("lib");
    let runtime_directory = install_directory.join("bin");
    println!(
        "cargo:rustc-link-search=native={}",
        library_directory.display()
    );
    println!(
        "cargo:rustc-link-search=native={}",
        runtime_directory.display()
    );
    println!("cargo:rustc-link-lib=dylib=fox_media_term");

    // These arguments cover this crate's tests; applications set their own rpath.
    if cfg!(target_os = "linux") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
    } else if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path");
    }

    stage_runtime_libraries(&output_directory, &install_directory);
    emit_rerun_directives(&project_directory);
}

fn required_environment(name: &str) -> String {
    env::var(name).unwrap_or_else(|_| panic!("{name} is not set by Cargo"))
}

fn default_preset() -> &'static str {
    if cfg!(target_os = "windows") {
        "ClangCL"
    } else if cfg!(target_os = "macos") {
        "AppleClang"
    } else {
        "ClangLinux"
    }
}

fn default_configuration() -> &'static str {
    match env::var("PROFILE").as_deref() {
        Ok("debug") => "Debug",
        Ok("release") => "Release",
        _ => "RelWithDebInfo",
    }
}

fn run(command: &mut Command, action: &str) {
    let status = command
        .status()
        .unwrap_or_else(|error| panic!("failed to {action}: {error}"));
    assert!(status.success(), "failed to {action}: {status}");
}

fn stage_runtime_libraries(output_directory: &Path, install_directory: &Path) {
    let profile_directory = output_directory
        .ancestors()
        .nth(3)
        .expect("Cargo OUT_DIR does not have the expected target/profile/build/package/out layout");
    let dependency_directory = profile_directory.join("deps");
    fs::create_dir_all(&dependency_directory)
        .expect("cannot create Cargo dependency output directory");

    for source_directory in [install_directory.join("bin"), install_directory.join("lib")] {
        let Ok(entries) = fs::read_dir(source_directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let source = entry.path();
            if source.is_file() && is_runtime_library(&source) {
                copy_if_different(&source, profile_directory);
                copy_if_different(&source, &dependency_directory);
            }
        }
    }
}

fn is_runtime_library(path: &Path) -> bool {
    let name = path.file_name().and_then(OsStr::to_str).unwrap_or_default();
    if cfg!(target_os = "windows") {
        name.ends_with(".dll")
    } else if cfg!(target_os = "macos") {
        name.ends_with(".dylib")
    } else {
        name.contains(".so")
    }
}

fn copy_if_different(source: &Path, destination_directory: &Path) {
    let destination = destination_directory.join(
        source
            .file_name()
            .expect("installed runtime library does not have a file name"),
    );
    let should_copy = !files_have_same_contents(source, &destination).unwrap_or(false);
    if should_copy {
        fs::copy(source, &destination).unwrap_or_else(|error| {
            let running_process_hint = if cfg!(target_os = "windows") {
                " Close processes using this DLL and retry; Windows does not allow replacing a loaded DLL."
            } else {
                ""
            };
            panic!(
                "cannot stage {} as {}: {error}.{running_process_hint}",
                source.display(),
                destination.display()
            )
        });
    }
}

fn files_have_same_contents(left: &Path, right: &Path) -> io::Result<bool> {
    if fs::metadata(left)?.len() != fs::metadata(right)?.len() {
        return Ok(false);
    }

    let mut left = File::open(left)?;
    let mut right = File::open(right)?;
    let mut left_buffer = [0_u8; 64 * 1024];
    let mut right_buffer = [0_u8; 64 * 1024];
    loop {
        let left_count = left.read(&mut left_buffer)?;
        let right_count = right.read(&mut right_buffer)?;
        if left_count != right_count || left_buffer[..left_count] != right_buffer[..right_count] {
            return Ok(false);
        }
        if left_count == 0 {
            return Ok(true);
        }
    }
}

fn emit_rerun_directives(project_directory: &Path) {
    for path in [
        "CMakeLists.txt",
        "CMakePresets.json",
        "CMake",
        "Include",
        "Source",
        "Ports",
        "Triplets",
        "vcpkg.json",
        "vcpkg-configuration.json",
    ] {
        println!(
            "cargo:rerun-if-changed={}",
            project_directory.join(path).display()
        );
    }
    println!("cargo:rerun-if-env-changed={PRESET_ENVIRONMENT}");
    println!("cargo:rerun-if-env-changed={CONFIGURATION_ENVIRONMENT}");
    println!("cargo:rerun-if-env-changed=VCPKG_ROOT");
}
