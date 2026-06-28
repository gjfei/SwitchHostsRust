use std::sync::Arc;

use gpui::*;
use gpui_component::{
    Root, TitleBar,
    button::{Button, ButtonVariants},
    h_flex, v_flex,
};

pub struct Example;
impl Render for Example {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .child(
                TitleBar::new().child(
                    h_flex()
                        .w_full()
                        .pr_2()
                        .justify_between()
                        .child("App with Custom title bar")
                        .child("Right Item"),
                ),
            )
            .child(
                div()
                    .id("window-body")
                    .p_5()
                    .size_full()
                    .items_center()
                    .justify_center()
                    .child("Hello, World!")
                    .child(
                        Button::new("ok")
                            .primary()
                            .label("Let's Go!")
                            .on_click(|_, _, _| println!("Clicked!")),
                    ),
            )
    }
}

fn window_options() -> WindowOptions {
    let mut titlebar = TitleBar::title_bar_options();
    titlebar.title = Some("SwitchHostsRust".into());

    WindowOptions {
        titlebar: Some(titlebar),
        app_id: Some("app.switchhostsrust.gpui".into()),
        icon: window_icon(),
        ..Default::default()
    }
}

/// GPUI `WindowOptions::icon` 目前仅 Linux X11 生效；macOS Dock 见 Packager.toml + `.app` 启动。
fn window_icon() -> Option<Arc<image::RgbaImage>> {
    let img = image::load_from_memory(ui_assets::app_icons::window_icon_png_bytes()).ok()?;
    Some(Arc::new(img.to_rgba8()))
}

fn open_main_window(cx: &mut App) {
    if !cx.windows().is_empty() {
        return;
    }
    cx.open_window(window_options(), |window, cx| {
        window.set_window_title("SwitchHostsRust");
        let view = cx.new(|_| Example);
        cx.new(|cx| Root::new(view, window, cx))
    })
    .expect("Failed to open window");
}

fn main() {
    let app = gpui_platform::application().with_assets(gpui_component_assets::Assets);

    app.on_reopen(|cx| open_main_window(cx));

    app.run(move |cx| {
        gpui_component::init(cx);
        open_main_window(cx);
    });
}
