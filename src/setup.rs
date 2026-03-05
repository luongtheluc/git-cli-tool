// setup.rs — Installer binary for the `repo` CLI tool.
//
// Copies the `repo` binary (found next to this executable) to ~/.repo/bin/
// and adds that directory to the user's persistent PATH:
//   - Windows: writes to HKCU\Environment\Path via the registry
//   - Unix:    appends an export line to ~/.profile

use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::{env, fs, path::{Path, PathBuf}};

fn main() -> Result<()> {
    println!("{}", "repo Setup Installer".bold());
    println!("{}", "─".repeat(40));
    println!();

    // Locate the `repo` binary sitting next to this setup executable.
    let source = find_source_binary()?;
    println!("  Source : {}", source.display());

    // Resolve install directory: ~/.repo/bin/
    let install_dir = get_install_dir()?;
    let dest = install_dir.join(source.file_name().expect("binary has a filename"));
    println!("  Target : {}", dest.display());
    println!();

    // Detect reinstall: binary already exists at the destination.
    let is_reinstall = dest.exists();
    if is_reinstall {
        println!("  {}", "Existing installation detected — updating binary...".yellow());
    }
    println!();

    // Create install directory if it doesn't exist.
    fs::create_dir_all(&install_dir).context("failed to create install directory")?;

    // Copy (overwrite) binary to install directory.
    fs::copy(&source, &dest).context("failed to copy repo binary")?;

    // Make the binary executable on Unix.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&dest, fs::Permissions::from_mode(0o755))
            .context("failed to set executable permissions")?;
    }

    if is_reinstall {
        println!("{} Updated  {}", "✓".green().bold(), dest.display());
    } else {
        println!("{} Installed {}", "✓".green().bold(), dest.display());
    }

    // Persist the install directory in the user PATH.
    add_to_path(&install_dir)?;

    println!();
    if is_reinstall {
        println!("{}", "Update complete!".bold().green());
        println!("  Run `repo --help` to verify the new version.");
    } else {
        println!("{}", "Installation complete!".bold().green());
        println!("  Restart your terminal, then run: repo --help");
    }

    Ok(())
}

/// Locate `repo` / `repo.exe` in the same directory as this setup binary.
fn find_source_binary() -> Result<PathBuf> {
    let exe = env::current_exe().context("cannot determine current exe path")?;
    let dir = exe.parent().context("exe has no parent directory")?;

    #[cfg(windows)]
    let name = "repo.exe";
    #[cfg(not(windows))]
    let name = "repo";

    let path = dir.join(name);
    if !path.exists() {
        bail!(
            "'{}' not found in {}.\nBuild with `cargo build --release` first.",
            name,
            dir.display()
        );
    }
    Ok(path)
}

/// Returns `~/.repo/bin/` as the install directory.
fn get_install_dir() -> Result<PathBuf> {
    Ok(home_dir()?.join(".repo").join("bin"))
}

/// Returns the current user's home directory.
fn home_dir() -> Result<PathBuf> {
    #[cfg(windows)]
    {
        env::var("USERPROFILE")
            .map(PathBuf::from)
            .context("USERPROFILE environment variable not set")
    }
    #[cfg(not(windows))]
    {
        env::var("HOME")
            .map(PathBuf::from)
            .context("HOME environment variable not set")
    }
}

/// Adds `dir` to the user's persistent PATH.
/// Skips if the directory is already present in the current PATH.
fn add_to_path(dir: &Path) -> Result<()> {
    let sep = if cfg!(windows) { ';' } else { ':' };
    let current = env::var("PATH").unwrap_or_default();

    // Check if already on PATH (case-insensitive on Windows).
    let already_present = current.split(sep).any(|segment| {
        let segment = segment.trim();
        #[cfg(windows)]
        return segment.eq_ignore_ascii_case(&dir.to_string_lossy());
        #[cfg(not(windows))]
        return Path::new(segment) == dir;
    });

    if already_present {
        println!("{} Already in PATH — nothing to do", "✓".green().bold());
        return Ok(());
    }

    #[cfg(windows)]
    add_to_path_windows(dir)?;

    #[cfg(not(windows))]
    add_to_path_unix(dir)?;

    Ok(())
}

/// Windows: append install dir to HKCU\Environment\Path via winreg.
#[cfg(windows)]
fn add_to_path_windows(dir: &Path) -> Result<()> {
    use winreg::{enums::*, RegKey};

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let env_key = hkcu
        .open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)
        .context("failed to open HKCU\\Environment registry key")?;

    // Read the current user PATH value; default to empty string if absent.
    let current: String = env_key.get_value("Path").unwrap_or_default();
    let dir_str = dir.to_string_lossy();

    let new_path = if current.is_empty() {
        dir_str.to_string()
    } else if current.ends_with(';') {
        format!("{}{}", current, dir_str)
    } else {
        format!("{};{}", current, dir_str)
    };

    env_key
        .set_value("Path", &new_path)
        .context("failed to write PATH to registry")?;

    println!(
        "{} Added to user PATH (HKCU\\Environment)",
        "✓".green().bold()
    );
    Ok(())
}

/// Unix: append an export line to ~/.profile.
#[cfg(not(windows))]
fn add_to_path_unix(dir: &Path) -> Result<()> {
    use std::io::Write;

    let home = home_dir()?;
    let profile = home.join(".profile");
    let dir_str = dir.to_string_lossy();

    // Avoid duplicate entries if the path already appears in the file.
    let existing = fs::read_to_string(&profile).unwrap_or_default();
    if existing.contains(dir_str.as_ref()) {
        println!("{} Already configured in ~/.profile", "✓".green().bold());
        return Ok(());
    }

    let export_line = format!(
        "\n# Added by repo setup\nexport PATH=\"{}:$PATH\"\n",
        dir_str
    );

    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&profile)
        .and_then(|mut f| f.write_all(export_line.as_bytes()))
        .context("failed to write to ~/.profile")?;

    println!("{} Added to PATH in ~/.profile", "✓".green().bold());
    Ok(())
}
