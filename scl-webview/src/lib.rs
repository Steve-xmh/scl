#![doc = "../README.md"]
#![forbid(missing_docs)]

#[cfg(target_os = "windows")]
use webview2::Controller;
#[cfg(target_os = "windows")]
use winapi::{
    shared::minwindef::*, shared::windef::*, um::libloaderapi::*, um::winbase::MulDiv,
    um::wingdi::*, um::winuser::*,
};

/// 一个动态的 [`Result`]
///
/// 是对 [`anyhow::Result`] 的包装
pub type DynResult<T = ()> = anyhow::Result<T>;

type OnUrlChangeCallback = fn(&str) -> bool;

/// 检查当前环境是否安装且支持使用 WebView
///
/// 除了 Windows 平台会检测 WebView2 是否存在，其余平台因为不安装对应库就不能运行所以默认返回 `true`
///
/// 注：如果是 Windows 平台没有安装 WebView2，可以指引用户通过
/// [MSEdge 安装包下载直链](https://go.microsoft.com/fwlink/?linkid=2069324&Channel=Stable&language=zh-cn)
/// 或
/// [WebView2 Runtime 安装包下载直链](https://go.microsoft.com/fwlink/p/?LinkId=2124703)
/// 来安装对应的 WebView2 运行时
pub fn is_supported() -> bool {
    #[cfg(target_os = "windows")]
    {
        webview2::get_available_browser_version_string(None).is_ok()
    }
    #[cfg(not(target_os = "windows"))]
    {
        true
    }
}

/// 一个可以显示单个浏览器窗口的对象
///
/// 通过设置链接回调来获取对应的链接
pub struct WebView {
    fixed_size: Option<(u32, u32)>,
    title: String,
    begin_url: String,
    url_change_callback: Option<OnUrlChangeCallback>,
}

#[cfg(target_os = "windows")]
fn set_dpi_aware() {
    unsafe {
        // Windows 10.
        let user32 = LoadLibraryA(b"user32.dll\0".as_ptr() as *const i8);
        let set_thread_dpi_awareness_context = GetProcAddress(
            user32,
            b"SetThreadDpiAwarenessContext\0".as_ptr() as *const i8,
        );
        if !set_thread_dpi_awareness_context.is_null() {
            let set_thread_dpi_awareness_context: extern "system" fn(
                DPI_AWARENESS_CONTEXT,
            )
                -> DPI_AWARENESS_CONTEXT = std::mem::transmute(set_thread_dpi_awareness_context);
            set_thread_dpi_awareness_context(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
            return;
        }
        // Windows 7.
        SetProcessDPIAware();
    }
}

#[cfg(target_os = "windows")]
fn utf_16_null_terminiated(x: &str) -> Vec<u16> {
    x.encode_utf16().chain(std::iter::once(0)).collect()
}

impl WebView {
    /// 创建一个默认空白页面的 [`WebView`]
    pub fn new() -> Self {
        Self::default()
    }

    /// 固定窗口大小
    pub fn set_fixed_size(&mut self, s: (u32, u32)) {
        self.fixed_size = Some(s)
    }

    /// 固定窗口大小
    pub fn fixed_size(mut self, s: (u32, u32)) -> Self {
        self.set_fixed_size(s);
        self
    }

    /// 设置一个链接变更回调函数，如果函数返回 `true` 则会关闭 WebView 并返回该链接
    ///
    /// 反之则继续运行 WebView
    pub fn set_on_url_change(&mut self, f: OnUrlChangeCallback) {
        self.url_change_callback = Some(f)
    }

    /// 设置第一个打开的链接
    pub fn begin_url(mut self, url: String) -> Self {
        self.set_begin_url(url);
        self
    }

    /// 设置第一个打开的链接
    pub fn set_begin_url(&mut self, url: String) {
        self.begin_url = url
    }

    /// 设置一个链接变更回调函数，如果函数返回 `true` 则会关闭 WebView 并返回该链接
    ///
    /// 反之则继续运行 WebView
    pub fn on_url_change(mut self, f: OnUrlChangeCallback) -> Self {
        self.set_on_url_change(f);
        self
    }

    /// 设置窗口标题
    pub fn set_title(&mut self, title: String) {
        self.title = title;
    }

    /// 设置窗口标题
    pub fn title(mut self, title: String) -> Self {
        self.set_title(title);
        self
    }

    /// 阻塞当前线程运行 WebView，当发生链接改变且已经设置回调函数时，将会返回退出时正在访问的链接
    ///
    /// 如果是用户关闭了窗口或发生其它情况，则返回空字符串
    pub fn run(&mut self) -> DynResult<String> {
        #[cfg(target_os = "windows")]
        {
            self.run_windows()
        }
        #[cfg(target_os = "linux")]
        {
            self.run_linux()
        }
        #[cfg(target_os = "macos")]
        {
            self.run_macos()
        }
    }

    #[cfg(target_os = "linux")]
    #[inline(always)]
    fn run_linux(&mut self) -> DynResult<String> {
        use std::sync::*;

        use ::glib::idle_add_once;
        use gtk::{prelude::*, WindowType};
        use webkit2gtk::{traits::*, *};

        let title = self.title.to_owned();
        let size = self.fixed_size.to_owned();
        let begin_url = self.begin_url.to_owned();
        let callback = self.url_change_callback.to_owned();

        // Channel
        let deleted = Arc::new(atomic::AtomicBool::new(false));
        let (tx, rx) = mpsc::channel();

        idle_add_once(move || {
            if !gtk::is_initialized_main_thread() {
                gtk::init().unwrap();
            }

            let win = Arc::new(gtk::Window::new(WindowType::Toplevel));
            let loaded = Arc::new(atomic::AtomicBool::new(false));

            win.set_resizable(false);
            win.set_title(&title);

            if let Some((width, height)) = size {
                win.set_size_request(width as _, height as _);
            } else {
                win.set_size_request(640, 480);
            }

            let w = webkit2gtk::WebView::new();
            w.load_uri(&begin_url);

            let _win = win.clone();
            let _tx = tx.clone();
            let _loaded = loaded.clone();
            w.connect_load_changed(move |w, load_event| {
                if load_event == LoadEvent::Finished {
                    let url = w.uri().unwrap().to_string();
                    if let Some(callback) = callback {
                        if callback(&url) {
                            _loaded.store(true, atomic::Ordering::SeqCst);
                            _win.close();
                            _tx.send(url).unwrap();
                        }
                    }
                }
            });

            let _deleted = deleted.clone();
            win.connect_destroy(move |_| {
                _deleted.store(true, atomic::Ordering::SeqCst);
                if !loaded.load(atomic::Ordering::SeqCst) {
                    tx.send("".into()).unwrap();
                }
            });

            win.add(&w);
            win.show_all();

            loop {
                gtk::main_iteration_do(false);
                if deleted.load(atomic::Ordering::SeqCst) {
                    break;
                }
            }
        });
        let result = rx.recv().unwrap();

        Ok(result)
    }

    #[cfg(target_os = "macos")]
    #[inline(always)]
    fn run_macos(&mut self) -> DynResult<String> {
        use std::sync::mpsc::Sender;

        use cocoa::{
            appkit::{
                NSBackingStoreType::NSBackingStoreBuffered, NSView, NSViewHeightSizable,
                NSViewWidthSizable, NSWindow, NSWindowStyleMask,
            },
            base::*,
            foundation::{NSAutoreleasePool, NSString, NSURL},
        };
        use core_graphics::geometry::{CGPoint, CGRect, CGSize};
        use darwin_webkit::{
            foundation::NSURLRequest,
            webkit::{WKUserContentController, WKWebViewConfiguration},
        };
        use objc::{
            declare::ClassDecl,
            runtime::{Object, Sel},
            *,
        };
        use once_cell::sync::Lazy;

        let size = self.fixed_size.unwrap_or((600, 800));
        let start_url = dbg!(self.begin_url.to_owned());
        let title = self.title.to_owned();
        let callback = self.url_change_callback.unwrap_or(|_| true);
        let (sx, rx) = std::sync::mpsc::channel::<String>();
        let sx = Box::new(sx);
        let sx = Box::leak(sx);

        unsafe {
            dispatch::Queue::main().exec_sync(move || {
                let rect = CGRect::new(
                    &CGPoint::new(0., 0.),
                    &CGSize::new(size.0 as _, size.1 as _),
                );
                let sx = dbg!(sx as *mut _) as usize;

                // === WebViewWindowControllerClass ===

                struct WebViewWindowControllerClass(*const objc::runtime::Class);
                unsafe impl Sync for WebViewWindowControllerClass {}
                unsafe impl Send for WebViewWindowControllerClass {}

                static WEBVIEW_WINDOW_CONTROLLER: Lazy<WebViewWindowControllerClass> = Lazy::new(|| unsafe {
                    let mut decl = ClassDecl::new("SCLWebViewWindowController", class!(NSWindowController)).unwrap();

                    decl.add_ivar::<id>("webview");
                    decl.add_ivar::<usize>("urlSender");

                    extern "C" fn window_will_close(this: &mut Object, _: Sel, _: id) {
                        unsafe {
                            println!("WindowWillClose");
                            let url_sender = *this.get_ivar::<usize>("urlSender");
                            if url_sender != 0 {
                                let sx = &mut *(url_sender as *mut Sender<String>);
                                let _ = dbg!(sx.send("".into()));

                                let webview = *this.get_ivar::<id>("webview");
                                if !webview.is_null() {
                                    (*webview).set_ivar("urlSender", 0usize);
                                }

                                println!("Dropping sender");
                                drop(Box::from_raw(sx));
                                println!("Empty URL Sent");
                            }
                            let webview = *this.get_ivar::<id>("webview");
                            if !webview.is_null() {
                                let _: id = msg_send![webview, release];
                            }
                            let _: id = msg_send![this, release];
                        }
                    }

                    decl.add_method(sel!(windowWillClose:), window_will_close as extern "C" fn(&mut Object, Sel, id),);

                    WebViewWindowControllerClass(decl.register())
                });

                // === SCLWebViewClass ===

                struct SCLWebViewClass(*const objc::runtime::Class);
                unsafe impl Sync for SCLWebViewClass {}
                unsafe impl Send for SCLWebViewClass {}

                static SCL_WEBVIEW: Lazy<SCLWebViewClass> = Lazy::new(|| unsafe {
                    let mut decl = ClassDecl::new("SCLWebView", class!(WKWebView)).unwrap();

                    decl.add_ivar::<*const libc::c_void>("urlChangeCallback");
                    decl.add_ivar::<usize>("urlSender");

                    extern "C" fn observe_value(this: &mut Object, _: Sel, _key_path: id, _of_object: id, _changed: id, _context: id) {
                        unsafe {
                            let url: id = msg_send![this, URL];
                            if url.is_null() {
                                // 出错了
                                println!("WARNING: URL is null!");
                                let win: id = msg_send![this, window];
                                let win_ctl: id = msg_send![win, windowController];
                                let url_sender = dbg!(*(*win_ctl).get_ivar::<usize>("urlSender"));
                                if url_sender != 0 {
                                    let sx = &mut *(url_sender as *mut Sender<String>);
                                    let _ = dbg!(sx.send("".into()));
                                    println!("Dropping sender");
                                    drop(Box::from_raw(sx));
                                    println!("Empty URL Sent");
                                }
                                (*win_ctl).set_ivar("urlSender", 0usize);
                                let _: id = msg_send![this, release];
                                win.close();
                                return;
                            }
                            let url: id = msg_send![url, absoluteString];
                            let url = NSString::UTF8String(url);
                            let url = std::ffi::CStr::from_ptr(url);
                            let url = url.to_str().unwrap_or_default().to_string();
                            println!("URL Has Changed: {}", url);
                            let callback: *const libc::c_void = *this.get_ivar("urlChangeCallback");
                            let callback: fn(&str) -> bool = std::mem::transmute(callback);
                            if dbg!((callback)(url.as_str())) {
                                let url_sender = dbg!(*this.get_ivar::<usize>("urlSender"));
                                if url_sender != 0 {
                                    let sx = &mut *(url_sender as *mut Sender<String>);
                                    let _ = dbg!(sx.send(url));
                                    println!("Dropping sender");
                                    drop(Box::from_raw(sx));
                                    println!("URL Sent");
                                }
                                let win: id = msg_send![this, window];
                                let win_ctl: id = msg_send![win, windowController];
                                (*win_ctl).set_ivar("urlSender", 0usize);
                                let _: id = msg_send![this, release];
                                win.close();
                            }
                        }
                    }

                    decl.add_method(sel!(observeValueForKeyPath:ofObject:change:context:), observe_value as extern "C" fn(&mut Object, Sel, id, id, id , id),);

                    SCLWebViewClass(decl.register())
                });

                // === Setup ==

                // 创建窗口
                let win_ctl_cls: id = msg_send![WEBVIEW_WINDOW_CONTROLLER.0, alloc];
                (*win_ctl_cls).set_ivar("urlSender", sx);
                let win_cls: id = msg_send![class!(NSWindow), alloc];
                let mask = NSWindowStyleMask::NSTitledWindowMask | NSWindowStyleMask::NSClosableWindowMask | NSWindowStyleMask::NSFullSizeContentViewWindowMask;
                let win: id = msg_send![win_cls, initWithContentRect: rect styleMask: mask  backing: NSBackingStoreBuffered defer: NO];
                // 设置窗口标题
                let title_string = NSString::alloc(nil)
                    .init_str(&title);
                win.setTitle_(title_string);
                let win_ctl: id = msg_send![win_ctl_cls, initWithWindow: win];
                // 挂载关闭事件，否则不能退出模态状态
                let ns_nc: id = msg_send![class!(NSNotificationCenter), defaultCenter];
                let notif_string = NSString::alloc(nil)
                    .init_str("NSWindowWillCloseNotification")
                    .autorelease();
                let _: id = msg_send![ns_nc, addObserver:win_ctl selector:sel!(windowWillClose:) name:notif_string object:nil];
                // 准备 WKWebView 视图
                let configuration = WKWebViewConfiguration::init(WKWebViewConfiguration::alloc(nil));
                // 我们不保留任何数据，以避免自动登录的问题
                let non_persistent_data_store: id = msg_send![class!(WKWebsiteDataStore), nonPersistentDataStore];
                let _: id = msg_send![configuration, setWebsiteDataStore: non_persistent_data_store];
                // (*configuration).set_ivar("websiteDataStore", non_persistent_data_store);
                let content_controller = WKUserContentController::init(WKUserContentController::alloc(nil));
                configuration.setUserContentController(content_controller);
                // let frame = cocoa::appkit::NSWindow::frame(win);
                let webview: id = msg_send![SCL_WEBVIEW.0, alloc];
                let webview: id = msg_send![webview, initWithFrame:rect configuration:configuration];
                // 把回调函数导入到 WebView 内
                (*webview).set_ivar("urlChangeCallback", callback as *const libc::c_void);
                (*webview).set_ivar("urlSender", sx);
                // 让视图和窗口一起缩放
                NSView::setAutoresizingMask_(webview, NSViewWidthSizable | NSViewHeightSizable);
                // 创建请求并让 WebView 去请求跳转
                let url = NSString::alloc(nil).init_str(start_url.as_str());
                let url = NSURL::alloc(nil).initWithString_(url);
                let req = NSURLRequest::alloc(nil).initWithURL_(url);
                let _: id = msg_send![webview, loadRequest: req];
                // 挂载链接变化事件
                let keypath_string = NSString::alloc(nil)
                    .init_str("URL")
                    .autorelease();
                let change_string = NSString::alloc(nil)
                    .init_str("NSKeyValueChangeNewKey")
                    .autorelease();
                let _: id = msg_send![webview, addObserver: webview forKeyPath: keypath_string options: change_string context: nil];
                // 设置窗口的视图为 WKWebView
                win.setContentView_(webview);
                win.makeKeyAndOrderFront_(nil);
                win.center();
            });
            println!("Waiting for URL");
            let url = rx.recv()?;
            println!("Got URL: {}", url);
            Ok(url)
        }
    }

    #[cfg(target_os = "windows")]
    #[inline(always)]
    fn run_windows(&mut self) -> DynResult<String> {
        use std::{cell::RefCell, ffi::OsStr, os::windows::prelude::OsStrExt, rc::Rc};

        use once_cell::sync::OnceCell;
        use winapi::{
            shared::ntdef::LPCWSTR,
            um::{
                processthreadsapi::OpenProcess, synchapi::WaitForSingleObject, winbase::INFINITE,
                winnt::SYNCHRONIZE,
            },
        };

        let user_data_path = std::env::current_exe()?
            .parent()
            .unwrap()
            .join(".SCL.WebView2")
            .to_string_lossy()
            .to_string();
        let user_data_path = std::path::Path::new(&user_data_path);
        std::fs::remove_dir_all(user_data_path).unwrap_or_default();
        let final_url = Rc::new(RefCell::new(String::new()));

        if webview2::get_available_browser_version_string(None).is_ok() {
            let controller = Rc::new(OnceCell::<Controller>::new());
            let controller_clone = controller.clone();

            let wnd_proc = move |hwnd, msg, w_param, l_param| match msg {
                WM_SIZE => {
                    if let Some(c) = controller.get() {
                        let mut r = unsafe { std::mem::zeroed() };
                        unsafe {
                            GetClientRect(hwnd, &mut r);
                        }
                        c.put_bounds(r).unwrap();
                    }
                    0
                }
                WM_MOVE => {
                    if let Some(c) = controller.get() {
                        let _ = c.notify_parent_window_position_changed();
                    }
                    0
                }
                // Optimization: don't render the webview when the window is minimized.
                WM_SYSCOMMAND if w_param == SC_MINIMIZE => {
                    if let Some(c) = controller.get() {
                        c.put_is_visible(false).unwrap();
                    }
                    unsafe { DefWindowProcW(hwnd, msg, w_param, l_param) }
                }
                WM_SYSCOMMAND if w_param == SC_RESTORE => {
                    if let Some(c) = controller.get() {
                        c.put_is_visible(true).unwrap();
                    }
                    unsafe { DefWindowProcW(hwnd, msg, w_param, l_param) }
                }
                // High DPI support.
                WM_DPICHANGED => unsafe {
                    let rect = *(l_param as *const RECT);
                    SetWindowPos(
                        hwnd,
                        std::ptr::null_mut(),
                        rect.left,
                        rect.top,
                        rect.right - rect.left,
                        rect.bottom - rect.top,
                        SWP_NOZORDER | SWP_NOACTIVATE,
                    );
                    0
                },
                WM_QUIT | WM_DESTROY => unsafe {
                    PostQuitMessage(0);
                    0
                },
                _ => unsafe { DefWindowProcW(hwnd, msg, w_param, l_param) },
            };

            set_dpi_aware();
            let h_instance = unsafe { GetModuleHandleW(std::ptr::null()) };
            let class_name = utf_16_null_terminiated("SCL_WebView2");

            let class = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                hCursor: unsafe { LoadCursorW(std::ptr::null_mut(), IDC_ARROW) },
                lpfnWndProc: Some(unsafe { wnd_proc_helper::as_global_wnd_proc(wnd_proc) }),
                lpszClassName: class_name.as_ptr(),
                hInstance: h_instance,
                hbrBackground: (COLOR_WINDOW + 1) as HBRUSH,
                ..unsafe { std::mem::zeroed() }
            };
            unsafe {
                if RegisterClassW(&class) == 0 {
                    panic!("RegisterClassW failed: {}", std::io::Error::last_os_error());
                }
            }

            let window_title = utf_16_null_terminiated(&self.title);
            let hdc = unsafe { GetDC(std::ptr::null_mut()) };
            let dpi = unsafe { GetDeviceCaps(hdc, LOGPIXELSX) };
            unsafe { ReleaseDC(std::ptr::null_mut(), hdc) };
            let (width, height) = unsafe {
                if let Some(s) = &self.fixed_size {
                    (
                        MulDiv(s.0 as i32, dpi, USER_DEFAULT_SCREEN_DPI),
                        MulDiv(s.1 as i32, dpi, USER_DEFAULT_SCREEN_DPI),
                    )
                } else {
                    (
                        MulDiv(600, dpi, USER_DEFAULT_SCREEN_DPI),
                        MulDiv(480, dpi, USER_DEFAULT_SCREEN_DPI),
                    )
                }
            };
            let hwnd = unsafe {
                CreateWindowExW(
                    WS_EX_DLGMODALFRAME,
                    class_name.as_ptr(),
                    window_title.as_ptr(),
                    WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
                    CW_USEDEFAULT,
                    CW_USEDEFAULT,
                    width,
                    height,
                    HWND_DESKTOP,
                    std::ptr::null_mut(),
                    h_instance,
                    std::ptr::null_mut(),
                )
            };
            if hwnd.is_null() {
                panic!(
                    "CreateWindowExW failed: {}",
                    std::io::Error::last_os_error()
                );
            }
            unsafe {
                let module = GetModuleHandleW(0usize as LPCWSTR);
                let small_icon = LoadImageW(
                    module,
                    OsStr::new("SCL_ICON")
                        .encode_wide()
                        .chain(Some(0))
                        .collect::<Vec<u16>>()
                        .as_ptr(),
                    IMAGE_ICON,
                    16,
                    16,
                    0,
                );
                let big_icon = LoadImageW(
                    module,
                    OsStr::new("SCL_ICON")
                        .encode_wide()
                        .chain(Some(0))
                        .collect::<Vec<u16>>()
                        .as_ptr(),
                    IMAGE_ICON,
                    32,
                    32,
                    0,
                );
                SendMessageW(hwnd, WM_SETICON, 0, small_icon as LPARAM);
                SendMessageW(hwnd, WM_SETICON, 1, big_icon as LPARAM);
                ShowWindow(hwnd, SW_SHOW);
                UpdateWindow(hwnd);
            }

            let begin_url = self.begin_url.to_owned();
            let callback = self.url_change_callback.take();
            let final_url = final_url.to_owned();
            let webview_pid = Rc::new(RefCell::new(0));
            let c_webview_pid = webview_pid.to_owned();

            webview2::Environment::builder()
                .with_user_data_folder(user_data_path)
                .build(move |env| {
                    let env = env?;
                    env.create_controller(hwnd, move |c| {
                        let c = c?;
                        let mut r = unsafe { std::mem::zeroed() };
                        unsafe {
                            GetClientRect(hwnd, &mut r);
                        }
                        c.put_bounds(r)?;
                        let w = c.get_webview()?;
                        *c_webview_pid.borrow_mut() = w.get_browser_process_id()?;
                        let s = w.get_settings()?;
                        s.put_are_default_context_menus_enabled(false)?;
                        s.put_are_dev_tools_enabled(false)?;
                        s.put_is_built_in_error_page_enabled(false)?;
                        w.add_navigation_starting(move |_, s| {
                            if let Some(callback) = &callback {
                                // println!("Navigation starting");
                                // println!("{:?}", s);
                                let uri = s.get_uri().unwrap_or_else(|_| "".into());
                                if callback(&uri) {
                                    let mut url = final_url.borrow_mut();
                                    *url = uri;
                                    unsafe {
                                        DestroyWindow(hwnd);
                                    }
                                }
                            }
                            Ok(())
                        })?;
                        w.navigate(&begin_url)?;
                        // println!("Navigated to {}", begin_url);
                        controller_clone.set(c).unwrap();
                        Ok(())
                    })?;
                    Ok(())
                })?;

            let mut msg: MSG = unsafe { std::mem::zeroed() };
            while unsafe { GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) } != 0 {
                unsafe {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            unsafe {
                if UnregisterClassW(class.lpszClassName, h_instance) == 0 {
                    panic!(
                        "UnregisterClassW failed: {}",
                        std::io::Error::last_os_error()
                    );
                }
            }
            let pid = webview_pid.borrow().to_owned();
            let user_data_path = user_data_path.to_owned();
            std::thread::spawn(move || {
                println!("Waiting WebView exitting");
                unsafe {
                    let ph = OpenProcess(SYNCHRONIZE, 0, pid);
                    WaitForSingleObject(ph, INFINITE);
                }
                println!("Removing WebView userdata");
                while user_data_path.is_dir() {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    std::fs::remove_dir_all(&user_data_path).unwrap_or_default();
                }
            });
        }
        let url = final_url.borrow().to_owned();
        Ok(url)
    }
}

impl Default for WebView {
    fn default() -> Self {
        Self {
            fixed_size: None,
            title: "SCL - WebView2".into(),
            begin_url: "http://afdian.net/@SteveXMH".into(),
            url_change_callback: None,
        }
    }
}

#[cfg(target_os = "windows")]
mod wnd_proc_helper {
    use std::cell::UnsafeCell;

    use super::*;

    struct UnsafeSyncCell<T> {
        inner: UnsafeCell<T>,
    }

    impl<T> UnsafeSyncCell<T> {
        const fn new(t: T) -> UnsafeSyncCell<T> {
            UnsafeSyncCell {
                inner: UnsafeCell::new(t),
            }
        }
    }

    impl<T: Copy> UnsafeSyncCell<T> {
        unsafe fn get(&self) -> T {
            self.inner.get().read()
        }

        unsafe fn set(&self, v: T) {
            self.inner.get().write(v)
        }
    }

    unsafe impl<T: Copy> Sync for UnsafeSyncCell<T> {}

    static GLOBAL_F: UnsafeSyncCell<usize> = UnsafeSyncCell::new(0);

    /// Use a closure as window procedure.
    ///
    /// The closure will be boxed and stored in a global variable. It will be
    /// released upon WM_DESTROY. (It doesn't get to handle WM_DESTROY.)
    pub unsafe fn as_global_wnd_proc<F: Fn(HWND, UINT, WPARAM, LPARAM) -> isize + 'static>(
        f: F,
    ) -> unsafe extern "system" fn(hwnd: HWND, msg: UINT, w_param: WPARAM, l_param: LPARAM) -> isize
    {
        let f_ptr = Box::into_raw(Box::new(f));
        GLOBAL_F.set(f_ptr as usize);

        unsafe extern "system" fn wnd_proc<F: Fn(HWND, UINT, WPARAM, LPARAM) -> isize + 'static>(
            hwnd: HWND,
            msg: UINT,
            w_param: WPARAM,
            l_param: LPARAM,
        ) -> isize {
            let f_ptr = GLOBAL_F.get() as *mut F;

            if msg == WM_DESTROY {
                drop(Box::from_raw(f_ptr));
                GLOBAL_F.set(0);
                PostQuitMessage(0);
                return 0;
            }

            if !f_ptr.is_null() {
                let f = &*f_ptr;

                f(hwnd, msg, w_param, l_param)
            } else {
                DefWindowProcW(hwnd, msg, w_param, l_param)
            }
        }

        wnd_proc::<F>
    }
}
