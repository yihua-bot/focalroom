//! macOS 状态栏入口：复用品牌图形的透明轮廓，由系统适配深浅色外观。
use tauri::{
    Manager,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
};

struct StatusMenu {
    open: MenuItem<tauri::Wry>,
    quit: MenuItem<tauri::Wry>,
}

fn icon() -> Result<tauri::image::Image<'static>, image::ImageError> {
    let mut rgba = image::load_from_memory(include_bytes!("../../public/focalroom-mark.png"))?
        .resize_exact(44, 44, image::imageops::FilterType::Lanczos3)
        .to_rgba8();
    for pixel in rgba.pixels_mut() {
        pixel[0] = 0;
        pixel[1] = 0;
        pixel[2] = 0;
    }
    Ok(tauri::image::Image::new_owned(rgba.into_raw(), 44, 44))
}

pub fn install(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let open = MenuItem::with_id(
        app,
        "focalroom.status.open",
        "Open FocalRoom",
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(
        app,
        "focalroom.status.quit",
        "Quit FocalRoom",
        true,
        None::<&str>,
    )?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&open, &separator, &quit])?;
    TrayIconBuilder::with_id("focalroom.status")
        .icon(icon()?)
        .icon_as_template(true)
        .tooltip("FocalRoom")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "focalroom.status.open" => {
                let result = (|| -> tauri::Result<()> {
                    app.show()?;
                    if let Some(window) = app.get_webview_window("main") {
                        window.unminimize()?;
                        window.show()?;
                        window.set_focus()?;
                    }
                    Ok(())
                })();
                if let Err(error) = result {
                    eprintln!("无法打开 FocalRoom 窗口：{error}");
                }
            }
            "focalroom.status.quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    // Tauri 管理图标生命周期，菜单项留作语言切换时更新。
    app.manage(StatusMenu { open, quit });
    Ok(())
}

pub fn set_locale(app: &tauri::AppHandle, locale: &str) -> tauri::Result<()> {
    let (open, quit) = match locale {
        "zh" => ("打开 FocalRoom", "退出 FocalRoom"),
        "ja" => ("FocalRoom を開く", "FocalRoom を終了"),
        "ko" => ("FocalRoom 열기", "FocalRoom 종료"),
        _ => ("Open FocalRoom", "Quit FocalRoom"),
    };
    let menu = app.state::<StatusMenu>();
    menu.open.set_text(open)?;
    menu.quit.set_text(quit)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn status_icon_has_a_visible_transparent_template_instead_of_a_square() {
        let icon = super::icon().unwrap();
        assert_eq!((icon.width(), icon.height()), (44, 44));
        let pixels: Vec<_> = icon.rgba().chunks_exact(4).collect();
        assert_eq!(pixels[0][3], 0);
        let visible = pixels.iter().filter(|pixel| pixel[3] > 128).count();
        assert!(visible > 100 && visible < pixels.len() / 2);
        assert!(pixels.iter().all(|pixel| pixel[..3] == [0, 0, 0]));
    }
}
