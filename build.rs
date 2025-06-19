// build.rs

use std::{env, fs};
use std::os::unix::fs::symlink;
use std::path::Path;

fn main() {
    // 1) SONAME 삽입
    let version = env!("CARGO_PKG_VERSION");
    println!(
        "cargo:rustc-link-arg=-Wl,-soname,libread_ahtx0_rs.so.{}",
        version
    );

    // 2) 빌드 완료 후 심볼릭 링크 생성
    let profile = env::var("PROFILE").unwrap();                // "debug" 또는 "release"
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let target_dir = Path::new(&manifest_dir).join("target").join(&profile);

    let so_path    = target_dir.join("libread_ahtx0_rs.so");
    let link_path  = target_dir.join(format!("libread_ahtx0_rs.so.{}", version));

    // 기존 링크 제거 후 새로 생성
    let _ = fs::remove_file(&link_path);
    symlink(&so_path, &link_path)
        .expect("Failed to create versioned .so symlink");
}
