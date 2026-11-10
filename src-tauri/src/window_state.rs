//! Remember only the main window, independently of accounts and child webviews.
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use tauri::{AppHandle, LogicalSize, Manager, PhysicalPosition, Window};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct Geometry {
    width: f64,
    height: f64,
    x: i32,
    y: i32,
    maximized: bool,
}
struct State {
    geometry: Mutex<Option<Geometry>>,
    restoring: AtomicBool,
}
const FILE: &str = "main-window.json";

impl Geometry {
    fn valid(&self) -> bool {
        self.width.is_finite()
            && self.height.is_finite()
            && (760. ..=20000.).contains(&self.width)
            && (540. ..=20000.).contains(&self.height)
    }
    fn fit(self, origin: (i32, i32), screen: (u32, u32), scale: f64) -> Self {
        let width = self.width.min(f64::from(screen.0) / scale).max(760.);
        let height = self.height.min(f64::from(screen.1) / scale).max(540.);
        let right = origin
            .0
            .saturating_add((f64::from(screen.0) - width * scale).max(0.) as i32);
        let bottom = origin
            .1
            .saturating_add((f64::from(screen.1) - height * scale).max(0.) as i32);
        Self {
            width,
            height,
            x: self.x.clamp(origin.0, right),
            y: self.y.clamp(origin.1, bottom),
            ..self
        }
    }
}
pub fn install(app: &AppHandle) {
    let saved = app
        .path()
        .app_config_dir()
        .ok()
        .and_then(|p| std::fs::read(p.join(FILE)).ok())
        .and_then(|b| serde_json::from_slice::<Geometry>(&b).ok())
        .filter(Geometry::valid);
    app.manage(State {
        geometry: Mutex::new(saved),
        restoring: AtomicBool::new(true),
    });
}
pub fn restore(window: &Window) {
    restore_geometry(window);
    window
        .state::<State>()
        .restoring
        .store(false, Ordering::Release);
    update(window);
}
fn restore_geometry(window: &Window) {
    let saved = window
        .state::<State>()
        .geometry
        .lock()
        .ok()
        .and_then(|g| *g);
    let Some(saved) = saved else { return };
    let monitors = window.available_monitors().unwrap_or_default();
    let monitor = monitors
        .iter()
        .find(|m| {
            let p = m.position();
            let size = m.size();
            i64::from(saved.x) >= i64::from(p.x)
                && i64::from(saved.x) < i64::from(p.x) + i64::from(size.width)
                && i64::from(saved.y) >= i64::from(p.y)
                && i64::from(saved.y) < i64::from(p.y) + i64::from(size.height)
        })
        .cloned()
        .or_else(|| window.primary_monitor().ok().flatten());
    let Some(monitor) = monitor else { return };
    let area = monitor.work_area();
    let p = &area.position;
    let size = &area.size;
    let geometry = saved.fit(
        (p.x, p.y),
        (size.width, size.height),
        monitor.scale_factor(),
    );
    let _ = window.set_size(LogicalSize::new(geometry.width, geometry.height));
    let _ = window.set_position(PhysicalPosition::new(geometry.x, geometry.y));
    if geometry.maximized {
        let _ = window.maximize();
    }
}
pub fn update(window: &Window) {
    if window.state::<State>().restoring.load(Ordering::Acquire) {
        return;
    }
    // Restore emits events too. Never hold the state lock while resizing the window.
    if window.is_minimized().unwrap_or(true) || window.is_fullscreen().unwrap_or(true) {
        return;
    }
    let maximized = window.is_maximized().unwrap_or(false);
    let Ok(mut state) = window.state::<State>().inner().geometry.lock() else {
        return;
    };
    if maximized {
        if let Some(geometry) = state.as_mut() {
            geometry.maximized = true;
        }
        return;
    }
    if let (Ok(size), Ok(position), Ok(scale)) = (
        window.inner_size(),
        window.outer_position(),
        window.scale_factor(),
    ) {
        let size = size.to_logical::<f64>(scale);
        let geometry = Geometry {
            width: size.width,
            height: size.height,
            x: position.x,
            y: position.y,
            maximized,
        };
        if geometry.valid() {
            *state = Some(geometry);
        }
    }
}
pub fn save(app: &AppHandle) {
    if let Some(window) = app.get_window("main") {
        update(&window);
    }
    let saved = app.state::<State>().geometry.lock().ok().and_then(|g| *g);
    if let (Some(saved), Ok(dir)) = (saved, app.path().app_config_dir()) {
        if let Ok(bytes) = serde_json::to_vec(&saved) {
            if std::fs::create_dir_all(&dir).is_ok() {
                let temporary = dir.join("main-window.json.tmp");
                if std::fs::write(&temporary, bytes).is_ok() {
                    let _ = std::fs::rename(temporary, dir.join(FILE));
                }
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn restore_fits_a_removed_or_smaller_screen() {
        let saved = Geometry {
            width: 1800.,
            height: 1100.,
            x: 5000,
            y: -300,
            maximized: false,
        };
        let fitted = saved.fit((0, 0), (2560, 1600), 2.);
        assert_eq!((fitted.width, fitted.height), (1280., 800.));
        assert_eq!((fitted.x, fitted.y), (0, 0));
    }
    #[test]
    fn valid_geometry_retains_normal_bounds_and_rejects_invalid_dimensions() {
        let saved = Geometry {
            width: 1000.,
            height: 700.,
            x: -1800,
            y: 50,
            maximized: true,
        };
        let fitted = saved.fit((-1920, 0), (1920, 1080), 1.);
        assert_eq!(
            (
                fitted.width,
                fitted.height,
                fitted.x,
                fitted.y,
                fitted.maximized
            ),
            (1000., 700., -1800, 50, true)
        );
        assert!(saved.valid());
        assert!(!Geometry {
            width: f64::NAN,
            ..saved
        }
        .valid());
        assert!(!Geometry {
            width: 10.,
            ..saved
        }
        .valid());
    }
}
