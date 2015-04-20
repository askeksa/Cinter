
Cinter is a software synthesizer designed for use in Amiga 4k intros.

The synth has three conponents:
- A VST instrument for creating sounds
- A script for converting music
- Replay code for inclusion in your intro

You will need:
- A Windows computer (or a way to use Windows VSTs)
- A suitable VST host
- Protracker, or a compatible tracker
- Python 2.x, where x >= 5
- An assembler, but you would need that for the intro anyway. :)


To use the synth, you need to follow these steps:
1. Download MetaSynth.dll from https://bitbucket.org/askeksa/metasynth/downloads
   (or use the one included in the Cinter distribution archive). This is a
   generic VST which delegates the actual sound generation to a Lua program.
2. Install the VST in your favorite VST host. For example, in Renoise, you need
   to place it in a directory mentioned under VST Plugins in the Plug/Misc
   settings. The VST host must provide a GUI for generic VST parameters which
   can adjust parameter values at 0.01 precision.
3. Select the VST for an instrument. In the file selector that pops up, select
   the Cinter3.lua file.
4. Adjust parameters to your liking. Test the sound using the notes C-1 to B-3,
   which correspond to the same notes in Protracker on the Amiga.
5. When satisfied with an instrument, play an E-4 note. This will save a raw
   8-bit sample into the directory where Cinter3.lua is located. The name of
   the file contains an encoding of all the parameters.
6. Use these samples to make music in Protracker (or another tracker capable of
   saving in Protracker format).
7. Run the ProtrackerConvert.py script on the Protracker module. It will output
   a binary file to be included in your intro.
8. Include the Cinter3.S source file and the binary output file from the
   conversion script in your intro and use them as prescribed.
9. Profit. :)


USING THE VST

Cinter is a simple, two-oscillator phase modulation synth. It has the following
parameters:

attack/decay:
  The durations for which the volume envelope of the sound rises and falls.
mpitch/bpitch (Modulation Pitch / Base Pitch):
  The pitch of the oscillators. To get an in-tune sound, set each of these to
  one of 0.01, 0.02, 0.04, 0.08, 0.16, 0.32 or 0.64.
mpitchdecay/bpitchdecay (Modulation Pitch Decay / Base Pitch Decay):
  Falloff of the oscillator pitches.
mod (Modulation):
  How strongly the modulation oscillator modulates the base oscillator.
moddecay (Modulation Decay):
  Falloff of the modulation strength.
mdist/bdist (Modulation Distortion / Base Distortion):
  Distort the oscillator waveforms from a sine towards a square.
vpower (Volume envelope Power):
  How quickly the volume envelope falls off.
fdist (Final Distortion):
  Amplifies and distorts the sound after application of the volume envelope.

At least one of attack and decay and at least one of the pitches must be raised
above zero to get any sound.


PROTRACKER GUIDELINES

You may change the initial 's' in the sample names to a different character,
but the sample names must be otherwise intact, in order to communicate the
instrument parameters to the conversion script.

You can write whatever you like in the names of unused instruments, so the
traditional module info can be placed here.

You are allowed to shorten instruments by changing their lengths or cutting
from the end in the sample editor. The new length will be in effect, both in
terms of replay, memory usage and precalculation time.

Finetune must be zero for all instruments.

Instrument volume can be set arbitrarily.*

Sample repeat must be either absent (offset 0, length 2) or placed at the very
end of the (possibly shortened) sample.

Support for effect commands are as follows:
0     (arpeggio)      Supported
1,2,3 (portamento)    Supported, but only for slide values 00-3F and C0-FF.
4,6   (vibrato)       Not supported
5,A   (volume slide)  Supported*
7     (tremolo)       Not supported
9     (sampleoffset)  Supported
B     (positionjump)  Not supported
C     (volume)        Supported*
D     (patternbreak)  Supported
E0    (filter)        Not supported
E1,E2 (fineslide)     Supported
E3    (glissando)     Not supported
E4    (vibr control)  Not supported
E5    (finetune)      Not supported
E6    (patternloop)   Not supported
E7    (trem control)  Not supported
E9    (retrig)        Supported
EA,EB (finevolume)    Supported*
EC    (notecut)       Supported
ED    (notedelay)     Not supported
EE    (patterndelay)  Not supported
EF    (invert loop)   Not supported
F     (speed)         Only vblank timing supported. F00 (stop) not supported.

*: All volumes (0-64) are supported, but volume 64 will be played as 63.


THE CONVERSION SCRIPT

Run the ProtrackerConvert.py script with two arguments: your module, and the
output file.

The script will let you know if the module contains any violations of the
restrictions mentioned in the previous section, or other nonsensical
constructs.

If successful, it will write a binary file for inclusion into the intro.


THE REPLAYER

Player source code is provided in the Cinter3.S file. There are three
important routines to know about:

CinterInit:
  Computes the samples and sets up the player state.
  A 7MHz 68000 can typically compute 5-7k of sample data per second,
  depending on the values of the distortion and vpower parameters
  (higher parameter values result in slower computation).

CinterPlay1:
  Call as the very first thing in your vblank interrupt.
  Stops previously playing samples in channels where a new sample is to be
  triggered.

CinterPlay2:
  Call as the very last thing in your vblank interrupt.
  Modifies volumes and periods according to music.
  Waits until enough time has passed since previously playing samples
  were stopped (7.5 rasterlines), then triggers the new samples.

The replayer does not stop automatically at the end of the music. The
intro must exit before the music reaches its end or the replayer will
play random garbage from memory. If you want extra silence at the end,
you must add empty patterns as appropriate.

