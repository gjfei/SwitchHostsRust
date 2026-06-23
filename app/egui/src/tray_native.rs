//! 原生系统托盘（tray-icon + muda）。

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

use switch_hosts_core::storage::manifest::Manifest;
use tray_icon::menu::{
    CheckMenuItem, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem,
};
use tray_icon::{TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::tray::build_tray_menu;

/// 托盘菜单触发的动作。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrayAction {
    ShowWindow,
    Quit,
    ToggleScheme(String),
}

#[derive(Clone)]
struct MenuIds {
    show_window: MenuId,
    quit: MenuId,
    schemes: HashMap<MenuId, String>,
}

static PENDING_TRAY_ACTIONS: Mutex<VecDeque<TrayAction>> = Mutex::new(VecDeque::new());

fn push_tray_action(action: TrayAction) {
    if let Ok(mut pending) = PENDING_TRAY_ACTIONS.lock() {
        pending.push_back(action);
    }
}

/// 轮询托盘图标事件（菜单点击由 `MenuEvent::set_event_handler` 同步处理）。
pub fn poll_tray_events_on_runloop() {
    while let Ok(event) = TrayIconEvent::receiver().try_recv() {
        if let TrayIconEvent::DoubleClick { .. } = event {
            dispatch_tray_action(Some(TrayAction::ShowWindow));
        }
    }
}

fn dispatch_tray_action(action: Option<TrayAction>) {
    let Some(action) = action else {
        return;
    };

    match &action {
        TrayAction::ShowWindow => {
            #[cfg(target_os = "macos")]
            crate::macos::show_main_window();
            push_tray_action(action);
        }
        TrayAction::Quit => {
            perform_immediate_quit();
        }
        TrayAction::ToggleScheme(_) => {
            push_tray_action(action);
        }
    }

    #[cfg(target_os = "macos")]
    crate::macos_delegate::wake_main_run_loop();
}

/// 托盘「退出」须同步执行：窗口隐藏时 egui `update` 可能长时间不运行。
fn perform_immediate_quit() {
    #[cfg(target_os = "macos")]
    {
        crate::macos_delegate::mark_quit_requested();
        crate::macos::quit_app();
    }
    #[cfg(not(target_os = "macos"))]
    push_tray_action(TrayAction::Quit);
}

fn install_menu_event_handler(ids: &MenuIds) {
    let show_window = ids.show_window.clone();
    let quit = ids.quit.clone();
    let schemes = ids.schemes.clone();
    let menu_ids = MenuIds {
        show_window,
        quit,
        schemes,
    };

    MenuEvent::set_event_handler(Some(move |event| {
        dispatch_tray_action(map_menu_event(&event, &menu_ids));
    }));
}

fn register_menu_ids(ids: &MenuIds) {
    install_menu_event_handler(ids);
    #[cfg(target_os = "macos")]
    crate::macos_delegate::install_tray_runloop_poll();
}

/// 从托盘事件队列取动作。
pub fn try_recv_tray_action() -> Option<TrayAction> {
    PENDING_TRAY_ACTIONS.lock().ok()?.pop_front()
}

fn map_menu_event(event: &MenuEvent, ids: &MenuIds) -> Option<TrayAction> {
    if event.id == ids.show_window {
        return Some(TrayAction::ShowWindow);
    }
    if event.id == ids.quit {
        return Some(TrayAction::Quit);
    }
    ids.schemes
        .get(&event.id)
        .cloned()
        .map(TrayAction::ToggleScheme)
}

/// 原生托盘控制器；持有 `TrayIcon` 生命周期。
pub struct TrayController {
    tray: TrayIcon,
    ids: MenuIds,
}

impl TrayController {
    /// 创建托盘；须在 eframe 事件循环已启动后调用（见 `try_init_tray`）。
    fn try_build(manifest: &Manifest) -> Option<Self> {
        let icon = crate::app_icon::tray_icon();
        let (menu, ids) = build_native_menu(manifest)?;
        let mut builder = TrayIconBuilder::new()
            .with_tooltip("SwitchHostsRust")
            .with_icon(icon)
            .with_menu(Box::new(menu));
        #[cfg(target_os = "macos")]
        {
            builder = builder.with_icon_as_template(true);
        }
        let tray = match builder.build() {
            Ok(tray) => tray,
            Err(err) => {
                tracing::error!("创建系统托盘失败: {err}");
                return None;
            }
        };
        register_menu_ids(&ids);
        Some(Self { tray, ids })
    }

    /// 方案变更后刷新托盘菜单。
    pub fn refresh(&mut self, manifest: &Manifest) {
        if let Some((menu, ids)) = build_native_menu(manifest) {
            self.tray.set_menu(Some(Box::new(menu)));
            self.ids = ids;
            register_menu_ids(&self.ids);
        }
    }
}

/// 延迟到 EventLoop 运行后再创建托盘（tray-icon 在 macOS 上要求如此）。
pub fn try_init_tray(app: &mut Option<TrayController>, manifest: &Manifest) -> bool {
    if app.is_some() {
        return true;
    }
    if std::env::var("SWITCH_HOSTS_RUST_DISABLE_TRAY").is_ok() {
        return false;
    }
    match TrayController::try_build(manifest) {
        Some(tray) => {
            *app = Some(tray);
            true
        }
        None => false,
    }
}

fn build_native_menu(manifest: &Manifest) -> Option<(Menu, MenuIds)> {
    let menu = Menu::new();
    let show_window = MenuItem::new("显示主窗口", true, None);
    let quit = MenuItem::new("退出", true, None);
    menu.append(&show_window).ok()?;
    menu.append(&PredefinedMenuItem::separator()).ok()?;

    let mut schemes = HashMap::new();
    for entry in build_tray_menu(manifest) {
        let item = CheckMenuItem::new(entry.label, true, entry.checked, None);
        let id = item.id().clone();
        menu.append(&item).ok()?;
        schemes.insert(id, entry.id);
    }

    menu.append(&PredefinedMenuItem::separator()).ok()?;
    menu.append(&quit).ok()?;

    Some((
        menu,
        MenuIds {
            show_window: show_window.id().clone(),
            quit: quit.id().clone(),
            schemes,
        },
    ))
}
