// release 构建不弹控制台窗口；debug 保留，方便看日志
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    plico_lib::run()
}
