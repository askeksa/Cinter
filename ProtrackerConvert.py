#!/usr/bin/env python

import struct
import sys
import math

class TrackRow:
	def __init__(self, f):
		i4_period, i0_cmd, self.arg = struct.unpack(">HBB", f.read(4))
		self.period = i4_period & 0x0fff
		self.inst = (i0_cmd >> 4) | ((i4_period & 0xf000) >> 8)
		self.cmd = i0_cmd & 0x0f
		self.note = int(round(math.log(856.0 / self.period, 2) * 12.0)) if self.period > 0 else None

class Instrument:
	def __init__(self, f):
		self.name = f.read(22).rstrip('\0')
		self.length, self.finetune, self.volume, self.repoffset, self.replen = struct.unpack(">HBBHH", f.read(8))
		self.samples = None

class Module:
	def __init__(self, f):
		self.name = f.read(20).rstrip('\0')
		self.instruments = [None] * 32
		for i in range(1,32):
			self.instruments[i] = Instrument(f)
		self.songlength, dummy = struct.unpack("BB", f.read(2))
		self.positions = list(struct.unpack("128B", f.read(128)))
		mk = f.read(4)
		self.patterns = []
		num_patterns = max(self.positions) + 1
		for p in range(num_patterns):
			pat = []
			for r in range(64):
				row = []
				for t in range(4):
					trackrow = TrackRow(f)
					row.append(trackrow)
				pat.append(row)
			self.patterns.append(pat)
		for inst in self.instruments:
			if inst:
				inst.samples = f.read(inst.length * 2)


def notename(n):
	if n is None:
		return "   "
	return ["C-","C#","D-","D#","E-","F-","F#","G-","G#","A-","A#","B-"][n % 12] + str(n / 12 + 1)

def printpattern(pat):
	for r in range(64):
		for t in range(4):
			tr = pat[r][t]
			sys.stdout.write(" %3s %2X %1X %02X   " % (notename(tr.note), tr.inst, tr.cmd, tr.arg))
		print



module = Module(open(sys.argv[1], "rb"))

volumedata = [[],[],[],[]]
notedata = [[],[],[],[]]
perioddata = [[],[],[],[]]
offsetdata = [[],[],[],[]]
posdata = []
vblank = 0

speed = 6
inst = [0,0,0,0]
period = [0,0,0,0]
volume = [0,0,0,0]
portamento_target = [0,0,0,0]
portamento_speed = [0,0,0,0]

periodtable = [
	856, 808, 762, 720, 678, 640, 604, 570, 538, 508, 480, 453,
	428, 404, 381, 360, 339, 320, 302, 285, 269, 254, 240, 226,
	214, 202, 190, 180, 170, 160, 151, 143, 135, 127, 120, 113
]

reported_errors = set()
def error(msg, p, t, r):
	if (msg, p, t, r) not in reported_errors:
		print "%s in pattern %d track %d row %d" % (msg, p, t, r)
	reported_errors.add((msg, p, t, r))

startrow = 0
for p in module.positions[:module.songlength]:
	pat = module.patterns[p]
	for r in range(startrow, 64):
		row = [(t, tr, tr.arg >> 4, tr.arg & 0xF) for t, tr in enumerate(pat[r])]

		# Check for unsupported commands
		for t, tr, arg1, arg2 in row:
			if tr.cmd in [0x4, 0x6, 0x7, 0xB]:
				error("Unsupported command %X" % tr.cmd, p, t, r)
			if tr.cmd == 0xE and arg1 in [0x0, 0x3, 0x4, 0x5, 0x6, 0x7, 0xD, 0xE, 0xF]:
				error("Unsupported command E%X" % arg1, p, t, r)

		# Pick up speed and break
		patternbreak = False
		startrow = 0
		for t, tr, arg1, arg2 in row:
			if tr.cmd == 0xF:
				speed = tr.arg
			if tr.cmd == 0xD:
				patternbreak = True
				startrow = arg1 * 10 + arg2
				if startrow > 63:
					error("Break to position outside pattern", p, t, r)
					startrow = 0

		for t, tr, arg1, arg2 in row:
			# Volume data
			if tr.inst != 0:
				volume[t] = module.instruments[tr.inst].volume
			if tr.cmd == 0xC:
				volume[t] = tr.arg
			if tr.cmd == 0xE and arg1 == 0xC:
				cut = arg2
				volumedata[t] += [volume[t]] * cut + [0] * (speed - cut)
			elif tr.cmd == 0x5 or tr.cmd == 0xA:
				if arg1:
					slide = arg1
				else:
					slide = -arg2
				volumedata[t] += [max(0, min(volume[t] + i * slide, 64)) for i in range(speed)]
				volume[t] = volumedata[t][-1]
			else:
				if tr.cmd == 0xE and arg1 == 0xA:
					volume[t] = min(volume[t] + arg2, 64)
				if tr.cmd == 0xE and arg1 == 0xB:
					volume[t] = max(0, volume[t] - arg2)
				volumedata[t] += [volume[t]] * speed

			# Note trigger data
			if tr.inst != 0:
				if tr.inst != inst[t] and tr.cmd in [0x3, 0x5]:
					error("Instrument change on toneportamento", p, t, r)
				inst[t] = tr.inst
			if tr.cmd == 0xE and arg1 == 0x9 and arg2 != 0:
				for i in range(speed):
					if (i % arg2) == 0:
						notedata[t] += [inst[t]]
					else:
						notedata[t] += [0]
			elif inst[t] != 0 and tr.note is not None and tr.cmd not in [0x3, 0x5]:
				notedata[t] += [inst[t]] + [0] * (speed - 1)
			else:
				notedata[t] += [0] * speed

			# Offset data
			if tr.cmd == 0x9:
				offsetdata[t] += [tr.arg] + [0] * (speed - 1)
			else:
				offsetdata[t] += [0] * speed

			# Period data
			if tr.note is not None and tr.cmd not in [0x3, 0x5]:
				period[t] = periodtable[tr.note]
			if tr.cmd == 0x0 and tr.arg != 0:
				# Arpeggio
				note = max(i for i,p in enumerate(periodtable) if p >= period[t])
				if periodtable[note] != period[t]:
					error("Arpeggio after slide", p, t, r)
				arp = [0, arg1, arg2]
				for i in range(speed):
					perioddata[t] += [periodtable[note + arp[i % 3]]]
			elif tr.cmd in [0x1, 0x2]:
				# Portamento
				if period[t] == 0:
					error("Portamento with no source", p, t, r)
				slide = -tr.arg if tr.cmd == 0x1 else tr.arg
				perioddata[t] += [period[t] + i * slide for i in range(speed)]
				period[t] += slide * (speed - 1)
			elif tr.cmd in [0x3, 0x5]:
				# Toneportamento
				if tr.note is not None:
					portamento_target[t] = periodtable[tr.note]
				if tr.cmd == 0x3 and tr.arg != 0:
					portamento_speed[t] = tr.arg
				if period[t] == 0:
					error("Toneportamento with no source", p, t, r)
				if portamento_target[t] == 0:
					error("Toneportamento with no target", p, t, r)
				if portamento_speed[t] == 0:
					error("Toneportamento with no speed", p, t, r)
				perioddata[t] += [period[t]]
				for i in range(speed - 1):
					if portamento_target[t] > period[t]:
						period[t] = min(period[t] + portamento_speed[t], portamento_target[t])
					else:
						period[t] = max(period[t] - portamento_speed[t], portamento_target[t])
					perioddata[t] += [period[t]]
			else:
				if tr.cmd == 0xE and arg1 == 0x1:
					period[t] -= arg2
				if tr.cmd == 0xE and arg1 == 0x2:
					period[t] += arg2
				perioddata[t] += [period[t]] * speed

		# Advance
		posdata += [(p,r)] * speed
		vblank += speed
		if patternbreak:
			break

musiclength = vblank


# Find note ranges and count notes per instrument
minmax_note = dict()
inst_counts = [0] * 32
for track in range(4):
	for inst,per,offset in zip(notedata[track], perioddata[track], offsetdata[track]):
		if inst != 0:
			inst_counts[inst] += 1
			note = periodtable.index(per)
			if (inst,offset) in minmax_note:
				note_min,note_max = minmax_note[(inst,offset)]
				note_min = min(note_min, note)
				note_max = max(note_max, note)
				minmax_note[(inst,offset)] = note_min,note_max
			else:
				minmax_note[(inst,offset)] = note,note

# List of used instruments
inst_list = [inst for inst in range(32) if inst_counts[inst] != 0]
#inst_list.sort(key=(lambda i : inst_counts[i]), reverse=True)
print [(i, inst_counts[i]) for i in inst_list]

# Build note ID mapping table
note_id = 0
note_ids = dict()
note_range_list = []
for inst in inst_list:
	if (inst,0) not in minmax_note:
		note_range_list += [(0,0,0)]
		note_id += 1
	for offset in range(0,256):
		if (inst,offset) in minmax_note:
			note_min,note_max = minmax_note[(inst,offset)]
			note_range_list += [(note_min,note_max,offset)]
			for n in range(note_min, note_max+1):
				note_ids[(inst,offset,n)] = note_id
				note_id += 1

print "Generated %d different note IDs" % note_id


# Export instrument parameters
def param(s):
	if s == "X" * len(s):
		return pow(10, len(s))
	return int(s)

inst_data = ""
total_inst_size = 0
for i in inst_list:
	inst = module.instruments[i]
	try:
		p = [param(inst.name[pi*2+1:pi*2+3]) for pi in range(8)]
		p += [param(inst.name[pi+17:pi+18]) for pi in range(4)]

		length = inst.length
		if inst.repoffset == 0 and inst.replen == 1:
			replen = 0
			while inst.samples[(length-1)*2:length*2] == "\0\0":
				length -= 1
		else:
			replen = inst.replen
			if inst.repoffset != inst.length - inst.replen:
				print "Instrument %d repeat is not at end" % i
		total_inst_size += length

		# Parameters on word form for synth code
		attack      = 65536-int(math.floor(10000.0 / (1 + p[0] * p[0])))
		decay       = int(math.floor(10000.0 / (1 + p[1] * p[1])))
		mpitch      = p[2] * 512
		mpitchdecay = int(math.floor(math.exp(-0.000002 * p[3] * p[3]) * 65536)) & 0xffff
		bpitch      = p[4] * 512
		bpitchdecay = int(math.floor(math.exp(-0.000002 * p[5] * p[5]) * 65536)) & 0xffff
		mod         = p[6]
		moddecay    = int(math.floor(math.exp(-0.000002 * p[7] * p[7]) * 65536)) & 0xffff

		# Distortion parameters for synth code
		dist = (p[8] << 12) | (p[9] << 8) | (p[10] << 4) | p[11]

		inst_data  += struct.pack(">11H", length, replen, mpitch, mod, bpitch, attack, dist, decay, mpitchdecay, moddecay, bpitchdecay)
	except ValueError:
		print "Could not parse parameters for instrument %d" % i

inst_data = struct.pack(">h", len(inst_list)-1) + inst_data

# Export note ranges
note_range_data = ""
for note_min,note_max,offset in note_range_list:
	note_range_data += struct.pack(">BBH", note_min, note_max - note_min + 1, offset * 128)

# Export notes
VOLUME_SHIFT = 9
NOTE_SHIFT = 0
NOTE_ABS_MASK = 0x80

out = ""
dataset = set()
for track in [3,2,1,0]:
	initial = True
	pvol = 0
	pper = 0
	pdper = 0
	for (pat,row),vol,per,inst,offset in zip(posdata, volumedata[track], perioddata[track], notedata[track], offsetdata[track]):
		if vol == 64:
			vol = 63
		if inst != 0:
			note = periodtable.index(per)
			data = 0x8000 | (note_ids[(inst,offset,note)] << NOTE_SHIFT) | (vol << VOLUME_SHIFT)
			initial = False
		elif initial:
			data = 0
		else:
			dper = (per - pper) & 511
			dvol = (vol - pvol) & 63
			if per != pper and dper != pdper and per in periodtable:
				note = periodtable.index(per)
				data = ((NOTE_ABS_MASK | note) << NOTE_SHIFT) | (dvol << VOLUME_SHIFT)
			else:
				if per - pper < -256 or per - pper > 255:
					error("Slide value out of range (from %d to %d)" % (pper, per), pat, track, row)
					per = pper + 255 if per > pper else pper - 256
					dper = (per - pper) & 511
				if ((dper >> 7) ^ (dper >> 6)) & 1 == 1:
					error("Unsupported slide value", pat, track, row)
					dper = 63
				data = (dper << NOTE_SHIFT) | (dvol << VOLUME_SHIFT)
				pdper = dper
		out += struct.pack(">H", data)
		dataset.add(data)
		pvol = vol
		pper = per

print "Generated %d different data words" % len(dataset)
print

print "MUSIC_LENGTH = %d" % musiclength
print "NUM_INSTRUMENTS = %d" % len(inst_list)
print "INSTRUMENT_BUFFER = %d" % (total_inst_size * 2)

fout = open(sys.argv[2], "wb")
fout.write(inst_data)
fout.write(struct.pack(">HH", len(out) / 4, len(note_range_data)))
fout.write(note_range_data)
fout.write(out)
fout.close()

#pat = module.patterns[0]
#printpattern(pat)
#print module.positions
