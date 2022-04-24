
use std::f32::consts::PI;
use std::sync::Arc;

pub const PARAMETER_COUNT: usize = 12;

pub struct CinterEngine {
	sine_table: Vec<i16>,
}

#[derive(Clone)]
pub struct CinterInstrument {
	engine: Arc<CinterEngine>,

	attack: i32,
	decay: i32,
	mpitch: u32,
	bpitch: u32,
	mod_: u32,
	mpitchdecay: u32,
	bpitchdecay: u32,
	moddecay: u32,
	mdist: i32,
	bdist: i32,
	vpower: i32,
	fdist: i32,

	pub length: usize,
	pub repeat_start: Option<usize>,

	phase: i32,
	amp: i32,
	amp_delta: i32,

	data: Vec<i8>,
}


impl CinterEngine {
	pub fn new() -> Self {
		CinterEngine {
			sine_table: (0..16384).map(|i| {
				((i as f32 / 16384.0 * (2.0 * PI)).sin() * 16384.0).round() as i16
			}).collect()
		}
	}

	pub fn get_parameter_name(index: i32) -> String {
		match index {
			0 => "attack",
			1 => "decay",
			2 => "mpitch",
			3 => "mpitchdecay",
			4 => "bpitch",
			5 => "bpitchdecay",
			6 => "mod",
			7 => "moddecay",
			8 => "mdist",
			9 => "bdist",
			10 => "vpower",
			11 => "fdist",
			_ => ""
		}.to_string()
	}

	pub fn get_parameter_text_and_label(index: i32, value: f32) -> (String, String) {
		let (text, label) = match index {
			// attack/decay envelope
			0 | 1 => match envfun(value) {
				0 => (format!("infinite"), ""),
				f => (format!("{}", 32767 / f + 1), "samples")
			},
			// pitch
			2 | 4 => match p100(value) {
				0 => (format!("none"), ""),
				v if v < 5 => (format!("{} oct", v - 5), ""),
				v if (v - 5) % 12 == 0 => (format!("{} oct", (v - 5) / 12), ""),
				v => (format!("{} oct {}", (v - 5) / 12, (v - 5) % 12), "st"),
			},
			// mod
			6 => (format!("{}", p100(value)), ""),
			// pitch/mod decay
			3 | 5 | 7 => (format!("{:.5}", decayfun(value) as f32 / 65536.0), ""),
			// dist
			8 | 9 | 11 => (format!("{}", p10(value)), ""),
			// vpower
			10 => (format!("{}", p10(value) + 1), ""),
			_ => (format!(""), "")
		};
		(text, label.to_string())
	}

	pub fn get_parameter_resolution(index: i32) -> f32 {
		if index < 8 { 0.01 } else { 0.1 }
	}

	pub fn get_sample_filename(params: &[f32; PARAMETER_COUNT]) -> String {
		let mut name = "1".to_string();
		for i in 0..PARAMETER_COUNT {
			let repr = if i < 8 {
				// Two digits
				let v = p100(params[i]);
				if v == 100 {
					"XX".to_string()
				} else {
					format!("{:02}", v)
				}
			} else {
				// One digit
				let v = p10(params[i]);
				if v == 10 {
					"X".to_string()
				} else {
					format!("{:01}", v)
				}
			};
			name += &repr;
		}
		name
	}

	pub fn parameters_from_sample_filename(name: &str) -> anyhow::Result<[f32; PARAMETER_COUNT]> {
		let legacy = !name.starts_with('1');
		let mut params = [0f32; PARAMETER_COUNT];
		for i in 0..PARAMETER_COUNT {
			if i < 8 {
				let digits: &str = name.get(i * 2 + 1 .. i * 2 + 3).ok_or(anyhow::anyhow!("Name too short"))?;
				params[i] = if digits.eq_ignore_ascii_case("XX") {
					100
				} else {
					digits.parse::<i32>()?
				} as f32 * 0.01;
			} else {
				let digit = name.get(i + 9 .. i + 10).ok_or(anyhow::anyhow!("Name too short"))?;
				params[i] = if digit.eq_ignore_ascii_case("X") {
					10
				} else {
					digit.parse::<i32>()?
				} as f32 * 0.1;
			};
		}

		if legacy {
			fn convert3to4(value: f32, fun3: &mut impl FnMut(f32) -> u32,  fun4: &mut impl FnMut(f32) -> u32) -> f32 {
				let target = fun3(value);
				let ((low, lowval), (high, highval)) = binary_search::binary_search((0, fun4(0.0)), (100, fun4(1.0)), |v| {
					let val = fun4(v as f32 * 0.01);
					if val < target {
						binary_search::Direction::Low(val)
					} else {
						binary_search::Direction::High(val)
					}
				});
				let v = if target - lowval < highval - target {
					low
				} else {
					high
				};
				v as f32 * 0.01
			}

			params[2] = convert3to4(params[2], &mut pitchfun3, &mut pitchfun);
			params[3] = convert3to4(params[3], &mut decayfun3, &mut decayfun);
			params[4] = convert3to4(params[4], &mut pitchfun3, &mut pitchfun);
			params[5] = convert3to4(params[5], &mut decayfun3, &mut decayfun);
			params[7] = convert3to4(params[7], &mut decayfun3, &mut decayfun);
		}

		Ok(params)
	}
}

impl CinterInstrument {
	pub fn new(
		engine: Arc<CinterEngine>,
		params: &[f32; PARAMETER_COUNT],
		length: Option<usize>,
		repeat_start: Option<usize>,
	) -> Self {
		let length = length.unwrap_or(65534);
		let repeat_start = match repeat_start {
			Some(start) if start >= length => None,
			_ => repeat_start
		};

		let mut inst = CinterInstrument {
			engine,

			attack:      envfun(params[0]),
			decay:       envfun(params[1]),
			mpitch:      pitchfun(params[2]) << 16,
			mpitchdecay: decayfun(params[3]),
			bpitch:      pitchfun(params[4]) << 16,
			bpitchdecay: decayfun(params[5]),
			mod_:        (p100(params[6]) << 16) as u32,
			moddecay:    decayfun(params[7]),
			mdist:       p10(params[8]),
			bdist:       p10(params[9]),
			vpower:      p10(params[10]),
			fdist:       p10(params[11]),

			length,
			repeat_start,

			phase:       0,
			amp:         0,
			amp_delta:   0,

			data:        Vec::with_capacity(length),
		};

		inst.data.push(0);
		inst.data.push(0);
		inst.amp_delta = inst.attack;

		inst
	}

	pub fn repeated_index(&self, index: usize) -> Option<usize> {
		if index < self.length {
			Some(index)
		} else {
			self.repeat_start.map(|repeat_start| {
				repeat_start + (index - self.length) % (self.length - repeat_start)
			})
		}
	}

	pub fn get_sample(&mut self, index: usize) -> i8 {
		if let Some(index) = self.repeated_index(index) {
			self.get_sample_raw(index)
		} else {
			0
		}
	}

	pub fn get_sample_raw(&mut self, index: usize) -> i8 {
		while self.data.len() <= index {
			let sample = self.compute_sample();
			self.data.push(sample);
		}
		self.data[index]
	}

	fn compute_sample(&mut self) -> i8 {
		let mval = self.distort(self.sintab(mul(self.phase, self.mpitch)), self.mdist);
		let mut val = self.distort(self.sintab(mul(self.phase, self.bpitch) + mul(mval, self.mod_)), self.bdist);
		let mut p = self.vpower;
		while p >= 0 {
			val = val * self.amp / 32768;
			p -= 1;
		}
		val = (self.distort(val, self.fdist) >> 7).min(127);

		self.mpitch = ((self.mpitch as u64 * self.mpitchdecay as u64) >> 16) as u32;
		self.bpitch = ((self.bpitch as u64 * self.bpitchdecay as u64) >> 16) as u32;
		self.mod_ = ((self.mod_ as u64 * self.moddecay as u64) >> 16) as u32;

		self.amp += self.amp_delta;
		if self.amp > 32767 {
			self.amp = 32767;
			self.amp_delta = -self.decay;
		} else if self.amp < 0 {
			self.amp = 0;
		}

		self.phase += 1;

		val as i8
	}

	fn sintab(&self, i: i32) -> i32 {
		self.engine.sine_table[((i >> 2) & 16383) as usize] as i32
	}

	fn distort(&self, mut val: i32, mut shift: i32) -> i32 {
		while shift > 0 {
			val = self.sintab(val);
			shift -= 1;
		}
		val
	}
}

fn mul(v16: i32, v32: u32) -> i32 {
	((v16 as i64 * (v32 >> 2) as i64) >> 16) as i32
}



fn p10(value: f32) -> i32 {
	(value * 10.0).round() as i32
}

fn p100(value: f32) -> i32 {
	(value * 100.0).round() as i32
}

fn envfun(value: f32) -> i32 {
	let v = p100(value);
	10000 / (1 + v * v)
}

fn pitchfun(value: f32) -> u32 {
	match p100(value) {
		0 => 0,
		v if v < 5 => 8 << v,
		v => (256.0 * ((v - 5) as f32 / 12.0).exp2()).round() as u32
	}
}

fn pitchfun3(value: f32) -> u32 {
	(p100(value) * 512) as u32
}

fn decayfun(value: f32) -> u32 {
	let v = p100(value) as f32 / 50.0 - 1.0;
	return ((0.0008 * v + 0.1 * v.powi(7)).exp() * 65536.0).round() as u32
}

fn decayfun3(value: f32) -> u32 {
	let v = p100(value) as f32;
	((-0.000002 * v * v).exp() * 65536.0).round() as u32
}
