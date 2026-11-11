//! Wry calls Tauri's `on_navigation` for every frame, iframes included, and gives it
//! only the URL. Moodle pages embed cross-origin content (Genially, video players,
//! H5P…), which the top-level policy would cancel and open in the browser on every
//! page load. This proxy decides embedded-frame navigations itself and forwards
//! everything else, including all other delegate methods, to Tauri/Wry unchanged.
//!
//! It also decides top-level responses through the public
//! `WKNavigationResponsePolicy::Download`: a PDF or image the user chose to save, or
//! the file Save to Downloads asked for again. With the default preferences every
//! response is forwarded untouched. WebKit then hands the
//! transfer to Wry's download delegate, so it follows the one managed download path.
//! This compatibility adapter requires regression checks across macOS releases.
use objc2::{
    define_class, msg_send,
    rc::Retained,
    runtime::{AnyObject, NSObject, ProtocolObject, Sel},
    DefinedClass, MainThreadOnly,
};
use objc2_foundation::{MainThreadMarker, NSHTTPURLResponse, NSObjectProtocol, NSString};
use objc2_web_kit::{
    WKDownload, WKNavigationAction, WKNavigationActionPolicy, WKNavigationDelegate,
    WKNavigationResponse, WKNavigationResponsePolicy, WKWebView,
};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    time::Instant,
};
use tauri::{AppHandle, Emitter, Manager};

struct Ivars {
    original: Retained<ProtocolObject<dyn WKNavigationDelegate>>,
    app: AppHandle,
    site: String,
    base: url::Url,
    /// Set by Save to Downloads; the next top-level response is saved, once.
    save_next: Cell<Option<Instant>>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = Ivars]
    struct FrameGuard;
    unsafe impl NSObjectProtocol for FrameGuard {
        #[unsafe(method(respondsToSelector:))]
        fn responds(&self, selector: Sel) -> bool {
            let own: bool = unsafe { msg_send![super(self), respondsToSelector: selector] };
            own || self.ivars().original.respondsToSelector(selector)
        }
    }
    unsafe impl WKNavigationDelegate for FrameGuard {
        #[unsafe(method(webView:navigationAction:didBecomeDownload:))]
        fn action_download(&self,view:&WKWebView,action:&WKNavigationAction,download:&WKDownload){
            crate::download_control::register(&self.ivars().site,download);
            unsafe{self.ivars().original.webView_navigationAction_didBecomeDownload(view,action,download);}
        }
        #[unsafe(method(webView:navigationResponse:didBecomeDownload:))]
        fn response_download(&self,view:&WKWebView,response:&WKNavigationResponse,download:&WKDownload){
            crate::download_control::register(&self.ivars().site,download);
            unsafe{self.ivars().original.webView_navigationResponse_didBecomeDownload(view,response,download);}
        }

        #[unsafe(method(webView:decidePolicyForNavigationAction:decisionHandler:))]
        fn decide(
            &self,
            view: &WKWebView,
            action: &WKNavigationAction,
            handler: &block2::DynBlock<dyn Fn(WKNavigationActionPolicy)>,
        ) {
            unsafe {
                // A missing target frame means a new window; that stays with the popup policy.
                let embedded = action
                    .targetFrame()
                    .is_some_and(|frame| !frame.isMainFrame());
                if embedded && !action.shouldPerformDownload() {
                    let target = action
                        .request()
                        .URL()
                        .and_then(|url| url.absoluteString())
                        .and_then(|url| url::Url::parse(&url.to_string()).ok());
                    if target.as_ref().is_some_and(md_moodle::embeddable) {
                        handler.call((WKNavigationActionPolicy::Allow,));
                        return;
                    }
                    // The top-level policy only opens the browser for HTTPS, which is
                    // handled above, so a frame can never open the browser through it.
                }
                self.ivars()
                    .original
                    .webView_decidePolicyForNavigationAction_decisionHandler(view, action, handler);
            }
        }
        #[unsafe(method(webView:decidePolicyForNavigationResponse:decisionHandler:))]
        fn respond(
            &self,
            view: &WKWebView,
            response: &WKNavigationResponse,
            handler: &block2::DynBlock<dyn Fn(WKNavigationResponsePolicy)>,
        ) {
            unsafe {
                if response.isForMainFrame() && self.captures(response) {
                    handler.call((WKNavigationResponsePolicy::Download,));
                    return;
                }
                self.ivars()
                    .original
                    .webView_decidePolicyForNavigationResponse_decisionHandler(
                        view, response, handler,
                    );
            }
        }
    }
    impl FrameGuard {
        #[unsafe(method(forwardingTargetForSelector:))]
        fn forward(&self, _selector: Sel) -> *mut AnyObject {
            Retained::as_ptr(&self.ivars().original).cast_mut().cast()
        }
    }
);
impl FrameGuard {
    /// Whether this top-level response goes to managed downloads instead of the page.
    fn captures(&self, response: &WKNavigationResponse) -> bool {
        let ivars = self.ivars();
        let requested = ivars
            .save_next
            .take()
            .is_some_and(|at| at.elapsed() < crate::OPEN_PENDING);
        let raw = unsafe { response.response() };
        let Some(target) = raw
            .URL()
            .and_then(|url| url.absoluteString())
            .and_then(|url| url::Url::parse(&url.to_string()).ok())
        else {
            return false;
        };
        let mime = raw.MIMEType().map(|m| m.to_string()).unwrap_or_default();
        let attachment = raw
            .downcast_ref::<NSHTTPURLResponse>()
            .and_then(|http| {
                http.valueForHTTPHeaderField(&NSString::from_str("Content-Disposition"))
            })
            .is_some_and(|value| md_moodle::is_attachment(&value.to_string()));
        if requested {
            // Save to Downloads re-requests a Moodle file; a page is never saved as a file.
            let page = matches!(
                mime.to_ascii_lowercase().as_str(),
                "text/html" | "application/xhtml+xml"
            );
            if !page && md_moodle::saveable_file(&ivars.base, &target) {
                return true;
            }
            let _ = ivars.app.emit_to(
                "shell",
                "shell://error",
                "This page is not a Moodle file. Use the file’s own download link.",
            );
        }
        let state = ivars.app.state::<crate::AppState>();
        let settings = state
            .store
            .lock()
            .ok()
            .and_then(|store| store.settings().ok())
            .unwrap_or_default();
        let saved = |handling| handling != md_store::FileHandling::Inline;
        if !md_moodle::response_downloads(
            &ivars.base,
            &target,
            &mime,
            saved(settings.pdf),
            saved(settings.images),
        ) {
            return false;
        }
        let handling = match md_moodle::inline_kind(&mime) {
            Some(md_moodle::InlineKind::Pdf) => settings.pdf,
            Some(md_moodle::InlineKind::Image) => settings.images,
            None => md_store::FileHandling::Inline,
        };
        // Only a transfer this preference started may open afterwards; a download link
        // the user clicked is saved and nothing more.
        if !attachment && handling == md_store::FileHandling::Open {
            if let Ok(mut pending) = state.open_next.lock() {
                pending.insert(ivars.site.clone(), Instant::now());
            }
        }
        true
    }
}
thread_local! { static GUARDS: RefCell<HashMap<String, Retained<FrameGuard>>> = RefCell::new(HashMap::new()); }

pub fn install(
    app: &AppHandle,
    view: &tauri::Webview,
    site: &str,
    base: url::Url,
) -> tauri::Result<()> {
    let label = view.label().to_owned();
    let app = app.clone();
    let site = site.to_owned();
    view.with_webview(move |native| {
        GUARDS.with(|guards| {
            // The native handle is valid for this main-thread closure. Retain the proxy
            // because WKWebView's delegate property itself is weak; Wry retains its own.
            unsafe {
                let webview: &WKWebView = &*native.inner().cast();
                let Some(original) = webview.navigationDelegate() else {
                    return;
                };
                // Decide from the live view, not the label: site labels are reused.
                let current = Retained::as_ptr(&original).cast::<AnyObject>();
                if guards
                    .borrow()
                    .values()
                    .any(|guard| std::ptr::eq(Retained::as_ptr(guard).cast::<AnyObject>(), current))
                {
                    return;
                }
                let guard = MainThreadMarker::new()
                    .expect("main thread")
                    .alloc::<FrameGuard>()
                    .set_ivars(Ivars {
                        original,
                        app,
                        site,
                        base,
                        save_next: Cell::new(None),
                    });
                let guard: Retained<FrameGuard> = msg_send![super(guard), init];
                webview.setNavigationDelegate(Some(ProtocolObject::from_ref(&*guard)));
                guards.borrow_mut().insert(label, guard);
            }
        });
    })
}

/// Requests the shown Moodle file again and saves that response. The caller has
/// already checked the address; the page stays where it is.
pub fn save_current(view: &tauri::Webview, target: url::Url) -> Result<(), String> {
    let label = view.label().to_owned();
    let handle = view.clone();
    view.with_webview(move |_| {
        let ready = GUARDS.with(|guards| {
            guards
                .borrow()
                .get(&label)
                .map(|guard| guard.ivars().save_next.set(Some(Instant::now())))
                .is_some()
        });
        if !ready || handle.navigate(target).is_err() {
            let _ = handle.emit_to("shell", "shell://error", "Could not save this file.");
        }
    })
    .map_err(|_| "Could not save this file.".into())
}

pub fn remove(app: &tauri::AppHandle, label: &str) {
    let label = label.to_owned();
    let _ = app.run_on_main_thread(move || {
        GUARDS.with(|guards| {
            guards.borrow_mut().remove(&label);
        });
    });
}
