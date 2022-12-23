
pub mod engine;

use std::collections::VecDeque;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, RwLock};

use vst::api::{Events, Supported};
use vst::buffer::AudioBuffer;
use vst::event::{Event, MidiEvent};
use vst::plugin::{CanDo, Category, HostCallback, Info, Plugin, PluginParameters};
use vst::plugin_main;

use rfd::FileDialog;

use crate::engine::PARAMETER_COUNT;
use crate::engine::{CinterEngine, CinterInstrument};

#[allow(dead_code)]
pub enum MidiCommand {
	NoteOn      { channel: u8, key: u8, velocity: u8 },
	NoteOff     { channel: u8, key: u8, velocity: u8 },
	AllNotesOff { channel: u8,          velocity: u8 },
	AllSoundOff { channel: u8,          velocity: u8 },
	Unknown
}

impl MidiCommand {
	fn from_data(data: &[u8; 3]) -> MidiCommand {
		match data[0] & 0xF0 {
			0x80 => MidiCommand::NoteOff { channel: data[0] & 0x0F, key: data[1], velocity: data[2] },
			0x90 => MidiCommand::NoteOn  { channel: data[0] & 0x0F, key: data[1], velocity: data[2] },
			0xB0 => match data[1] {
				120 => MidiCommand::AllSoundOff { channel: data[0] & 0x0F, velocity: data[2] },
				123 => MidiCommand::AllNotesOff { channel: data[0] & 0x0F, velocity: data[2] },
				_   => MidiCommand::Unknown
			},
			_    => MidiCommand::Unknown
		}
	}
}

pub struct TimedMidiCommand {
	time: usize,
	command: MidiCommand,
}

pub struct Note {
	instrument: Arc<RwLock<CinterInstrument>>,

	time: usize,
	pub key: u8,
	freq: f32,

	release_time: Option<usize>
}

impl Note {
	pub fn new(instrument: Arc<RwLock<CinterInstrument>>, key: u8, _velocity: u8, sample_rate: f32) -> Note {
		Note {
			instrument: instrument.clone(),
			time: 0,
			key,
			freq: 440.0 * ((key + 27) as f32 / 12.0).exp2() / sample_rate,

			release_time: None
		}
	}

	pub fn release_amplitude(&self) -> f32 {
		match self.release_time {
			Some(release_time) => (1.0 - (self.time - release_time) as f32 * 0.001).max(0.0),
			None => 1.0
		}
	}

	pub fn produce_sample(&mut self) -> f32 {
		let phase = self.time as f32 * self.freq;
		let i = phase.floor() as usize;
		let t = phase - i as f32;
		let a0 = t*((2.0-t)*t-1.0);
		let a1 = t*t*(3.0*t-5.0)+2.0;
		let a2 = t*((4.0-3.0*t)*t+1.0);
		let a3 = t*t*(t-1.0);
		let mut instrument = self.instrument.write().unwrap();
		let d0 = instrument.get_sample(i) as f32;
		let d1 = instrument.get_sample(i + 1) as f32;
		let d2 = instrument.get_sample(i + 2) as f32;
		let d3 = instrument.get_sample(i + 3) as f32;
		let mut v = a0*d0 + a1*d1 + a2*d2 + a3*d3;
		v *= self.release_amplitude();
		self.time += 1;
		v / 254.0
	}

	pub fn current_index(&self) -> usize {
		let phase = self.time as f32 * self.freq;
		phase.floor() as usize
	}

	pub fn release(&mut self, _velocity: u8) {
		self.release_time = Some(self.time);
	}

	pub fn is_released(&self) -> bool {
		self.release_time.is_some()
	}

	pub fn is_alive(&self) -> bool {
		self.release_amplitude() > 0.0
	}
}


pub struct CinterPlugin {
	param_object: Arc<CinterParameterObject>,

	sample_rate: f32,
	time: usize,
	notes: Vec<Note>,
	events: VecDeque<TimedMidiCommand>,

	engine: Arc<CinterEngine>,
	instrument: Arc<RwLock<CinterInstrument>>,
}

pub struct CinterParameterObject {
	params: RwLock<Parameters>,
}

struct Parameters {
	values: [f32; PARAMETER_COUNT],
	changed: bool,
}

impl Default for CinterPlugin {
	fn default() -> Self {
		let params = Parameters {
			values: [
				0.05, 0.40, 0.53, 0.50, 0.65, 0.50, 0.20, 0.40, 0.0, 0.0, 0.1, 0.2
			],
			changed: false,
		};
		let engine = Arc::new(CinterEngine::new());
		let instrument = Arc::new(RwLock::new(CinterInstrument::new(
			engine.clone(), &params.values, &[], None, None
		)));

		CinterPlugin {
			param_object: Arc::new(CinterParameterObject {
				params: RwLock::new(params),
			}),

			sample_rate: 44100.0,
			time: 0,
			notes: Vec::new(),
			events: VecDeque::new(),

			engine,
			instrument,
		}
	}
}

impl Plugin for CinterPlugin {
	fn new(_host: HostCallback) -> CinterPlugin {
		CinterPlugin::default()
	}

	fn get_info(&self) -> Info {
		Info {
			presets: 1,
			parameters: PARAMETER_COUNT as i32,
			inputs: 0,
			outputs: 2,
			category: Category::Synth,
			f64_precision: false,
			preset_chunks: true,

			name: "Cinter".to_string(),
			vendor: "Loonies".to_string(),
			unique_id: 0xC1D7EA,
			version: 4000,

			.. Info::default()
		}
	}

	fn can_do(&self, can_do: CanDo) -> Supported {
		match can_do {
			CanDo::ReceiveMidiEvent => Supported::Yes,
			_                       => Supported::No
		}
	}

	fn process_events(&mut self, events: &Events) {
		for e in events.events() {
			match e {
				Event::Midi(MidiEvent { delta_frames, ref data, .. }) => {
					self.events.push_back(TimedMidiCommand {
						time: self.time + (delta_frames as usize),
						command: MidiCommand::from_data(data)
					});
				}
				_ => {}
			}
		}
	}

	fn process(&mut self, buffer: &mut AudioBuffer<f32>) {
		let mut outputs = buffer.split().1;
		for i in 0..outputs[0].len() {
			while !self.events.is_empty() && self.events.front().unwrap().time == self.time {
				let event = self.events.pop_front().unwrap();
				self.handle_event(event);
			}
			let sample = self.produce_sample();
			outputs[0][i] = sample;
			outputs[1][i] = sample;
			self.time += 1;
		}
	}

	fn set_sample_rate(&mut self, rate: f32) {
		self.sample_rate = rate;
	}

	fn get_parameter_object(&mut self) -> Arc<dyn PluginParameters> {
		Arc::clone(&mut self.param_object) as Arc<dyn PluginParameters>
	}
}

impl PluginParameters for CinterParameterObject {
	fn get_parameter_name(&self, index: i32) -> String {
		CinterEngine::get_parameter_name(index)
	}

	fn get_parameter_text(&self, index: i32) -> String {
		let params = self.params.read().unwrap();
		CinterEngine::get_parameter_text_and_label(index, params.values[index as usize]).0
	}

	fn get_parameter_label(&self, index: i32) -> String {
		let params = self.params.read().unwrap();
		CinterEngine::get_parameter_text_and_label(index, params.values[index as usize]).1
	}

	fn get_parameter(&self, index: i32) -> f32 {
		let params = self.params.read().unwrap();
		params.values[index as usize]
	}

	fn set_parameter(&self, index: i32, value: f32) {
		let mut params = self.params.write().unwrap();
		params.values[index as usize] = value;
		params.changed = true;
	}

	fn get_preset_name(&self, _preset: i32) -> String {
		"Boing".to_string()
	}

	fn get_preset_data(&self) -> Vec<u8> {
		let params = self.params.read().unwrap();
		let mut data = vec![];
		for p in &params.values {
			data.extend_from_slice(&p.to_bits().to_le_bytes());
		}
		data
	}

	fn get_bank_data(&self) -> Vec<u8> {
		self.get_preset_data()
	}

	fn load_preset_data(&self, data: &[u8]) {
		let mut params = self.params.write().unwrap();
		for (i, chunk) in data.chunks_exact(4).enumerate() {
			let mut bytes = [0u8; 4];
			bytes.copy_from_slice(chunk);
			params.values[i] = f32::from_bits(u32::from_le_bytes(bytes));
		}
		params.changed = true;
	}

	fn load_bank_data(&self, data: &[u8]) {
		self.load_preset_data(data);
	}
}

impl CinterPlugin {
	fn handle_event(&mut self, event: TimedMidiCommand) {
		let mut write_filename = None;

		match event.command {
			MidiCommand::NoteOn { key, velocity, .. } => {
				let mut params = self.param_object.params.write().unwrap();
				if params.changed {
					self.instrument = Arc::new(RwLock::new(CinterInstrument::new(
						self.engine.clone(), &params.values, &[], None, None
					)));
					params.changed = false;
				}
				self.notes.push(Note::new(self.instrument.clone(), key, velocity, self.sample_rate));

				if key == 52 {
					write_filename = Some(CinterEngine::sample_filename_from_parameters(&params.values));
				}
			},
			MidiCommand::NoteOff { key, velocity, .. } => {
				for note in &mut self.notes {
					if note.key == key && !note.is_released() {
						note.release(velocity);
						break;
					}
				}
			},
			MidiCommand::AllNotesOff { velocity, .. } => {
				for note in &mut self.notes {
					if !note.is_released() {
						note.release(velocity);
					}
				}
			},
			MidiCommand::AllSoundOff { .. } => {
				self.notes.clear();
			},
			MidiCommand::Unknown => {}
		}

		if let Some(filename) = write_filename {
			if let Some(path) = FileDialog::new().pick_folder() {
				let full_path = Path::new(&path).join(filename + ".raw");
				if let Ok(mut file) = File::create(&full_path) {
					let mut instrument = self.instrument.write().unwrap();
					let data: Vec<u8> = (0..65534).map(|i| {
						instrument.get_sample(i) as u8
					}).collect();
					let mut len = data.len();
					while len > 2 && data[len - 2 .. len] == [0, 0] {
						len -= 2;
					}
					match file.write_all(&data[0 .. len]) {
						Ok(_) => println!("Cinter: Wrote sample to file: {:?}", full_path),
						Err(_) => println!("Cinter: Could not write to file: {:?}", full_path),
					}
				} else {
					println!("Cinter: Could not open file: {:?}", full_path);
				}
			}
		}
	}

	fn produce_sample(&mut self) -> f32 {
		let mut sample = 0f32;
		for i in (0..self.notes.len()).rev() {
			if self.notes[i].is_alive() {
				sample += self.notes[i].produce_sample();
			} else {
				self.notes.remove(i);
			}
		}
		sample
	}
}

plugin_main!(CinterPlugin);
