use std::fs::File;
use std::io::prelude::*;
use std::ops::RangeInclusive;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::{Arc, RwLock};

use eframe::egui;
use egui::{Event, Key};
use rand::{thread_rng, Rng};

use cpal::traits::{DeviceTrait, HostTrait, EventLoopTrait};

use cinter::engine::{CinterEngine, CinterInstrument, PARAMETER_COUNT};

use crate::iff::{IffReader, IffWriter};

pub const TITLE: &'static str = "Cinter 4.1 by Blueberry";

pub struct CinterApp {
	player: SyncSender<PlayerMessage>,
	cursors: Vec<Arc<AtomicUsize>>,

	params: CinterParameters,
	auto_length: bool,

	engine: Arc<CinterEngine>,
	current_instrument: CinterInstrument,
	octaves: Octaves,
	volume: f32,

	error_string: Option<String>,
}

pub struct CinterParameters {
	values: [f32; PARAMETER_COUNT],
	length: usize,
	repeat_length: usize,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Octaves { Low, High }

enum FileFormat { Raw, Iff }

impl FileFormat {
	fn extension(&self) -> &'static str {
		match self {
			FileFormat::Raw => ".raw",
			FileFormat::Iff => ".8svx",
		}
	}
}

struct PlayerState {
	instrument: Option<Arc<RwLock<CinterInstrument>>>,
	notes: Vec<(cinter::Note, Option<Arc<AtomicUsize>>)>,
	target_volume: f32,
	current_volume: f32,
}

enum PlayerMessage {
	Instrument { instrument: CinterInstrument },
	NoteOn { key: u8, cursor: Arc<AtomicUsize> },
	NoteOff { key: u8 },
	SetVolume { volume: f32 },
}

fn translate_key(key: Key) -> Option<u8> {
	use Key::*;
	match match key {
		Z => 0, S => 1, X => 2, D => 3, C => 4, V => 5,
		G => 6, B => 7, H => 8, N => 9, J => 10, M => 11, L => 13,
		Q => 12, Num2 => 13, W => 14, Num3 => 15, E => 16, R => 17,
		Num5 => 18, T => 19, Num6 => 20, Y => 21, Num7 => 22, U => 23,
		I => 24, Num9 => 25, O => 26, Num0 => 27, P => 28,
		_ => 255,
	} {
		255 => None,
		k => Some(k),
	}
}

impl CinterApp {
	pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
		if cc.integration_info.prefer_dark_mode.is_none() {
			cc.egui_ctx.set_visuals(egui::Visuals::dark());
		}

		let player = Self::start_player();
		let engine = Arc::new(CinterEngine::new());
		let params = [
			0.05, 0.40, 0.53, 0.50, 0.65, 0.50, 0.20, 0.40, 0.0, 0.0, 0.1, 0.2
		];

		let mut current_instrument = CinterInstrument::new(Arc::clone(&engine), &params, None, None);
		player.send(PlayerMessage::Instrument { instrument: current_instrument.clone() }).ok();
		let length = Self::compute_length(&mut current_instrument);

		Self {
			player,
			cursors: vec![],

			params: CinterParameters {
				values: params,
				length,
				repeat_length: 0,
			},
			auto_length: true,

			engine,
			current_instrument,
			octaves: Octaves::High,
			volume: 1.0,

			error_string: None,
		}
	}

	fn new_cursor(&mut self) -> Arc<AtomicUsize> {
		let cursor = Arc::new(0.into());
		self.cursors.push(Arc::clone(&cursor));
		cursor
	}

	fn repeat_start(&self) -> Option<usize> {
		if self.params.repeat_length > 0 && self.params.repeat_length <= self.params.length {
			Some(self.params.length - self.params.repeat_length)
		} else {
			None
		}
	}

	fn start_player() -> SyncSender<PlayerMessage> {
		let (sender, receiver) = sync_channel(3);

		std::thread::spawn(move || {
			let mut state = PlayerState {
				instrument: None,
				notes: vec![],
				target_volume: 1.0,
				current_volume: 1.0,
			};

			let host = cpal::default_host();
			let device = host.default_output_device().expect("No output device available");
			let format = device.default_output_format().expect("No default output format");
			let event_loop = host.event_loop();
			let stream = event_loop.build_output_stream(&device, &format).expect("Failed to create stream");
			let sample_rate = format.sample_rate.0 as f32;
			event_loop.play_stream(stream).expect("Failed to play stream");
			event_loop.run(move |_stream_id, stream_result| {
				let data = stream_result.expect("Error in stream");
				if let cpal::StreamData::Output { buffer: cpal::UnknownTypeOutputBuffer::F32(mut buffer) } = data {
					for msg in receiver.try_iter() {
						match msg {
							PlayerMessage::Instrument { instrument } => {
								match &mut state.instrument {
									Some(irc) => *irc.write().unwrap() = instrument,
									None => state.instrument = Some(Arc::new(RwLock::new(instrument))),
								}
							},
							PlayerMessage::NoteOn { key, cursor } => {
								if !state.notes.iter().any(|(note, _)| note.key == key) {
									if let Some(irc) = &mut state.instrument {
										let note = cinter::Note::new(Arc::clone(irc), key, 127, sample_rate);
										state.notes.push((note, Some(cursor)));
									}
								}
							},
							PlayerMessage::NoteOff { key } => {
								for (note, _) in &mut state.notes {
									if note.key == key {
										note.release(127);
									}
								}
							},
							PlayerMessage::SetVolume { volume } => {
								state.target_volume = volume;
							},
						}
					}

					for i in 0..buffer.len() {
						buffer[i] = 0.0;
					}
					let instrument = &mut state.instrument;
					let mut volume = state.current_volume;
					let target_volume = state.target_volume;
					state.notes.retain_mut(|(note, cursor)| {
						for i in (0..buffer.len()).step_by(2) {
							let value = note.produce_sample() * volume;
							buffer[i + 0] += value;
							buffer[i + 1] += value;
							if volume != target_volume {
								if volume < target_volume {
									volume = target_volume.min(volume + 0.01);
								} else {
									volume = target_volume.max(volume - 0.01);
								}
							}
						}
						if let Some(index) = instrument.as_ref().unwrap().read().unwrap().repeated_index(note.current_index()) {
							if let Some(cursor) = cursor { cursor.store(index, Ordering::Relaxed); }
						} else {
							*cursor = None;
						}
						note.is_alive()
					});
					state.current_volume = volume;
				}
			});
		});

		sender
	}

	fn save_sample(&mut self, format: FileFormat) -> std::io::Result<()> {
		let filename = CinterEngine::get_sample_filename(&self.params.values);
		let mut file = File::create(filename.clone() + format.extension())?;
		let data: Vec<u8> = (0..self.params.length).map(|i| self.current_instrument.get_sample(i) as u8).collect();
		match format {
			FileFormat::Raw => file.write_all(&data),
			FileFormat::Iff => {
				let mut w = IffWriter::new();
				w.write_chunk("FORM", |w| {
					w.write_bytes("8SVX");
					w.write_chunk("VHDR", |w| {
						w.write_u32((self.params.length - self.params.repeat_length) as u32); // oneShotHiSamples
						w.write_u32(self.params.repeat_length as u32); // repeatHiSamples
						w.write_u32(32); // samplesPerHiCycle
						w.write_u16(16726); // samplesPerSec
						w.write_u8(1); // ctOctave
						w.write_u8(0); // sCompression
						w.write_u32(0x10000); // volume
					});
					w.write_chunk("NAME", |w| {
						w.write_string_padded(filename.as_str());
					});
					w.write_chunk("ANNO", |w| {
						w.write_string_padded(TITLE);
					});
					w.write_chunk("BODY", |w| {
						w.write_bytes(data);
					});
				});
				file.write_all(w.get_data())
			},
		}
	}

	fn load_sample(&mut self, filename: &str) -> anyhow::Result<CinterParameters> {
		let mut data = vec![];
		File::open(filename)?.read_to_end(&mut data)?;
		if let Ok([b'8', b'S', b'V', b'X', chunks @ ..]) = IffReader::find_chunk(&data, "FORM") {
			// 8SVX file
			let header = IffReader::find_chunk(chunks, "VHDR")?;
			let once_length = u32::from_be_bytes(header[0..4].try_into()?) as usize;
			let repeat_length = u32::from_be_bytes(header[4..8].try_into()?) as usize;
			let length = once_length + repeat_length;
			let name = match IffReader::find_chunk(chunks, "NAME") {
				Ok(name) => std::str::from_utf8(name)?,
				_ => filename,
			};
			let param_values = CinterEngine::parameters_from_sample_filename(name)?;
			Ok(CinterParameters {
				values: param_values,
				length,
				repeat_length,
			})
		} else {
			// RAW file
			let param_values = CinterEngine::parameters_from_sample_filename(filename)?;
			Ok(CinterParameters {
				values: param_values,
				length: data.len(),
				repeat_length: 0,
			})
		}
	}

	fn set_random_parameters(&mut self) {
		let mut random = thread_rng();
		for p in 0..PARAMETER_COUNT {
			self.params.values[p] = random.gen::<f32>()
		}
		self.params.repeat_length = 0;
	}

	fn compute_length(instrument: &mut CinterInstrument) -> usize {
		let mut length = 65534usize;
		while length > 2 && instrument.get_sample_raw(length - 1) == 0 {
			length -= 1;
		}
		(length + 1) & !1
	}
}

fn with_width(ui: &mut egui::Ui, width: f32, add_contents: impl FnOnce(&mut egui::Ui)) {
	ui.with_layout(egui::Layout::top_down_justified(egui::Align::Min), |ui| {
		ui.set_width(width);
		add_contents(ui);
	});
}

impl eframe::App for CinterApp {
	fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
		egui::CentralPanel::default().show(ctx, |ui| {

			let old_params = self.params.values;
			let old_length = self.params.length;
			let old_repeat_start = self.repeat_start();
			let old_auto_length = self.auto_length;

			ui.horizontal(|ui| {
				ui.heading("Parameters");
				if ui.button("Random").clicked() {
					self.set_random_parameters();
				}
				if ui.button("Random melodic").clicked() {
					self.set_random_parameters();
					self.params.values[3] = 0.5;
					self.params.values[5] = 0.5;
					self.params.values[7] *= 0.5;
				}
				ui.with_layout(egui::Layout::right_to_left(), |ui| {
					egui::widgets::global_dark_light_mode_buttons(ui);
				});
			});
			ui.separator();

			for p in 0..PARAMETER_COUNT {
				let param = &mut self.params.values[p];
				let resolution = CinterEngine::get_parameter_resolution(p as i32);
				ui.horizontal(|ui| {
					let (value, label) = CinterEngine::get_parameter_text_and_label(p as i32, *param);
					ui.spacing_mut().slider_width = 400.0;
					with_width(ui, 100.0, |ui| {
						ui.label(CinterEngine::get_parameter_name(p as i32));
					});
					ui.add(egui::Slider::new(param, 0.0..=1.0).show_value(false));
					if ui.small_button("➖").clicked() {
						*param = (((*param / resolution).round() - 1.0) * resolution).max(0.0);
					}
					if ui.small_button("➕").clicked() {
						*param = (((*param / resolution).round() + 1.0) * resolution).min(1.0);
					}
					ui.label(value + " " + &label);
				});
			}

			ui.separator();

			ui.horizontal(|ui| {
				if ui.button("Save as RAW").clicked() {
					match self.save_sample(FileFormat::Raw) {
						Ok(..) => self.error_string = None,
						Err(err) => self.error_string = Some(format!("{}", err)),
					}
				}
				if ui.button("Save as 8SVX").clicked() {
					match self.save_sample(FileFormat::Iff) {
						Ok(..) => self.error_string = None,
						Err(err) => self.error_string = Some(format!("{}", err)),
					}
				}
				ui.add(egui::Label::new(CinterEngine::get_sample_filename(&self.params.values)));
				if let Some(err) = &self.error_string {
					ui.add(egui::Label::new(egui::RichText::new(err).color(egui::Color32::RED)));
				}
			});

			ui.separator();

			let (plot_col, loop_col, cursor_col) = if ui.style().visuals.dark_mode {
				(egui::Color32::LIGHT_BLUE, egui::Color32::WHITE, egui::Color32::YELLOW)
			} else {
				(egui::Color32::DARK_BLUE, egui::Color32::BLACK, egui::Color32::BROWN)
			};

			let plot_size = egui::Vec2 { x: ui.available_width(), y: 220.0 };
			let (_response, painter) =
				ui.allocate_painter(plot_size, egui::Sense::drag());
			let rect = painter.clip_rect();

			let mut lines = vec![];
			let mut prev_pos = None;
			for i in 0..self.params.length {
				let sample = self.current_instrument.get_sample(i as usize);
				let x = i as f32 * plot_size.x / self.params.length as f32;
				let y = (130 - sample as i32) as f32 * plot_size.y / 260.0;
				let pos = egui::Pos2 { x: rect.min.x + x, y: rect.min.y + y };
				if let Some(prev_pos) = prev_pos {
					lines.push(egui::Shape::LineSegment {
						points: [prev_pos, pos],
						stroke: egui::Stroke { width: 0.8, color: plot_col },
					});
				}
				prev_pos = Some(pos);
			}
			let length = self.params.length;
			let mut vline = |index: usize, color: egui::Color32| {
				let x = index as f32 * plot_size.x / length as f32;
				let points = [
					egui::Pos2 { x: rect.min.x + x, y: rect.min.y },
					egui::Pos2 { x: rect.min.x + x, y: rect.max.y },
				];
				let line = egui::Shape::LineSegment {
					points,
					stroke: egui::Stroke { width: 0.8, color },
				};
				lines.push(line);
			};
			self.cursors.retain(|cursor| {
				if Arc::strong_count(cursor) == 1 {
					false
				} else {
					vline(cursor.load(Ordering::Relaxed), cursor_col);
					true
				}
			});
			if let Some(repeat_start) = self.repeat_start() {
				vline(repeat_start, loop_col);
			}
			painter.add(egui::Shape::Vec(lines));

			ui.separator();

			fn make_adjuster(value: &mut i32, range: RangeInclusive<impl egui::emath::Numeric>) -> egui::widgets::DragValue<'_> {
				egui::widgets::DragValue::from_get_set(move |v: Option<f64>| {
					if let Some(v) = v {
						*value = ((v as i32) + 16) & -32;
					}
					*value as f64
				}).clamp_range(range.start().to_f64() ..= range.end().to_f64())
				  .speed(10.0)
				  .min_decimals(0)
				  .max_decimals(0)
			}

			ui.horizontal(|ui| {
				ui.group(|ui| {
					let mut length = self.params.length as i32;
					ui.add(egui::Label::new(egui::RichText::new("Length: ").text_style(egui::TextStyle::Button)));
					ui.add_enabled_ui(!self.auto_length, |ui| {
						ui.add(make_adjuster(&mut length, 0 ..= 65534));
						if ui.button("➖").clicked() {
							length = (length - 2).max(0);
						}
						if ui.button("➕").clicked() {
							length = (length + 2).min(65534);
						}
					});
					ui.checkbox(&mut self.auto_length, "Auto");
					self.params.length = length as usize;
				});

				ui.group(|ui| {
					let mut repeat_length = self.params.repeat_length as i32;
					ui.add(egui::Label::new(egui::RichText::new("Repeat: ").text_style(egui::TextStyle::Button)));
					ui.add(make_adjuster(&mut repeat_length, 0 ..= self.params.length));
					if ui.button("➖").clicked() {
						repeat_length = (repeat_length - 2).max(0);
					}
					if ui.button("➕").clicked() {
						repeat_length = (repeat_length + 2).min(self.params.length as i32);
					}
					self.params.repeat_length = repeat_length as usize;
				});

				ui.group(|ui| {
					ui.add(egui::Label::new(egui::RichText::new("Octaves: ").text_style(egui::TextStyle::Button)));
					ui.selectable_value(&mut self.octaves, Octaves::Low, "Low");
					ui.selectable_value(&mut self.octaves, Octaves::High, "High");
				});

				ui.group(|ui| {
					ui.add(egui::Label::new(egui::RichText::new("Volume: ").text_style(egui::TextStyle::Button)));
					let volume = self.volume;
					ui.add(egui::widgets::DragValue::new(&mut self.volume).speed(0.01).clamp_range(0.0 ..= 5.0));
					if self.volume != volume {
						self.player.send(PlayerMessage::SetVolume { volume: self.volume }).ok();
					}
				});
			});

			ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
				egui::warn_if_debug_build(ui);
			});

			for file in &ctx.input().raw.dropped_files {
				if let Some(name) = file.path.as_ref().and_then(|f| f.file_name()).and_then(|n| n.to_str()) {
					match self.load_sample(name) {
						Ok(params) => {
							self.error_string = None;
							self.params = params;
							self.current_instrument = CinterInstrument::new(
								self.engine.clone(), &self.params.values, None, None
							);
							self.auto_length = self.params.length == Self::compute_length(&mut self.current_instrument);
						},
						Err(err) => {
							self.error_string = Some(format!("{}", err));
						},
					}
				}
			}

			if self.params.length != old_length ||
					self.repeat_start() != old_repeat_start ||
					self.params.values != old_params ||
					self.auto_length != old_auto_length {
				self.error_string = None;
				self.current_instrument = CinterInstrument::new(
					self.engine.clone(), &self.params.values, Some(self.params.length), self.repeat_start()
				);
				if self.auto_length {
					self.params.length = Self::compute_length(&mut self.current_instrument);
					self.current_instrument.length = self.params.length;
				}
				self.player.send(PlayerMessage::Instrument { instrument: self.current_instrument.clone() }).ok();
			}

			for event in &ui.input().events {
				if let Event::Key { key, pressed, .. } = event {
					if let Some(mut key) = translate_key(*key) {
						key += match self.octaves {
							Octaves::Low => 12,
							Octaves::High => 24,
						};
						if key >= 12 && key < 48 {
							if *pressed {
								let cursor = self.new_cursor();
								self.player.send(PlayerMessage::NoteOn { key, cursor }).ok();
							} else {
								self.player.send(PlayerMessage::NoteOff { key }).ok();
							}
						}
					}
				}
			}
		});

		if !self.cursors.is_empty() {
			ctx.request_repaint();
		}
	}
}
