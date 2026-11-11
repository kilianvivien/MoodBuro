//! Native “Open as timetable” item in a Moodle view's link context menu.
//!
//! Public API only (ADR 0003): `-[NSView willOpenMenu:withEvent:]` is added
//! at run time to the web view's class (Wry's `WryWebView` does not implement
//! it) and calls WebKit's own implementation first, so Open Link, Download
//! Linked File and Copy Link stay intact. No WebKit delegate is replaced.
//!
//! The clicked link is recorded by a `contextmenu` listener in a dedicated
//! `WKContentWorld`, invisible to the page's own scripts, and read back by
//! native code. There is no JS→native message channel: a page cannot call in.
//! The item stays hidden until Rust has validated the link for this site.
//! Only views of a supported (INSP) site are registered.
use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject, Imp, NSObject, Sel};
use objc2::{define_class, msg_send, sel, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{NSEvent, NSMenu, NSMenuItem};
use objc2_foundation::{NSError, NSString};
use objc2_web_kit::{WKContentWorld, WKUserScript, WKUserScriptInjectionTime, WKWebView};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use tauri::AppHandle;

const WORLD: &str = "moodburo-timetable";
const LISTENER: &str = r#"(() => {
  if (window.__moodburoTimetableReady) return;
  window.__moodburoTimetableReady = true;
  addEventListener('contextmenu', (event) => {
    const link = event.target && event.target.closest ? event.target.closest('a[href]') : null;
    window.__moodburoTimetableTarget = link ? {
      href: String(link.href).slice(0, 4096),
      label: (link.textContent || '').trim().replace(/\s+/g, ' ').slice(0, 300),
      at: Date.now()
    } : null;
  }, true);
})();"#;
const READ: &str = "JSON.stringify(window.__moodburoTimetableTarget || null)";

struct Entry {
    app: AppHandle,
    site_id: String,
    user_id: i64,
    base: url::Url,
    label: String,
}

struct Pending {
    view: usize,
    generation: isize,
    href: url::Url,
    label: String,
    page: Option<String>,
}

thread_local! {
    /// Registered views, by `WKWebView` pointer.
    static VIEWS: RefCell<HashMap<usize, Entry>> = RefCell::new(HashMap::new());
    /// Classes the method was added to, with the class whose implementation runs first.
    static CLASSES: RefCell<HashMap<usize, &'static AnyClass>> = RefCell::new(HashMap::new());
    static PENDING: RefCell<Option<Pending>> = const { RefCell::new(None) };
    static GENERATION: Cell<isize> = const { Cell::new(0) };
    static TARGET: RefCell<Option<Retained<MenuTarget>>> = const { RefCell::new(None) };
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "MoodBuroTimetableMenuTarget"]
    struct MenuTarget;
    impl MenuTarget {
        #[unsafe(method(openTimetable:))]
        fn open(&self, sender: &NSMenuItem) {
            chosen(sender.tag());
        }
    }
);

fn ns(text: &str) -> Retained<NSString> {
    NSString::from_str(text)
}

fn chosen(generation: isize) {
    let Some(pending) = PENDING.with(|p| p.borrow_mut().take()) else {
        return;
    };
    if pending.generation != generation {
        return;
    }
    let intent = VIEWS.with(|views| {
        views.borrow().get(&pending.view).map(|entry| {
            (
                entry.app.clone(),
                crate::timetable::Intent {
                    site_id: entry.site_id.clone(),
                    user_id: entry.user_id,
                    view_label: entry.label.clone(),
                    page: pending.page.clone(),
                    href: pending.href.clone(),
                    label: pending.label.clone(),
                    created: std::time::Instant::now(),
                },
            )
        })
    });
    if let Some((app, intent)) = intent {
        crate::timetable::link_chosen(&app, intent);
    }
}

/// The class whose `willOpenMenu:withEvent:` must run before ours.
fn superclass_for(this: &AnyObject) -> Option<&'static AnyClass> {
    let mut class = Some(this.class());
    CLASSES.with(|classes| {
        let classes = classes.borrow();
        while let Some(current) = class {
            if let Some(superclass) = classes.get(&(current as *const AnyClass as usize)) {
                return Some(*superclass);
            }
            class = current.superclass();
        }
        None
    })
}

#[derive(serde::Deserialize)]
struct Target {
    href: String,
    label: String,
    at: f64,
}

extern "C-unwind" fn will_open_menu(this: &AnyObject, _cmd: Sel, menu: &NSMenu, event: &NSEvent) {
    if let Some(superclass) = superclass_for(this) {
        unsafe {
            let _: () = msg_send![super(this, superclass), willOpenMenu: menu, withEvent: event];
        }
    }
    let view = this as *const AnyObject as usize;
    let Some((base, french)) = VIEWS.with(|views| {
        views
            .borrow()
            .get(&view)
            .map(|e| (e.base.clone(), crate::menus::french(&e.app)))
    }) else {
        return;
    };
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let generation = GENERATION.with(|g| {
        g.set(g.get().wrapping_add(1));
        g.get()
    });
    PENDING.with(|p| *p.borrow_mut() = None);
    let title = if french {
        "Ouvrir comme emploi du temps"
    } else {
        "Open as timetable"
    };
    let item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            mtm.alloc(),
            &ns(title),
            Some(sel!(openTimetable:)),
            &ns(""),
        )
    };
    TARGET.with(|target| unsafe {
        item.setTarget(target.borrow().as_deref().map(|t| t as &AnyObject))
    });
    item.setTag(generation);
    item.setHidden(true);
    menu.insertItem_atIndex(&item, 0);
    let webview: &WKWebView = unsafe { &*(this as *const AnyObject).cast::<WKWebView>() };
    let page = unsafe { webview.URL() }
        .and_then(|url| url.absoluteString())
        .map(|s| s.to_string());
    let world = unsafe { WKContentWorld::worldWithName(&ns(WORLD), mtm) };
    let reply = RcBlock::new(move |result: *mut AnyObject, error: *mut NSError| {
        if !error.is_null() || result.is_null() {
            return;
        }
        let Some(text) = unsafe { &*result }
            .downcast_ref::<NSString>()
            .map(|s| s.to_string())
        else {
            return;
        };
        let Ok(Some(target)) = serde_json::from_str::<Option<Target>>(&text) else {
            return;
        };
        let fresh = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|now| (now.as_millis() as f64 - target.at).abs() < 5_000.0)
            .unwrap_or(false);
        let Ok(href) = url::Url::parse(&target.href) else {
            return;
        };
        #[cfg(debug_assertions)]
        eprintln!(
            "timetable menu: link {}",
            crate::timetable::link_shape(&base, &href)
        );
        if !fresh
            || crate::timetable::link_target(&base, &href).is_none()
            || !md_inspcal::looks_like_timetable(&target.label)
        {
            return;
        }
        if GENERATION.with(|g| g.get()) != generation {
            return;
        }
        PENDING.with(|p| {
            *p.borrow_mut() = Some(Pending {
                view,
                generation,
                href,
                label: md_moodle::plain_text(&target.label, 300),
                page: page.clone(),
            })
        });
        item.setHidden(false);
    });
    unsafe {
        webview.evaluateJavaScript_inFrame_inContentWorld_completionHandler(
            &ns(READ),
            None,
            &world,
            Some(&reply),
        );
    }
}

fn add_method(class: &'static AnyClass) -> bool {
    let key = class as *const AnyClass as usize;
    if CLASSES.with(|classes| classes.borrow().contains_key(&key)) {
        return true;
    }
    let Some(superclass) = class.superclass() else {
        return false;
    };
    // SAFETY: the function matches `v@:@@` (void; self, _cmd, NSMenu*, NSEvent*).
    let imp: Imp = unsafe {
        std::mem::transmute::<extern "C-unwind" fn(&AnyObject, Sel, &NSMenu, &NSEvent), Imp>(
            will_open_menu,
        )
    };
    let added = unsafe {
        objc2::ffi::class_addMethod(
            class as *const AnyClass as *mut AnyClass,
            sel!(willOpenMenu:withEvent:),
            imp,
            c"v@:@@".as_ptr(),
        )
        .as_bool()
    };
    if added {
        CLASSES.with(|classes| classes.borrow_mut().insert(key, superclass));
    } else {
        // The class implements it already (a future Wry?): leave it alone.
        #[cfg(debug_assertions)]
        eprintln!("timetable menu: web view class already handles willOpenMenu; item unavailable");
    }
    added
}

/// Register a supported site's Moodle view. Idempotent.
pub fn install(
    app: &AppHandle,
    view: &tauri::Webview,
    site_id: &str,
    user_id: i64,
    base: url::Url,
) -> tauri::Result<()> {
    let app = app.clone();
    let site_id = site_id.to_owned();
    let label = view.label().to_owned();
    view.with_webview(move |native| {
        let Some(mtm) = MainThreadMarker::new() else {
            return;
        };
        // SAFETY: the native handle is valid for this main-thread closure.
        let webview: &WKWebView = unsafe { &*native.inner().cast() };
        let key = webview as *const WKWebView as usize;
        let object: &AnyObject = webview.as_ref();
        if !add_method(object.class()) {
            return;
        }
        TARGET.with(|target| {
            if target.borrow().is_none() {
                *target.borrow_mut() = Some(unsafe { msg_send![MenuTarget::alloc(mtm), init] });
            }
        });
        let fresh = VIEWS.with(|views| {
            views
                .borrow_mut()
                .insert(
                    key,
                    Entry {
                        app,
                        site_id,
                        user_id,
                        base,
                        label,
                    },
                )
                .is_none()
        });
        if !fresh {
            return;
        }
        let world = unsafe { WKContentWorld::worldWithName(&ns(WORLD), mtm) };
        unsafe {
            let script = WKUserScript::initWithSource_injectionTime_forMainFrameOnly_inContentWorld(
                mtm.alloc(),
                &ns(LISTENER),
                WKUserScriptInjectionTime::AtDocumentStart,
                true,
                &world,
            );
            webview
                .configuration()
                .userContentController()
                .addUserScript(&script);
            // The page already loading missed the document-start script.
            webview.evaluateJavaScript_inFrame_inContentWorld_completionHandler(
                &ns(LISTENER),
                None,
                &world,
                None,
            );
        }
    })
}

/// Forget a view that is being closed or reassigned.
pub fn remove(app: &AppHandle, label: &str) {
    let label = label.to_owned();
    let _ = app.run_on_main_thread(move || {
        VIEWS.with(|views| views.borrow_mut().retain(|_, entry| entry.label != label));
        PENDING.with(|p| *p.borrow_mut() = None);
    });
}
