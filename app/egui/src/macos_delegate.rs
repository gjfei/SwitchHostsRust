//! macOS Dock 重新打开 / 托盘显示窗口：AppKit 层操作。

use std::ffi::CStr;
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::Once;

use objc2::ffi::{class_addMethod, class_respondsToSelector};
use objc2::runtime::{AnyClass, AnyObject, Bool, Imp, Sel};
use objc2::sel;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSWindow};

static DOCK_SHOW_REQUESTED: AtomicBool = AtomicBool::new(false);
static QUIT_REQUESTED: AtomicBool = AtomicBool::new(false);
/// 与「关闭窗口时最小化到托盘」相反：未勾选时最后一个窗口关闭后应正常退出。
static TERMINATE_AFTER_LAST_WINDOW_CLOSED: AtomicBool = AtomicBool::new(false);
static HANDLER_INSTALLED: Once = Once::new();
static TRAY_POLL_INSTALLED: Once = Once::new();
static MAIN_NS_WINDOW: AtomicPtr<std::ffi::c_void> = AtomicPtr::new(ptr::null_mut());

const WINIT_DELEGATE_CLASS: &CStr = c"WinitApplicationDelegate";

/// 缓存 winit 主窗口（`NSApplication.windows()` 在隐藏后可能不可靠）。
pub fn register_main_ns_window(window: &NSWindow) {
    MAIN_NS_WINDOW.store(
        (window as *const NSWindow).cast_mut().cast(),
        Ordering::Release,
    );
}

/// 在 AppKit 层激活应用并显示主窗口（须在菜单/Dock 回调中同步调用）。
pub fn show_windows_at_appkit_level() {
    let Some(mtm) = MainThreadMarker::new() else {
        tracing::warn!("show_windows_at_appkit_level 不在主线程");
        return;
    };

    unsafe {
        let app = NSApplication::sharedApplication(mtm);
        app.unhide(None);
        #[allow(deprecated)]
        app.activateIgnoringOtherApps(true);

        let window_ptr = MAIN_NS_WINDOW.load(Ordering::Acquire);
        if !window_ptr.is_null() {
            let window = &*(window_ptr as *const NSWindow);
            window.makeKeyAndOrderFront(None);
        } else {
            let windows = app.windows();
            for i in 0..windows.count() {
                windows.objectAtIndex(i).makeKeyAndOrderFront(None);
            }
        }
    }

    wake_main_run_loop();
}

pub fn wake_main_run_loop() {
    use core_foundation::runloop::{CFRunLoopGetMain, CFRunLoopWakeUp};
    unsafe {
        CFRunLoopWakeUp(CFRunLoopGetMain());
    }
}

extern "C-unwind" fn application_should_handle_reopen(
    _this: *mut AnyObject,
    _sel: Sel,
    _sender: *mut AnyObject,
    _has_visible_windows: Bool,
) -> Bool {
    DOCK_SHOW_REQUESTED.store(true, Ordering::SeqCst);
    show_windows_at_appkit_level();
    Bool::new(true)
}

extern "C-unwind" fn application_should_terminate_after_last_window_closed(
    _this: *mut AnyObject,
    _sel: Sel,
    _sender: *mut AnyObject,
) -> Bool {
    Bool::new(TERMINATE_AFTER_LAST_WINDOW_CLOSED.load(Ordering::SeqCst))
}

/// 同步 macOS「最后窗口关闭后是否退出」策略。
pub fn sync_terminate_after_last_window_closed(tray_mini_window: bool) {
    TERMINATE_AFTER_LAST_WINDOW_CLOSED.store(!tray_mini_window, Ordering::SeqCst);
}

extern "C-unwind" fn application_should_terminate(
    _this: *mut AnyObject,
    _sel: Sel,
    _sender: *mut AnyObject,
) -> usize {
    QUIT_REQUESTED.store(true, Ordering::SeqCst);
    // NSTerminateNow
    1
}

pub fn quit_was_requested() -> bool {
    QUIT_REQUESTED.load(Ordering::SeqCst)
}

pub fn mark_quit_requested() {
    QUIT_REQUESTED.store(true, Ordering::SeqCst);
}

fn add_instance_method(cls: &AnyClass, selector: Sel, imp: Imp, types: &CStr) -> bool {
    unsafe {
        class_addMethod(
            (cls as *const AnyClass).cast_mut(),
            selector,
            imp,
            types.as_ptr(),
        )
        .as_bool()
    }
}

/// 向 winit delegate 注入 Dock 重新打开等行为（须在 EventLoop 创建之后调用）。
pub fn install_app_delegate() {
    HANDLER_INSTALLED.call_once(|| {
        let Some(cls) = AnyClass::get(WINIT_DELEGATE_CLASS) else {
            tracing::warn!(
                "未找到 {WINIT_DELEGATE_CLASS:?}，Dock 点击可能无法重新打开窗口"
            );
            return;
        };

        let reopen_sel = sel!(applicationShouldHandleReopen:hasVisibleWindows:);
        if !unsafe { class_respondsToSelector(cls, reopen_sel).as_bool() } {
            let added = add_instance_method(
                cls,
                reopen_sel,
                unsafe {
                    std::mem::transmute::<
                        extern "C-unwind" fn(*mut AnyObject, Sel, *mut AnyObject, Bool) -> Bool,
                        Imp,
                    >(application_should_handle_reopen)
                },
                c"B@:@B",
            );
            if !added {
                tracing::warn!("无法注入 applicationShouldHandleReopen");
            }
        }

        let terminate_sel = sel!(applicationShouldTerminateAfterLastWindowClosed:);
        if !unsafe { class_respondsToSelector(cls, terminate_sel).as_bool() } {
            let added = add_instance_method(
                cls,
                terminate_sel,
                unsafe {
                    std::mem::transmute::<
                        extern "C-unwind" fn(*mut AnyObject, Sel, *mut AnyObject) -> Bool,
                        Imp,
                    >(application_should_terminate_after_last_window_closed)
                },
                c"B@:@",
            );
            if !added {
                tracing::warn!("无法注入 applicationShouldTerminateAfterLastWindowClosed");
            }
        }

        let should_terminate_sel = sel!(applicationShouldTerminate:);
        if !unsafe { class_respondsToSelector(cls, should_terminate_sel).as_bool() } {
            let added = add_instance_method(
                cls,
                should_terminate_sel,
                unsafe {
                    std::mem::transmute::<
                        extern "C-unwind" fn(*mut AnyObject, Sel, *mut AnyObject) -> usize,
                        Imp,
                    >(application_should_terminate)
                },
                c"Q@:@",
            );
            if !added {
                tracing::warn!("无法注入 applicationShouldTerminate");
            }
        }
    });
}

pub fn take_dock_show_request() -> bool {
    DOCK_SHOW_REQUESTED.swap(false, Ordering::SeqCst)
}

/// 窗口隐藏时在 RunLoop 中轮询托盘事件（egui `update` 可能暂停）。
pub fn install_tray_runloop_poll() {
    TRAY_POLL_INSTALLED.call_once(|| {
        use core_foundation::base::kCFAllocatorDefault;
        use core_foundation::runloop::{
            kCFRunLoopAfterWaiting, kCFRunLoopBeforeWaiting, kCFRunLoopCommonModes,
            CFRunLoopAddObserver, CFRunLoopGetMain, CFRunLoopObserverContext,
            CFRunLoopObserverCreate,
        };
        use core_foundation_sys::base::Boolean;

        extern "C" fn poll_tray(
            _: *mut core_foundation::runloop::__CFRunLoopObserver,
            _: usize,
            _: *mut std::ffi::c_void,
        ) {
            crate::tray_native::poll_tray_events_on_runloop();
        }

        unsafe {
            let mut context = CFRunLoopObserverContext {
                version: 0,
                info: ptr::null_mut(),
                retain: None,
                release: None,
                copyDescription: None,
            };
            let observer = CFRunLoopObserverCreate(
                kCFAllocatorDefault,
                kCFRunLoopBeforeWaiting | kCFRunLoopAfterWaiting,
                1 as Boolean,
                0,
                poll_tray,
                &mut context,
            );
            if !observer.is_null() {
                CFRunLoopAddObserver(CFRunLoopGetMain(), observer, kCFRunLoopCommonModes);
            }
        }
    });
}
