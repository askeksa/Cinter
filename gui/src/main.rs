#![windows_subsystem = "windows"]

mod app;
mod iff;

use eframe::{NativeOptions, run_native};
use eframe::egui::{Vec2, ViewportBuilder};

fn main() -> eframe::Result<()> {
	let viewport = ViewportBuilder {
		inner_size: Some(Vec2::new(700.0, 600.0)),
		drag_and_drop: Some(true),
		.. Default::default()
	};
	let options = NativeOptions {
		viewport: viewport,
		.. Default::default()
	};
	run_native(crate::app::TITLE, options,
		Box::new(|cc| {
			Ok(Box::new(crate::app::CinterApp::new(cc)))
		})
	)
}
