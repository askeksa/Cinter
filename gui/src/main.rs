mod app;

use eframe::{NativeOptions, run_native};
use eframe::egui::Vec2;

fn main() {
	let app = crate::app::CinterApp::init();
	let options = NativeOptions {
		initial_window_size: Some(Vec2::new(700.0, 600.0)),
		drag_and_drop_support: true,
		.. Default::default()
	};
	run_native(Box::new(app), options);
}
