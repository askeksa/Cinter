#!/usr/bin/env python2

import struct
import sys
import math

data = open(sys.argv[1], "rb").read()

def valstring(v, d):
	maxv = math.pow(10, d)
	if v > maxv:
		v = maxv
	if v == maxv:
		return "X" * d
	return "%0*d" % (d, v)

print "                         Att Dec Mpi Mpd Bpi Bpd Mod Mdd Mdi Bdi Vpo Fdi"
for i in range(1,32):
	ns = 20+30*(i-1)
	ne = 20+30*(i-1)+21
	name = data[ns:ne]
	try:
		p = [int(name[pi*2+5:pi*2+7], 16) for pi in range(8)]
		try:
			dist = int(name[1:5], 16)
		except ValueError:
			dist = 0x8880

		rdist = ((dist >> 3) | (dist << 13)) & 0xFFFF
		rdist = rdist - 0x1110
		p += [(rdist >> s) & 0xF for s in [0,4,8,12]]

		newname = name[0]
		for pi in range(8):
			newname += valstring(p[pi], 2)
		for pi in range(8,12):
			newname += valstring(p[pi], 1)

		data = data[:ns] + newname + data[ne:]

		print ("%s -> " + ("%3d "*12) + " -> %s") % ((name,) + tuple(p) + (newname,))
	except ValueError:
		print name

of = open(sys.argv[2], "wb")
of.write(data)
of.close()
