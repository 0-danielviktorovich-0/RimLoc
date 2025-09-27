fn main() {
  // generate icons if missing (Tauri expects them under `icons/`)
  let out_dir = std::path::Path::new("icons");
  let _ = std::fs::create_dir_all(out_dir);
  let png = out_dir.join("icon.png");
  let ico_path = out_dir.join("icon.ico");
  if !png.exists() {
    generate_png(&png, 256, 256);
  }
  if !ico_path.exists() { generate_ico(&png, &ico_path); }
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
  for x in 0..w { img.put_pixel(x, 0, image::Rgba([20,30,40,255])); img.put_pixel(x, h-1, image::Rgba([20,30,40,255])); }
  for y in 0..h { img.put_pixel(0, y, image::Rgba([20,30,40,255])); img.put_pixel(w-1, y, image::Rgba([20,30,40,255])); }
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
