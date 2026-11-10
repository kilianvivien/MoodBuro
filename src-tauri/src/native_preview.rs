//! Rust-owned AppKit reader and toolbar. No document bytes or paths cross IPC.
use objc2::{
    define_class, msg_send,
    rc::Retained,
    runtime::{NSObject, ProtocolObject},
    sel, ClassType, DefinedClass, MainThreadOnly,
};
use objc2_app_kit::{
    NSAlert, NSAutoresizingMaskOptions, NSImage, NSScrollView, NSTextField, NSToolbar,
    NSToolbarDelegate, NSToolbarDisplayMode, NSToolbarItem, NSToolbarItemIdentifier,
    NSToolbarItemValidation, NSView, NSWindow, NSWorkspace,
};
use objc2_foundation::{MainThreadMarker, NSArray, NSObjectProtocol, NSSize, NSString, NSURL};
use objc2_pdf_kit::{PDFDisplayMode, PDFDocument, PDFView};
use objc2_quick_look_ui::{QLPreviewView, QLPreviewViewStyle};
use std::{
    cell::RefCell,
    collections::HashMap,
    path::{Path, PathBuf},
};

pub type ResolvePath = Box<dyn Fn() -> Result<PathBuf, String>>;
struct Ivars {
    root: Retained<NSView>,
    zoom: Option<Retained<NSScrollView>>,
    window: Retained<NSWindow>,
    path: ResolvePath,
    position: Retained<NSTextField>,
    fr: bool,
}
// Both delegates and their views are main-thread-only. NSToolbar's delegate is weak.
thread_local! { static READERS: RefCell<HashMap<String, Retained<ReaderToolbar>>> = RefCell::new(HashMap::new()); }

fn pdf_in(root: &NSView, depth: usize) -> Option<Retained<PDFView>> {
    if root.isKindOfClass(PDFView::class()) {
        // Class checked above; retain a PDFView owned by this view hierarchy.
        return Some(unsafe {
            Retained::retain((root as *const NSView).cast::<PDFView>().cast_mut())?
        });
    }
    if depth >= 20 {
        return None;
    }
    root.subviews()
        .iter()
        .find_map(|child| pdf_in(&child, depth + 1))
}

const ITEMS: [&str; 10] = [
    "previous",
    "next",
    "position",
    "zoom-out",
    "zoom-in",
    "fit",
    "NSToolbarFlexibleSpaceItem",
    "open",
    "reveal",
    "fullscreen",
];
fn identifiers(paged: bool) -> Retained<NSArray<NSToolbarItemIdentifier>> {
    NSArray::from_retained_slice(
        &ITEMS
            .iter()
            .filter(|id| paged || !matches!(**id, "previous" | "next"))
            .map(|id| NSString::from_str(id))
            .collect::<Vec<_>>(),
    )
}
fn label(id: &str, fr: bool) -> (&'static str, &'static str) {
    match (id, fr) {
        ("position", false) => ("Page and zoom", "doc.text"),
        ("position", true) => ("Page et zoom", "doc.text"),
        ("previous", false) => ("Previous Page", "chevron.up"),
        ("previous", true) => ("Page précédente", "chevron.up"),
        ("next", false) => ("Next Page", "chevron.down"),
        ("next", true) => ("Page suivante", "chevron.down"),
        ("zoom-out", false) => ("Zoom Out", "minus.magnifyingglass"),
        ("zoom-out", true) => ("Réduire", "minus.magnifyingglass"),
        ("zoom-in", false) => ("Zoom In", "plus.magnifyingglass"),
        ("zoom-in", true) => ("Agrandir", "plus.magnifyingglass"),
        ("fit", false) => ("Fit to Window", "arrow.up.left.and.arrow.down.right"),
        ("fit", true) => ("Adapter à la fenêtre", "arrow.up.left.and.arrow.down.right"),
        ("open", false) => ("Open in Default App", "arrow.up.forward.app"),
        ("open", true) => (
            "Ouvrir dans l’application par défaut",
            "arrow.up.forward.app",
        ),
        ("reveal", false) => ("Show in Finder", "folder"),
        ("reveal", true) => ("Afficher dans le Finder", "folder"),
        ("fullscreen", false) => ("Full Screen", "arrow.up.left.and.arrow.down.right"),
        _ => ("Plein écran", "arrow.up.left.and.arrow.down.right"),
    }
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = Ivars]
    struct ReaderToolbar;
    unsafe impl NSObjectProtocol for ReaderToolbar {}
    unsafe impl NSToolbarDelegate for ReaderToolbar {
        #[unsafe(method_id(toolbarDefaultItemIdentifiers:))]
        fn defaults(&self, _toolbar: &NSToolbar) -> Retained<NSArray<NSToolbarItemIdentifier>> { identifiers(self.ivars().zoom.is_none()) }
        #[unsafe(method_id(toolbarAllowedItemIdentifiers:))]
        fn allowed(&self, _toolbar: &NSToolbar) -> Retained<NSArray<NSToolbarItemIdentifier>> { identifiers(self.ivars().zoom.is_none()) }
        #[unsafe(method_id(toolbar:itemForItemIdentifier:willBeInsertedIntoToolbar:))]
        fn item(&self, _toolbar: &NSToolbar, id: &NSToolbarItemIdentifier, _insert: bool) -> Option<Retained<NSToolbarItem>> {
            let name = id.to_string();
            let item = NSToolbarItem::initWithItemIdentifier(self.mtm().alloc(), id);
            let (title, symbol) = label(&name, self.ivars().fr);
            if name == "position" {
                item.setLabel(&NSString::from_str(if self.ivars().fr { "Page et zoom" } else { "Page and zoom" }));
                item.setView(Some(&self.ivars().position));
            }
            let title = NSString::from_str(title);
            item.setLabel(&title);
            item.setToolTip(Some(&title));
            item.setImage(NSImage::imageWithSystemSymbolName_accessibilityDescription(&NSString::from_str(symbol), Some(&title)).as_deref());
            unsafe { item.setTarget(Some(self)); item.setAction(Some(sel!(readerAction:))); }
            Some(item)
        }
    }
    unsafe impl NSToolbarItemValidation for ReaderToolbar {
        #[unsafe(method(validateToolbarItem:))]
        fn validate(&self, item: &NSToolbarItem) -> bool {
            self.update_position();
            let id = item.itemIdentifier().to_string();
            let pdf = pdf_in(&self.ivars().root,0);
            if let Some(pdf) = pdf.as_ref() {
                unsafe {
                    match id.as_str() {
                        "previous" => pdf.canGoToPreviousPage(), "next" => pdf.canGoToNextPage(),
                        "zoom-in" => pdf.canZoomIn(), "zoom-out" => pdf.canZoomOut(), _ => true,
                    }
                }
            } else {
                let enabled = match id.as_str() {
                    "previous" | "next" => false,
                    "zoom-in" => self.ivars().zoom.as_ref().is_some_and(|v| v.magnification() < v.maxMagnification()),
                    "zoom-out" => self.ivars().zoom.as_ref().is_some_and(|v| v.magnification() > v.minMagnification()),
                    _ => true,
                };
                if !enabled {
                    item.setToolTip(Some(&NSString::from_str(if self.ivars().fr {
                        "Utilisez les commandes du document ou ouvrez-le dans l’application par défaut."
                    } else { "Use the document’s own controls or open it in the default app." })));
                }
                enabled
            }
        }
    }
    impl ReaderToolbar {
        #[unsafe(method(readerAction:))]
        fn action(&self, sender: &NSToolbarItem) {
            let ivars = self.ivars();
            let name = sender.itemIdentifier().to_string();
            if let Some(pdf) = pdf_in(&ivars.root,0) {
                unsafe {
                    match name.as_str() {
                        "previous" => pdf.goToPreviousPage(None), "next" => pdf.goToNextPage(None),
                        "zoom-in" => pdf.zoomIn(None), "zoom-out" => pdf.zoomOut(None),
                        "fit" => pdf.setAutoScales(true), _ => {},
                    }
                }
            } else if let Some(zoom) = &ivars.zoom {
                let value = match name.as_str() {
                    "zoom-in" => zoom.magnification() * 1.2,
                    "zoom-out" => zoom.magnification() / 1.2,
                    "fit" => {
                        let viewport = zoom.contentSize();
                        let size = zoom.documentView().map(|view| view.frame().size).unwrap_or(viewport);
                        (viewport.width / size.width.max(1.)).min(viewport.height / size.height.max(1.))
                    }, _ => zoom.magnification(),
                };
                zoom.setMagnification(value.clamp(zoom.minMagnification(), zoom.maxMagnification()));
            }
            match name.as_str() {
                "open" | "reveal" => {
                    let result = (ivars.path)().and_then(|path| {
                        let path = path.to_str().ok_or("This download is unavailable.")?;
                        let url = NSURL::fileURLWithPath(&NSString::from_str(path));
                        let workspace = NSWorkspace::sharedWorkspace();
                        if name == "open" {
                            if !workspace.openURL(&url) { return Err("Could not open the downloaded file.".into()); }
                        } else { workspace.activateFileViewerSelectingURLs(&NSArray::from_retained_slice(&[url])); }
                        Ok(())
                    });
                    if result.is_err() {
                        let alert = NSAlert::new(self.mtm());
                        alert.setMessageText(&NSString::from_str(if ivars.fr { "Ce fichier n’est plus disponible" } else { "This file is unavailable" }));
                        alert.setInformativeText(&NSString::from_str(if ivars.fr { "Revenez aux téléchargements pour vérifier le fichier ou le télécharger à nouveau depuis Moodle." } else { "Return to Downloads to check the file or download it again from Moodle." }));
                        alert.runModal();
                    }
                },
                "fullscreen" => ivars.window.toggleFullScreen(None), _ => {},
            }
            self.update_position();
        }
    }
);

impl ReaderToolbar {
    fn update_position(&self) {
        let ivars = self.ivars();
        let text = if let Some(pdf) = pdf_in(&ivars.root, 0) {
            unsafe {
                match (pdf.document(), pdf.currentPage()) {
                    (Some(document), Some(page)) => format!(
                        "{} / {} · {:.0}%",
                        document.indexForPage(&page) + 1,
                        document.pageCount(),
                        pdf.scaleFactor() * 100.
                    ),
                    _ => String::new(),
                }
            }
        } else {
            ivars
                .zoom
                .as_ref()
                .map(|v| format!("{:.0}%", v.magnification() * 100.))
                .unwrap_or_default()
        };
        ivars.position.setStringValue(&NSString::from_str(&text));
    }
}

pub fn mount(
    window: &tauri::Window,
    path: &Path,
    fr: bool,
    resolve: ResolvePath,
) -> Result<(), String> {
    let mtm = MainThreadMarker::new().ok_or("Could not preview this document.")?;
    let ptr = window
        .ns_window()
        .map_err(|_| "Could not preview this document.")?;
    // Tauri owns this NSWindow; called only on AppKit's main thread.
    let native = unsafe { &*ptr.cast::<NSWindow>() };
    let content = native
        .contentView()
        .ok_or("Could not preview this document.")?;
    let url = NSURL::fileURLWithPath(&NSString::from_str(
        path.to_str().ok_or("This download is unavailable.")?,
    ));
    let (root, zoom): (Retained<NSView>, _) = if path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"))
    {
        let document = unsafe { PDFDocument::initWithURL(mtm.alloc(), &url) }
            .ok_or("Could not preview this document.")?;
        let view = unsafe { PDFView::initWithFrame(mtm.alloc(), content.bounds()) };
        unsafe {
            view.setDocument(Some(&document));
            view.setDisplayMode(PDFDisplayMode::SinglePageContinuous);
            view.setAutoScales(true);
        }
        (Retained::into_super(view), None)
    } else {
        let view = unsafe {
            QLPreviewView::initWithFrame_style(
                mtm.alloc(),
                content.bounds(),
                QLPreviewViewStyle::Normal,
            )
        }
        .ok_or("Could not preview this document.")?;
        unsafe {
            view.setAutostarts(false);
            view.setShouldCloseWithWindow(true);
            view.setPreviewItem(Some(ProtocolObject::from_ref(&*url)));
        }
        let scroll = NSScrollView::initWithFrame(mtm.alloc(), content.bounds());
        scroll.setHasVerticalScroller(true);
        scroll.setHasHorizontalScroller(true);
        scroll.setAllowsMagnification(true);
        scroll.setMinMagnification(0.5);
        scroll.setMaxMagnification(4.);
        scroll.setDocumentView(Some(&view));
        (Retained::into_super(scroll.clone()), Some(scroll))
    };
    root.setAutoresizingMask(
        NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable,
    );
    content.addSubview(&root);
    let position = NSTextField::labelWithString(&NSString::from_str("100%"), mtm);
    position.setFrameSize(NSSize::new(116., 20.));
    let delegate = mtm.alloc::<ReaderToolbar>().set_ivars(Ivars {
        root,
        zoom,
        window: unsafe { Retained::retain(ptr.cast::<NSWindow>()) }
            .ok_or("Could not preview this document.")?,
        path: resolve,
        position,
        fr,
    });
    let delegate: Retained<ReaderToolbar> = unsafe { msg_send![super(delegate), init] };
    let toolbar =
        NSToolbar::initWithIdentifier(mtm.alloc(), &NSString::from_str("MoodBuroDocumentReader"));
    toolbar.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
    toolbar.setDisplayMode(NSToolbarDisplayMode::IconOnly);
    toolbar.setAllowsUserCustomization(false);
    native.setToolbar(Some(&toolbar));
    delegate.update_position();
    READERS.with(|readers| {
        readers
            .borrow_mut()
            .insert(window.label().to_owned(), delegate);
    });
    Ok(())
}
pub fn remove(label: &str) {
    READERS.with(|readers| {
        readers.borrow_mut().remove(label);
    });
}

/// Desktop smoke harness: invokes the real toolbar actions through Objective-C.
#[cfg(debug_assertions)]
#[allow(dead_code)] // Called by the standalone desktop QA example, not the app.
pub fn verify_controls(label: &str) -> bool {
    READERS.with(|readers| {
        let readers = readers.borrow();
        let Some(reader) = readers.get(label) else {
            return false;
        };
        let invoke = |name: &str| {
            let item = reader
                .ivars()
                .window
                .toolbar()
                .unwrap()
                .items()
                .iter()
                .find(|item| item.itemIdentifier().to_string() == name)
                .unwrap();
            unsafe {
                let _: () = msg_send![&**reader, readerAction: &*item];
            }
        };
        if let Some(zoom) = &reader.ivars().zoom {
            let original = zoom.magnification();
            invoke("zoom-in");
            assert!(zoom.magnification() > original);
            assert_eq!(reader.ivars().position.stringValue().to_string(), "120%");
            invoke("zoom-out");
            invoke("fit");
            assert!(zoom.magnification() <= 1.);
            eprintln!("Reader toolbar: Office zoom, fit and readout passed.");
        } else if let Some(pdf) = pdf_in(&reader.ivars().root, 0) {
            unsafe {
                let original = pdf.scaleFactor();
                invoke("zoom-in");
                assert!(pdf.scaleFactor() > original);
                invoke("zoom-out");
                invoke("fit");
                if pdf.canGoToNextPage() {
                    let before = pdf.currentPage().unwrap();
                    invoke("next");
                    assert_ne!(before, pdf.currentPage().unwrap());
                    assert!(reader
                        .ivars()
                        .position
                        .stringValue()
                        .to_string()
                        .starts_with("2 /"));
                    invoke("previous");
                    assert_eq!(before, pdf.currentPage().unwrap());
                }
            }
            eprintln!("Reader toolbar: PDF paging, zoom, fit and readout passed.");
        } else {
            return false;
        }
        true
    })
}
