use wmc_library::{LibraryConfig, MediaKind};

fn main() -> anyhow::Result<()> {
    let path = LibraryConfig::location()?;
    let config = LibraryConfig::load(&path)?;
    println!("配置：{}", path.display());
    for kind in MediaKind::ALL {
        for root in config.roots(kind) {
            println!("{}：{}", kind.label(), root.display());
        }
    }
    let result = wmc_library::scan(&config);
    for kind in MediaKind::ALL {
        println!(
            "{}：{} 个文件",
            kind.label(),
            result.files.iter().filter(|file| file.kind == kind).count()
        );
    }
    println!("不可访问项：{}", result.skipped);
    Ok(())
}
