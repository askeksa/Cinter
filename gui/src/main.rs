mod app;
mod iff;

use eframe::{NativeOptions, run_native};
use eframe::egui::Vec2;

fn main() {
	let options = NativeOptions {
		initial_window_size: Some(Vec2::new(674.0, 630.0)),
		drag_and_drop_support: true,
		.. Default::default()
	};
	run_native(crate::app::TITLE, options,
		Box::new(|cc| {
			Box::new(crate::app::CinterApp::new(cc))
		})
	);
}
