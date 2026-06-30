pub mod capture_x11;

pub use capture_x11::{
    capture_fullscreen, capture_region, capture_window, list_windows, CaptureResult, WindowInfo,
};