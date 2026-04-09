use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const APP_NAME: &str = "PrintCountPay";
const DATA_DIR_ENV: &str = "PRINTCOUNTPAY_DATA_DIR";

#[derive(Debug, Clone)]
pub(crate) struct AppPaths {
    pub(crate) data_root: PathBuf,
    pub(crate) profiles_root: PathBuf,
    pub(crate) printers_file: PathBuf,
    pub(crate) manual_bills_file: PathBuf,
    pub(crate) counter_oids_file: PathBuf,
    pub(crate) poll_export_file: PathBuf,
    pub(crate) status: Option<String>,
}

pub(crate) fn resolve_app_paths() -> AppPaths {
    let install_root = executable_root().unwrap_or_else(fallback_root);
    let dev_root = development_root();
    let data_root = dev_root
        .clone()
        .or_else(data_root_from_env)
        .or_else(windows_app_data_root)
        .unwrap_or_else(|| install_root.clone());

    let profiles_root = data_root.join("profiles");
    let printers_file = data_root.join("printers.ron");
    let manual_bills_file = data_root.join("manual_bills.ron");
    let counter_oids_file = data_root.join("counter_oids.ron");
    let poll_export_file = data_root.join("polling_export.txt");

    let mut issues = Vec::new();
    if let Err(error) = fs::create_dir_all(&profiles_root) {
        issues.push(format!(
            "Failed to prepare data directory {}: {error}",
            data_root.display()
        ));
    } else if let Some(source_profiles) = profile_source_root(dev_root.as_ref(), &install_root) {
        if source_profiles != profiles_root
            && let Err(error) = seed_directory(&source_profiles, &profiles_root)
        {
            issues.push(format!(
                "Failed to initialize profiles from {}: {error}",
                source_profiles.display()
            ));
        }
    }

    AppPaths {
        data_root,
        profiles_root,
        printers_file,
        manual_bills_file,
        counter_oids_file,
        poll_export_file,
        status: (!issues.is_empty()).then(|| issues.join(" | ")),
    }
}

fn development_root() -> Option<PathBuf> {
    let cwd = env::current_dir().ok()?;
    let has_workspace_manifest = cwd.join("Cargo.toml").is_file();
    let has_profiles = cwd.join("profiles").is_dir();
    (has_workspace_manifest && has_profiles).then_some(cwd)
}

fn data_root_from_env() -> Option<PathBuf> {
    env::var_os(DATA_DIR_ENV).map(PathBuf::from)
}

fn windows_app_data_root() -> Option<PathBuf> {
    if !cfg!(target_os = "windows") {
        return None;
    }

    env::var_os("APPDATA")
        .or_else(|| env::var_os("LOCALAPPDATA"))
        .map(PathBuf::from)
        .map(|root| root.join(APP_NAME))
}

fn executable_root() -> Option<PathBuf> {
    let executable = env::current_exe().ok()?;
    executable.parent().map(Path::to_path_buf)
}

fn fallback_root() -> PathBuf {
    env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn profile_source_root(dev_root: Option<&PathBuf>, install_root: &Path) -> Option<PathBuf> {
    dev_root
        .map(|root| root.join("profiles"))
        .filter(|path| path.is_dir())
        .or_else(|| {
            let installed_profiles = install_root.join("profiles");
            installed_profiles.is_dir().then_some(installed_profiles)
        })
}

fn seed_directory(source: &Path, destination: &Path) -> io::Result<()> {
    if source == destination || !source.is_dir() {
        return Ok(());
    }

    fs::create_dir_all(destination)?;

    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            seed_directory(&source_path, &destination_path)?;
        } else if file_type.is_file() && should_seed_file(&source_path, &destination_path)? {
            if let Some(parent) = destination_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&source_path, &destination_path)?;
        }
    }

    Ok(())
}

fn should_seed_file(source: &Path, destination: &Path) -> io::Result<bool> {
    if !destination.exists() {
        return Ok(true);
    }

    let source_modified = fs::metadata(source)?.modified()?;
    let destination_modified = fs::metadata(destination)?.modified()?;
    Ok(source_modified > destination_modified)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::thread;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    fn temp_test_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("current time")
            .as_nanos();
        env::temp_dir().join(format!(
            "printcountpay-{name}-{}-{unique}",
            std::process::id()
        ))
    }

    #[test]
    fn seed_directory_overwrites_older_destination_files() {
        let root = temp_test_dir("seed-directory");
        let source = root.join("source");
        let destination = root.join("destination");
        let source_file = source.join("machines").join("printer.ron");
        let destination_file = destination.join("machines").join("printer.ron");

        fs::create_dir_all(destination_file.parent().expect("destination parent"))
            .expect("create destination directory");
        fs::write(&destination_file, "old-profile").expect("write destination file");

        thread::sleep(Duration::from_millis(1100));

        fs::create_dir_all(source_file.parent().expect("source parent"))
            .expect("create source directory");
        fs::write(&source_file, "new-profile").expect("write source file");

        seed_directory(&source, &destination).expect("seed profiles");

        assert_eq!(
            fs::read_to_string(&destination_file).expect("read destination file"),
            "new-profile"
        );

        let _ = fs::remove_dir_all(root);
    }
}
