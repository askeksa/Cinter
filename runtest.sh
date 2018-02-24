#!/bin/bash
for mod in examples/*.mod; do
	M1=${mod/examples/test}
	M2=${M1/.mod/.cinter4}
	M3=${M1/.mod/.out}
	convert/CinterConvert.py $mod $M2 >$M3
	echo `grep "[ ]errors[.]" $M3` `md5sum $M2`
done
