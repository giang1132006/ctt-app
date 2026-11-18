#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;

use app::CttApp;
use std::cell::RefCell;
use std::rc::Rc;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let app_window = AppWindow::new()?;
    let state = Rc::new(RefCell::new(CttApp::new()));

    // Synchronize initial UI properties with domain state
    {
        let s = state.borrow();
        app_window.set_counter(s.counter());
        app_window.set_status_text(s.status().into());
        app_window.set_search_query(s.search_query().into());
    }

    // Handle counter increment callback
    let app_weak = app_window.as_weak();
    let state_clone = state.clone();
    app_window.on_increment_counter(move || {
        if let Some(window) = app_weak.upgrade() {
            let mut s = state_clone.borrow_mut();
            let new_count = s.increment();
            window.set_counter(new_count);
        }
    });

    // Handle quit callback
    let app_weak = app_window.as_weak();
    app_window.on_quit_requested(move || {
        if let Some(window) = app_weak.upgrade() {
            let _ = window.hide();
        }
    });

    app_window.run()
}
