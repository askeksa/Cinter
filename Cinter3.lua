
local ffi = require("ffi")
local bit = require("bit")
local event_type = ffi.typeof("struct { int delta; unsigned char midi[3]; } *")
local buffer_type = ffi.typeof("float **")

function printf(...)
	print(string.format(...))
	io.flush()
end

function process(num_events, events, num_samples, inputs, outputs, program)
	events = ffi.cast(event_type, events)
	outputs = ffi.cast(buffer_type, outputs)
	program = programs[program]
	if program then
		local event_index = 0
		local pos = 0
		while event_index < num_events do
			if pos < events[event_index].delta then
				render(inputs, outputs, pos, events[event_index].delta)
				pos = events[event_index].delta
			end
			handle_event(events[event_index], program)
			event_index = event_index + 1
		end
		if pos < num_samples then
			render(inputs, outputs, pos, num_samples)
		end
	end
end

--persistent.notes = {}
persistent.notes = persistent.notes or {}
notes = persistent.notes

function handle_event(event, program)
	--printf("  %02x %02x %02x", event.midi[0], event.midi[1], event.midi[2])
	local cmd = bit.rshift(event.midi[0], 4)
	local channel = bit.band(event.midi[0], 15)
	local tone = event.midi[1]
	local velocity = event.midi[2]
	if cmd == 9 then
		-- note on
		if program.new then
			note = program.new(channel, tone, velocity)
			note.program = program
			note.channel = channel
			note.tone = tone
			note._time = 0
			note._timestep = 1 / samplerate
			note._released = false
			table.insert(notes, note)
		end
	elseif cmd == 8 then
		-- note off
		for i,note in ipairs(notes) do
			if not note._released and note.channel == channel and note.tone == tone then
				note:off(note._time, velocity)
				note._released = true
			end
		end
	end
end

function render(inputs, outputs, start, stop)
	for i,note in ipairs(notes) do
		for i = start,stop-1 do
			left, right = note:render(note._time)
			outputs[0][i] = outputs[0][i] + left
			outputs[1][i] = outputs[1][i] + right
			note._time = note._time + note._timestep
		end
	end
	for i = 1, #notes do
		while notes[i] and not notes[i]:alive(notes[i]._time) do
			table.remove(notes, i)
		end
	end
end

programs = {
	cinter3 = {
		paramnames = {
			"attack",
			"decay",
			"mpitch",
			"mpitchdecay",
			"bpitch",
			"bpitchdecay",
			"mod",
			"moddecay",
			"mdist",
			"bdist",
			"vpower",
			"fdist"
		},
		new = function(channel, tone, velocity)
			function p(v)
				return math.floor(v * 100 + 0.5)
			end
			function ps(v, width)
				if width == 1 then
					local p = math.floor(v * 10 + 0.5)
					if p == 10 then return "X" end
					return string.format("%01d", p)
				else
					local p = math.floor(v * 100 + 0.5)
					if p == 100 then return "XX" end
					return string.format("%02d", p)
				end
			end

			note = {
				attack      = math.floor(10000 / (1 + p(params.attack) * p(params.attack))),
				decay       = math.floor(10000 / (1 + p(params.decay)  * p(params.decay) )),
				mpitch      = p(params.mpitch) * 512,
				bpitch      = p(params.bpitch) * 512,
				mod         = p(params.mod),
				mpitchdecay = math.floor(math.exp(-0.000002 * p(params.mpitchdecay) * p(params.mpitchdecay)) * 65536),
				bpitchdecay = math.floor(math.exp(-0.000002 * p(params.bpitchdecay) * p(params.bpitchdecay)) * 65536),
				moddecay    = math.floor(math.exp(-0.000002 * p(params.moddecay)    * p(params.moddecay)   ) * 65536),
				mdist       = math.floor(0.5 + params.mdist * 10),
				bdist       = math.floor(0.5 + params.bdist * 10),
				vpower      = math.floor(0.5 + params.vpower * 10),
				fdist       = math.floor(0.5 + params.fdist * 10),

				amp            = 0,
				attacking      = true,
				released       = false,
				releasetime    = 0,
				phase          = 0,
				lowpass_state  = 0,
				highpass_state = 0,
				samfreq        = 440 * 2 ^ ((tone + 27) / 12) / samplerate,
				sampledata     = { 0, 0 },
				tone           = tone
			}

			function note.off(note, time, velocity)
				note.released = true
				note.releasetime = time

				-- Play E-4 or F-4 to save sample
				if note.tone == 52 or note.tone == 53 then
					while #note.sampledata < 65534 and note.amp > 0 do
						table.insert(note.sampledata, note:singlesample(#note.sampledata-2))
					end
					l = #note.sampledata
					while l > 1 and note.sampledata[l] == 0 do
						l = l - 1
					end
					if bit.band(l, 1) == 1 then
						l = l + 1
						note.sampledata[l] = 0
					end

					local filename
					if note.tone == 52 then
						-- Save with Protracker-compatible name
						filename = string.format("s%s%s%s%s%s%s%s%s%s%s%s%s.raw",
							ps(params.attack, 2), ps(params.decay, 2),
							ps(params.mpitch, 2), ps(params.mpitchdecay, 2),
							ps(params.bpitch, 2), ps(params.bpitchdecay, 2),
							ps(params.mod, 2),    ps(params.moddecay, 2),
							ps(params.mdist, 1),  ps(params.bdist, 1),
							ps(params.vpower, 1), ps(params.fdist, 1))
					else
						-- Save with a name to copy directly into asm data
						local dist = note.mdist * 4096 + note.bdist * 256 + note.vpower * 16 + note.fdist
						filename = string.format("sample_$%04x,$%04x,$%04x,$%04x,$%04x,$%04x,$%04x,$%04x,$%04x,$%04x.raw",
							l / 2,
							p(params.mpitch) * 512, p(params.mod), p(params.bpitch) * 512,
							bit.band(-note.attack, 65535), dist, note.decay,
							bit.band(note.mpitchdecay, 65535), bit.band(note.moddecay, 65535), bit.band(note.bpitchdecay, 65535))
					end
					f = io.open(filename, "wb")
					for i = 1, l do
						f:write(string.char(bit.band(note.sampledata[i], 255)))
					end
					f:close()
				end
			end
			function note.alive(note, time)
				return not note.released or time < note.releasetime + 0.001
			end
			function note.singlesample(note, samindex)
				function sintab(i)
					return math.floor(0.5 + 16384 * math.sin(math.floor(i / 4) / 16384 * (2 * math.pi)))
				end
				function distort(val, shift)
					while shift > 0 do
						val = sintab(val)
						shift = shift - 1
					end
					return val
				end
				local mpitch = math.floor(note.mpitch * 16384) / 65536
				local bpitch = math.floor(note.bpitch * 16384) / 65536
				local mod = math.floor(note.mod * 16384) / 65536
				local mval = distort(sintab(samindex * mpitch), note.mdist)
				val = distort(sintab(samindex * bpitch + mval * mod), note.bdist)
				local p = note.vpower
				while p >= 0 do
					val = val * note.amp / 32768
					p = p - 1
				end
				val = math.min(math.floor(distort(val, note.fdist) / 128), 127)

				note.mpitch = math.floor(note.mpitch * note.mpitchdecay) / 65536
				note.bpitch = math.floor(note.bpitch * note.bpitchdecay) / 65536
				note.mod    = math.floor(note.mod    * note.moddecay   ) / 65536

				if note.attacking then
					note.amp = note.amp + note.attack
					if note.amp > 32767 then
						note.amp = 32767
						note.attacking = false
					end
				else
					note.amp = note.amp - note.decay
					if note.amp < 0 then
						note.amp = 0
					end
				end

				return val
			end
			function note.render(note, time)
				while #note.sampledata <= note.phase + 3 do
					table.insert(note.sampledata, note:singlesample(#note.sampledata-2))
				end

				-- Catmull-Rom interpolation
				local t = note.phase - math.floor(note.phase)
				local a0 = t*((2-t)*t-1)
				local a1 = t*t*(3*t-5)+2
				local a2 = t*((4-3*t)*t+1)
				local a3 = t*t*(t-1)
				local d0 = note.sampledata[#note.sampledata-3]
				local d1 = note.sampledata[#note.sampledata-2]
				local d2 = note.sampledata[#note.sampledata-1]
				local d3 = note.sampledata[#note.sampledata-0]
				local v = a0*d0 + a1*d1 + a2*d2 + a3*d3

				-- Update phase
				note.phase = note.phase + note.samfreq

				if note.released then
					v = v * math.max(0, note.releasetime + 0.001 - time) / 0.001
				end

				return v / 254, v / 254
			end

			return note
		end
	}
}

printf("Cinter 3 Lua code loaded")
