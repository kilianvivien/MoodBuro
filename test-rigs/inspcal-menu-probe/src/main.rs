//! Headless probe: can a native item be added to WKWebView's *link* context
//! menu through public API only (NSView willOpenMenu:withEvent:, added to a
//! WKWebView subclass at run time as MoodBuro would do to Wry's class), and
//! can the clicked link be read from an isolated content world, without any
//! JS->native message handler?
use std::cell::{Cell, RefCell};
use std::time::Instant;

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::Message;
use objc2::runtime::{AnyClass, AnyObject, Imp, NSObject, Sel};
use objc2::{define_class, msg_send, sel, ClassType, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::*;
use objc2_foundation::*;
use objc2_web_kit::*;

define_class!(
    #[unsafe(super(WKWebView, NSView, NSResponder, NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "ProbeWebView"]
    struct ProbeWebView;
);

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "ProbeTarget"]
    struct ProbeTarget;
    impl ProbeTarget {
        #[unsafe(method(openTimetable:))]
        fn open(&self, _sender: &AnyObject) { println!("item selected"); }
    }
);

thread_local! {
    static VIEW: RefCell<Option<Retained<ProbeWebView>>> = const { RefCell::new(None) };
    static WORLD: RefCell<Option<Retained<WKContentWorld>>> = const { RefCell::new(None) };
    static TARGET: RefCell<Option<Retained<ProbeTarget>>> = const { RefCell::new(None) };
    static OPEN: Cell<bool> = const { Cell::new(false) };
    static OPENED_AT: Cell<Option<Instant>> = const { Cell::new(None) };
    static MENUS: Cell<u32> = const { Cell::new(0) };
}

fn ns(s: &str) -> Retained<NSString> { NSString::from_str(s) }

extern "C-unwind" fn will_open_menu(this: &AnyObject, _cmd: Sel, menu: &NSMenu, event: &NSEvent) {
    unsafe { let _: () = msg_send![super(this, WKWebView::class()), willOpenMenu: menu, withEvent: event]; }
    let mtm = MainThreadMarker::new().unwrap();
    OPEN.set(true);
    OPENED_AT.set(Some(Instant::now()));
    MENUS.set(MENUS.get() + 1);
    println!("willOpenMenu #{} with {} items:", MENUS.get(), menu.numberOfItems());
    for item in menu.itemArray().iter() {
        let id: Option<Retained<NSString>> = unsafe { msg_send![&*item, identifier] };
        println!("   - {:?} [{}]", item.title().to_string(), id.map(|i| i.to_string()).unwrap_or_default());
    }
    let item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(mtm.alloc(), &ns("Ouvrir comme emploi du temps"), Some(sel!(openTimetable:)), &ns(""))
    };
    TARGET.with(|t| unsafe { item.setTarget(t.borrow().as_deref().map(|t| t as &AnyObject)) });
    item.setHidden(true);
    menu.insertItem_atIndex(&item, 0);
    let view = VIEW.with(|v| v.borrow().clone().unwrap());
    let world = WORLD.with(|w| w.borrow().clone().unwrap());
    let block = RcBlock::new(move |result: *mut AnyObject, error: *mut NSError| {
        let elapsed = OPENED_AT.get().map(|t| t.elapsed().as_millis()).unwrap_or(0);
        let text = if result.is_null() { None } else {
            let object = unsafe { &*result };
            object.downcast_ref::<NSString>().map(|s| s.to_string())
        };
        let open = OPEN.get();
        println!("   target reply after {elapsed} ms (menu still open: {open}) error={} -> {:?}", !error.is_null(), text);
        if let Some(text) = text { if text != "null" { item.setHidden(false); println!("   item unhidden: {}", !item.isHidden()); } }
    });
    unsafe {
        view.evaluateJavaScript_inFrame_inContentWorld_completionHandler(
            &ns("JSON.stringify(window.__moodburoTarget || null)"), None, &world, Some(&block));
    }
    let menu: Retained<NSMenu> = menu.retain();
    let cancel = RcBlock::new(move |_t: std::ptr::NonNull<NSTimer>| { let first = menu.itemAtIndex(0).unwrap(); if !first.isHidden() { println!("   selecting our item"); menu.performActionForItemAtIndex(0); } menu.cancelTracking(); });
    let timer = unsafe { NSTimer::timerWithTimeInterval_repeats_block(0.6, false, &cancel) };
    unsafe { NSRunLoop::mainRunLoop().addTimer_forMode(&timer, NSRunLoopCommonModes) };
}

extern "C-unwind" fn did_close_menu(this: &AnyObject, _cmd: Sel, menu: &NSMenu, event: Option<&NSEvent>) {
    unsafe { let _: () = msg_send![super(this, WKWebView::class()), didCloseMenu: menu, withEvent: event]; }
    OPEN.set(false);
    println!("didCloseMenu");
}

fn add_method(class: &AnyClass, name: Sel, imp: Imp, types: &std::ffi::CStr) -> bool {
    unsafe { objc2::ffi::class_addMethod(class as *const AnyClass as *mut AnyClass, name, imp, types.as_ptr()).as_bool() }
}

fn right_click(window: &NSWindow, x: f64, y: f64) {
    let mtm = MainThreadMarker::new().unwrap();
    let _ = mtm;
    let now = NSProcessInfo::processInfo().systemUptime();
    for kind in [NSEventType::RightMouseDown, NSEventType::RightMouseUp] {
        let event = NSEvent::mouseEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_clickCount_pressure(
            kind, NSPoint::new(x, y), NSEventModifierFlags::empty(), now, window.windowNumber(), None, 0, 1, 1.0).unwrap();
        window.sendEvent(&event);
    }
}

fn after(seconds: f64, f: impl Fn() + 'static) {
    let block = RcBlock::new(move |_t: std::ptr::NonNull<NSTimer>| f());
    let timer = unsafe { NSTimer::timerWithTimeInterval_repeats_block(seconds, false, &block) };
    unsafe { NSRunLoop::mainRunLoop().addTimer_forMode(&timer, NSRunLoopCommonModes) };
}

fn main() {
    let mtm = MainThreadMarker::new().unwrap();
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);

    let class = ProbeWebView::class();
    let types = c"v@:@@";
    assert!(add_method(class, sel!(willOpenMenu:withEvent:), unsafe { std::mem::transmute::<extern "C-unwind" fn(&AnyObject, Sel, &NSMenu, &NSEvent), Imp>(will_open_menu) }, types));
    assert!(add_method(class, sel!(didCloseMenu:withEvent:), unsafe { std::mem::transmute::<extern "C-unwind" fn(&AnyObject, Sel, &NSMenu, Option<&NSEvent>), Imp>(did_close_menu) }, types));

    let config = unsafe { WKWebViewConfiguration::new(mtm) };
    let world = unsafe { WKContentWorld::worldWithName(&ns("moodburo-link-target"), mtm) };
    let source = "addEventListener('contextmenu', e => { const a = e.target && e.target.closest ? e.target.closest('a[href]') : null; window.__moodburoTarget = a ? { href: a.href, label: (a.textContent || '').trim().slice(0, 200) } : null; }, true);";
    let script = unsafe { WKUserScript::initWithSource_injectionTime_forMainFrameOnly_inContentWorld(mtm.alloc(), &ns(source), WKUserScriptInjectionTime::AtDocumentStart, false, &world) };
    unsafe { config.userContentController().addUserScript(&script) };
    WORLD.with(|w| *w.borrow_mut() = Some(world));
    TARGET.with(|t| *t.borrow_mut() = Some(unsafe { msg_send![ProbeTarget::alloc(mtm), init] }));

    let frame = NSRect::new(NSPoint::new(200.0, 200.0), NSSize::new(600.0, 400.0));
    let view: Retained<ProbeWebView> = unsafe { msg_send![ProbeWebView::alloc(mtm), initWithFrame: frame, configuration: &*config] };
    // Page script tries to spoof the target in the *page* world; it must not leak into ours.
    let html = r#"<!doctype html><body style="margin:0"><script>window.__moodburoTarget={href:'https://evil.example/x.pdf'};</script>
<a href="/pluginfile.php/42/mod_resource/content/1/Edt%20du%2009.11.2026%20au%2013.11.2026.pdf" style="display:block;height:200px;font:30px sans-serif;background:#eef">Edt du 09 novembre au 13 novembre 26</a>
<p style="height:200px;margin:0">No link here</p></body>"#;
    unsafe { view.loadHTMLString_baseURL(&ns(html), Some(&NSURL::URLWithString(&ns("https://moodle.example/course/view.php?id=7")).unwrap())) };

    let window = unsafe { NSWindow::initWithContentRect_styleMask_backing_defer(mtm.alloc(), frame, NSWindowStyleMask::Titled, NSBackingStoreType::Buffered, false) };
    unsafe { window.setReleasedWhenClosed(false) };
    window.setContentView(Some(&view));
    window.makeKeyAndOrderFront(None);
    #[allow(deprecated)]
    app.activateIgnoringOtherApps(true);
    VIEW.with(|v| *v.borrow_mut() = Some(view));

    let w1 = window.clone();
    after(2.0, move || { println!("right-click on the link"); right_click(&w1, 100.0, 300.0); });
    let w2 = window.clone();
    after(4.0, move || { println!("right-click off the link"); right_click(&w2, 100.0, 100.0); });
    after(6.5, move || { println!("menus opened: {}", MENUS.get()); std::process::exit(0); });
    app.run();
}
