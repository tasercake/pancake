//! Embed Explorer branding when building the native Windows executable.

#[cfg(all(windows, target_env = "msvc"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::{env, fs, io, path::PathBuf, process::Command};

    use image::{
        ExtendedColorType,
        codecs::ico::{IcoEncoder, IcoFrame},
        imageops::{self, FilterType},
    };

    const ICON: &str = "../../../assets/pancake-stacker-icon.png";

    // Build dependencies are host-specific. The supported Windows release is
    // built natively, and a Windows host may also build non-Windows targets.
    if env::var("CARGO_CFG_TARGET_OS")? != "windows" || env::var("CARGO_CFG_TARGET_ENV")? != "msvc"
    {
        return Ok(());
    }

    println!("cargo::rerun-if-changed={ICON}");
    println!("cargo::rerun-if-changed=windows.rc");

    let out = PathBuf::from(env::var_os("OUT_DIR").ok_or(io::Error::other("Missing OUT_DIR"))?);
    let original = image::open(ICON)?.into_rgba8();
    let frames = [16, 24, 32, 48, 64, 128, 256]
        .map(|size| {
            let pixels = imageops::resize(&original, size, size, FilterType::Lanczos3);
            IcoFrame::as_png(&pixels, size, size, ExtendedColorType::Rgba8)
        })
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    IcoEncoder::new(fs::File::create(out.join("pancake.ico"))?).encode_images(&frames)?;

    let host = env::var("HOST")?;
    let architecture = host.split('-').next().unwrap_or_default();
    let sdk = find_msvc_tools::find_windows_sdk(architecture)
        .ok_or(io::Error::other("Could not find the Windows SDK"))?;
    let compiler = sdk
        .path()
        .map(|directory| directory.join("rc.exe"))
        .find(|file| file.is_file())
        .ok_or(io::Error::other("Could not find rc.exe in the Windows SDK"))?;
    let resource = out.join("pancake.res");
    let version = format!(
        "{},{},{},0",
        env!("CARGO_PKG_VERSION_MAJOR"),
        env!("CARGO_PKG_VERSION_MINOR"),
        env!("CARGO_PKG_VERSION_PATCH")
    );
    let version_text = env!("CARGO_PKG_VERSION");
    let script = include_str!("windows.rc");
    fs::write(
        out.join("windows.rc"),
        format!(
            "#define PANCAKE_VERSION {version}\n#define PANCAKE_VERSION_TEXT \"{version_text}\\0\"\n{script}"
        ),
    )?;
    let status = Command::new(compiler)
        .current_dir(&out)
        .args(["/nologo", "/fo", "pancake.res", "windows.rc"])
        .status()?;
    if !status.success() {
        return Err(io::Error::other(format!("rc.exe failed: {status}")).into());
    }
    println!(
        "cargo::rustc-link-arg-bin=pancake-gui={}",
        resource.display()
    );
    Ok(())
}

#[cfg(not(all(windows, target_env = "msvc")))]
fn main() {}
