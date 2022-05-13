
pub struct IffWriter {
	data: Vec<u8>,
}

impl IffWriter {
	pub fn new() -> Self {
		Self {
			data: vec![]
		}
	}

	pub fn get_data(&self) -> &[u8] {
		self.data.as_ref()
	}

	pub fn write_chunk(&mut self, id: &str, body: impl FnOnce(&mut Self)) {
		assert!(id.len() == 4);
		self.write_bytes(id);
		let size_offset = self.data.len();
		self.write_u32(0);
		body(self);
		let size = self.data.len() - size_offset - 4;
		self.data[size_offset .. size_offset + 4].copy_from_slice(&(size as u32).to_be_bytes());
	}

	pub fn write_bytes(&mut self, bytes: impl Into<Vec<u8>>) {
		self.data.append(&mut bytes.into());
	}

	pub fn write_u8(&mut self, value: u8) {
		self.write_bytes([value]);
	}

	pub fn write_u16(&mut self, value: u16) {
		self.write_bytes(value.to_be_bytes());
	}

	pub fn write_u32(&mut self, value: u32) {
		self.write_bytes(value.to_be_bytes());
	}

	pub fn write_string_padded(&mut self, value: &str) {
		self.write_bytes(value);
		self.write_bytes(vec![0; 4 - (value.len() & 3)]);
	}
}

pub struct IffReader;

impl IffReader {
	pub fn find_chunk<'d>(data: &'d [u8], id: &str) -> anyhow::Result<&'d [u8]> {
		let mut index = 0;
		while index < data.len() - 8 {
			let chunk = &data[index .. index + 4];
			let length = u32::from_be_bytes(data[index + 4 .. index + 8].try_into().unwrap()) as usize;
			let start = index + 8;
			if length > data.len() - start {
				return Err(anyhow::anyhow!("{} chunk truncated", String::from_utf8_lossy(chunk)));
			}
			if chunk == id.as_bytes() {
				return Ok(&data[start .. start + length]);
			}
			index = start + length;
		}
		Err(anyhow::anyhow!("Could not find {} chunk", id))
	}
}
