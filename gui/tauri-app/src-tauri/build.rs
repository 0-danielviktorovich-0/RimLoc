fn main() {
    emit_build_identity();
    // generate icons if missing (Tauri expects them under `icons/`)
    let out_dir = std::path::Path::new("icons");
    let _ = std::fs::create_dir_all(out_dir);
    let png = out_dir.join("icon.png");
    let ico_path = out_dir.join("icon.ico");
    if !png.exists() {
        generate_png(&png, 256, 256);
    }
    if !ico_path.exists() {
        generate_ico(&png, &ico_path);
    }
    tauri_build::build()
}

fn generate_png(path: &std::path::Path, w: u32, h: u32) {
    let mut img = image::RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let t = x as f32 / w as f32;
            let r = (122.0 + 133.0 * t) as u8; // gradient
            let g = (162.0 + 50.0 * (1.0 - t)) as u8;
            let b = (247.0 - 80.0 * t) as u8;
            img.put_pixel(x, y, image::Rgba([r, g, b, 255]));
        }
    }
    // simple border
    for x in 0..w {
        img.put_pixel(x, 0, image::Rgba([20, 30, 40, 255]));
        img.put_pixel(x, h - 1, image::Rgba([20, 30, 40, 255]));
    }
    for y in 0..h {
        img.put_pixel(0, y, image::Rgba([20, 30, 40, 255]));
        img.put_pixel(w - 1, y, image::Rgba([20, 30, 40, 255]));
    }
    let _ = img.save(path);
}

fn generate_ico(png: &std::path::Path, out: &std::path::Path) {
    let img = image::open(png).expect("icon base png").to_rgba8();
    let (w, h) = img.dimensions();
    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);
    let image = ico::IconImage::from_rgba_data(w, h, img.into_raw());
    icon_dir.add_entry(ico::IconDirEntry::encode(&image).expect("encode ico"));
    let mut f = std::fs::File::create(out).expect("create ico");
    let _ = icon_dir.write(&mut f);
}

/// Build identity (owner soak-hardening §1): the binary carries its own
/// source truth so a running app can be identified independently of any
/// wrapper path. Reruns when HEAD moves; a dirty tree is marked explicitly
/// (never silently claimed as a clean commit).
fn emit_build_identity() {
    println!("cargo:rerun-if-changed=../../../.git/HEAD");
    println!("cargo:rerun-if-changed=build.rs");
    let repo = std::path::Path::new("../../..");
    let commit = git(repo, &["rev-parse", "HEAD"]).unwrap_or_else(|| "unknown".into());
    let dirty = git(repo, &["status", "--porcelain"])
        .map(|o| !o.trim().is_empty())
        .unwrap_or(false);
    let commit = if dirty { format!("{commit}-dirty") } else { commit };
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "unknown".into());
    println!("cargo:rustc-env=RIMLOC_SOURCE_COMMIT={commit}");
    println!("cargo:rustc-env=RIMLOC_BUILD_PROFILE={profile}");
    println!(
        "cargo:rustc-env=RIMLOC_BUILD_FEATURES={}",
        std::env::var("RIMLOC_BUILD_FEATURES").unwrap_or_default()
    );
}

fn git(repo: &std::path::Path, args: &[&str]) -> Option<String> {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        None
    }
}
