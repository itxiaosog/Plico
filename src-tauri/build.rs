fn main() {
    // tauri-build 默认只追踪 tauri.conf.json 与 capabilities，**不追踪图标**。
    // 后果：换图标后 build script 不重跑，out/resource.lib 仍是旧图标编译的结果，
    // 于是 plico.exe 内嵌旧图标，而 NSIS 安装包现场读 icon.ico 却是新图标
    // —— 表现为「安装包装出来还是旧 Logo」。显式声明依赖即可根治。
    println!("cargo:rerun-if-changed=icons/icon.ico");
    tauri_build::build()
}
