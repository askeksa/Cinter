-- MetaSynth module for Cinter synth

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


function p(v)
	return math.floor(v * 100 + 0.5)
end

function envfun(value)
	local v = p(value)
	return math.floor(10000 / (1 + v * v))
end

function envdisplay(value)
	local f = envfun(value)
	if f == 0 then
		return "infinite", ""
	end
	return string.format("%d", math.ceil(32768 / f)), "samples"
end

function pitchfun(value)
	local v = p(value)
	if v == 0 then
		return 0
	end
	if v < 5 then
		return bit.lshift(8, v)
	end
	return math.floor(0.5 + 256 * math.pow(2, (v - 5) / 12))
end

function pitchdisplay(value)
	local v = p(value)
	if v == 0 then
		return "none", ""
	end
	if v < 5 then
		return string.format("%d oct", v - 5), ""
	end
	local t = v - 5
	if t % 12 == 0 then
		return string.format("%d oct", t / 12), ""
	end
	return string.format("%d oct %d", math.floor(t / 12), t % 12), "st"
end

function moddisplay(value)
	return string.format("%d", p(value)), ""
end

function decayfun(value)
	local v = p(value) / 50 - 1
	return math.floor(0.5 + math.exp(0.0008 * v + 0.1 * math.pow(v, 7)) * 65536)
end

function decaydisplay(value)
	local f = decayfun(value)
	return string.format("%.5f", f / 65536), ""
end

function distfun(value)
	return math.floor(0.5 + value * 10)
end

function distdisplay(value)
	return string.format("%d", distfun(value)), ""
end

function powerdisplay(value)
	return string.format("%d", distfun(value) + 1), ""
end

programs = {
	cinter4 = {
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
		paramdefaults = {
			0.05, 0.40, 0.53, 0.50, 0.65, 0.50, 0.20, 0.40, 0.0, 0.0, 0.1, 0.2
		},
		paramdisplay = {
			envdisplay,
			envdisplay,
			pitchdisplay,
			decaydisplay,
			pitchdisplay,
			decaydisplay,
			moddisplay,
			decaydisplay,
			distdisplay,
			distdisplay,
			powerdisplay,
			distdisplay,
		},
		new = function(channel, tone, velocity)
			note = {
				attack      = envfun(params.attack),
				decay       = envfun(params.decay),
				mpitch      = pitchfun(params.mpitch) * 65536,
				bpitch      = pitchfun(params.bpitch) * 65536,
				mod         = p(params.mod)           * 65536,
				mpitchdecay = decayfun(params.mpitchdecay),
				bpitchdecay = decayfun(params.bpitchdecay),
				moddecay    = decayfun(params.moddecay),
				mdist       = distfun(params.mdist),
				bdist       = distfun(params.bdist),
				vpower      = distfun(params.vpower),
				fdist       = distfun(params.fdist),

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
						filename = string.format("1%s%s%s%s%s%s%s%s%s%s%s%s.raw",
							ps(params.attack, 2), ps(params.decay, 2),
							ps(params.mpitch, 2), ps(params.mpitchdecay, 2),
							ps(params.bpitch, 2), ps(params.bpitchdecay, 2),
							ps(params.mod, 2),    ps(params.moddecay, 2),
							ps(params.mdist, 1),  ps(params.bdist, 1),
							ps(params.vpower, 1), ps(params.fdist, 1))
					else
						-- Save with a name to copy directly into asm data
						local dist = note.mdist * 4096 + note.bdist * 256 + note.vpower * 16 + note.fdist
						filename = string.format("cinter4_$%04x,$%04x,$%04x,$%04x,$%04x,$%04x,$%04x,$%04x,$%04x,$%04x.raw",
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
				function mul(v16, v32)
					return math.floor(v16 * math.floor(v32 / 4) / 65536)
				end

				local mval = distort(sintab(mul(samindex, note.mpitch)), note.mdist)
				local val = distort(sintab(mul(samindex, note.bpitch) + mul(mval, note.mod)), note.bdist)
				local p = note.vpower
				while p >= 0 do
					val = val * note.amp / 32768
					p = p - 1
				end
				val = math.min(math.floor(distort(val, note.fdist) / 128), 127)

				note.mpitch = bit.band(math.floor(note.mpitch * note.mpitchdecay / 65536), 0xFFFFFFFF)
				note.bpitch = bit.band(math.floor(note.bpitch * note.bpitchdecay / 65536), 0xFFFFFFFF)
				note.mod    = bit.band(math.floor(note.mod    * note.moddecay    / 65536), 0xFFFFFFFF)

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

printf("Cinter 4 Lua code loaded")
