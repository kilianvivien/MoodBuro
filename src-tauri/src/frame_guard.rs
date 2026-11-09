//! Wry calls Tauri's `on_navigation` for every frame, iframes included, and gives it
//! only the URL. Moodle pages embed cross-origin content (Genially, video players,
//! H5P…), which the top-level policy would cancel and open in the browser on every
//! page load. This proxy decides embedded-frame navigations itself and forwards
//! everything else, including all other delegate methods, to Tauri/Wry unchanged.
//! This compatibility adapter requires regression checks across macOS releases.
use objc2::{
    define_class, msg_send,
    rc::Retained,
    runtime::{AnyObject, NSObject, ProtocolObject, Sel},
    DefinedClass, MainThreadOnly,
};
use objc2_foundation::{MainThreadMarker, NSObjectProtocol};
use objc2_web_kit::{
    WKNavigationAction, WKNavigationActionPolicy, WKNavigationDelegate, WKWebView,
};
use std::{cell::RefCell, collections::HashMap};

struct Ivars {
    original: Retained<ProtocolObject<dyn WKNavigationDelegate>>,
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
    }
    impl FrameGuard {
        #[unsafe(method(forwardingTargetForSelector:))]
        fn forward(&self, _selector: Sel) -> *mut AnyObject {
            Retained::as_ptr(&self.ivars().original).cast_mut().cast()
        }
    }
);
thread_local! { static GUARDS: RefCell<HashMap<String, Retained<FrameGuard>>> = RefCell::new(HashMap::new()); }

pub fn install(view: &tauri::Webview) -> tauri::Result<()> {
    let label = view.label().to_owned();
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
                    .set_ivars(Ivars { original });
                let guard: Retained<FrameGuard> = msg_send![super(guard), init];
                webview.setNavigationDelegate(Some(ProtocolObject::from_ref(&*guard)));
                guards.borrow_mut().insert(label, guard);
            }
        });
    })
}

pub fn remove(app: &tauri::AppHandle, label: &str) {
    let label = label.to_owned();
    let _ = app.run_on_main_thread(move || {
        GUARDS.with(|guards| {
            guards.borrow_mut().remove(&label);
        });
    });
}
