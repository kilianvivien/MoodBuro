//! macOS PDFKit's Save control uses a WebKit-specific save-data delegate callback,
//! not WKDownload. Forward all other UI delegate methods to Tauri/Wry unchanged.
//! This compatibility adapter requires regression checks across macOS releases.
use objc2::{
    define_class, msg_send,
    rc::Retained,
    runtime::{AnyObject, NSObject, Sel},
    DefinedClass, MainThreadOnly,
};
use objc2_foundation::{MainThreadMarker, NSData, NSObjectProtocol, NSString, NSURL};
use objc2_web_kit::{WKUIDelegate, WKWebView};
use std::{cell::RefCell, collections::HashMap};
use tauri::{AppHandle, Emitter, Manager};

struct Ivars {
    original: Retained<objc2::runtime::ProtocolObject<dyn WKUIDelegate>>,
    app: AppHandle,
    site: String,
    user_id: i64,
    label: String,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = Ivars]
    struct PdfSaveDelegate;
    unsafe impl NSObjectProtocol for PdfSaveDelegate {
        #[unsafe(method(respondsToSelector:))]
        fn responds(&self, selector: Sel) -> bool {
            let own: bool = unsafe { msg_send![super(self), respondsToSelector: selector] };
            own || self.ivars().original.respondsToSelector(selector)
        }
    }
    unsafe impl WKUIDelegate for PdfSaveDelegate {
        #[unsafe(method(webViewDidClose:))]
        fn did_close(&self, _view: &WKWebView) {
            let ivars = self.ivars();
            if !ivars.label.starts_with("popup-") { return; }
            let app = ivars.app.clone();
            let label = ivars.label.clone();
            let site = ivars.site.clone();
            tauri::async_runtime::spawn(async move {
                if let Some(window) = app.get_window(&label) { let _ = window.close(); }
                if crate::windows::switch(&app, &site).is_ok() {
                    let _ = app.emit_to("shell", "navigation://site-opened", &site);
                }
                crate::tray::show(&app);
            });
        }
    }
    impl PdfSaveDelegate {
        #[unsafe(method(forwardingTargetForSelector:))]
        fn forward(&self, _selector: Sel) -> *mut AnyObject {
            Retained::as_ptr(&self.ivars().original).cast_mut().cast()
        }
        #[unsafe(method(_webView:saveDataToFile:suggestedFilename:mimeType:originatingURL:))]
        fn save(&self, _view: &WKWebView, data: &NSData, filename: &NSString, mime: &NSString, origin: Option<&NSURL>) {
            let ivars = self.ivars();
            let Some(origin) = origin.and_then(|origin| origin.absoluteString()).and_then(|s| url::Url::parse(&s.to_string()).ok()) else { return; };
            if mime.to_string() != "application/pdf" || data.len() > 128 * 1024 * 1024 {
                let _ = ivars.app.emit_to("shell", "shell://error", "This PDF could not be saved. Use Moodle’s file download link.");
                return;
            }
            let bytes = data.to_vec();
            let Some(filename) = md_os::downloads::pdf_save_name(&mime.to_string(), &bytes, &filename.to_string()) else { return; };
            let app = ivars.app.clone();
            let site = ivars.site.clone();
            let user_id = ivars.user_id;
            tauri::async_runtime::spawn_blocking(move || {
                use std::io::Write;
                let mut destination = std::path::PathBuf::from(filename);
                if !crate::downloads::handle_for_user(&app, &site, tauri::webview::DownloadEvent::Requested { url: origin.clone(), destination: &mut destination }, Some(user_id)) { return; }
                let result = (|| -> std::io::Result<()> {
                    let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&destination)?;
                    file.write_all(&bytes)?;
                    file.sync_all()?;
                    // Preserve the normal downloaded-file trust boundary on this direct-data path.
                    let quarantine = format!("0083;{:x};MoodBuro;", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs());
                    let status = std::process::Command::new("/usr/bin/xattr").args(["-w", "com.apple.quarantine", &quarantine]).arg(&destination).status()?;
                    if !status.success() { return Err(std::io::Error::other("Could not quarantine PDF")); }
                    Ok(())
                })();
                crate::downloads::handle(&app, &site, tauri::webview::DownloadEvent::Finished { url: origin, path: Some(destination), success: result.is_ok() });
            });
        }
    }
);
thread_local! { static DELEGATES: RefCell<HashMap<String, Retained<PdfSaveDelegate>>> = RefCell::new(HashMap::new()); }

pub fn install(
    app: &AppHandle,
    view: &tauri::Webview,
    site: &str,
    user_id: i64,
) -> tauri::Result<()> {
    let app = app.clone();
    let site = site.to_owned();
    let label = view.label().to_owned();
    view.with_webview(move |native| {
        DELEGATES.with(|delegates| {
            if delegates.borrow().contains_key(&label) {
                return;
            }
            // The native handle is valid for this main-thread closure. Retain both
            // delegates because WKWebView's delegate property itself is weak.
            unsafe {
                let webview: &WKWebView = &*native.inner().cast();
                if let Some(original) = webview.UIDelegate() {
                    let delegate = MainThreadMarker::new()
                        .expect("main thread")
                        .alloc::<PdfSaveDelegate>()
                        .set_ivars(Ivars {
                            original,
                            app,
                            site,
                            user_id,
                            label: label.clone(),
                        });
                    let delegate: Retained<PdfSaveDelegate> = msg_send![super(delegate), init];
                    webview
                        .setUIDelegate(Some(objc2::runtime::ProtocolObject::from_ref(&*delegate)));
                    delegates.borrow_mut().insert(label, delegate);
                }
            }
        });
    })
}
pub fn remove(app: &AppHandle, label: &str) {
    let label = label.to_owned();
    let _ = app.run_on_main_thread(move || {
        DELEGATES.with(|delegates| {
            delegates.borrow_mut().remove(&label);
        });
    });
}
