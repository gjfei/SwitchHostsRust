fn main() {
    let font = std::path::Path::new("assets/fonts/AlibabaPuHuiTi-3-55-Regular.ttf");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", font.display());
    if !font.is_file() {
        panic!(
            "缺少阿里巴巴普惠体字体文件: {}\n请先运行: ./scripts/sync-fonts.sh",
            font.display()
        );
    }
}
