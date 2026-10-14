// Windows 发布模式隐藏额外控制台窗口。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    between_frames_lib::run()
}
