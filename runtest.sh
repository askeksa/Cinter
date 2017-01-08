#!/bin/bash
for mod in examples/*.mod; do
	M1=${mod/examples/test}
	M2=${M1/.mod/.cinter3}
	M3=${M1/.mod/.out}
	./ProtrackerConvert.py $mod $M2 >$M3
	echo `tail -1 $M3` `md5sum $M2`
done
