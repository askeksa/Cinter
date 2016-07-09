
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

You can easily convert back from a saved sample to the original parameters:
- For the first 8 parameters, divide two digits by 100. XX means 1.
- For the last 4 parameters, divide one digit by 10. X means 1.


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
0     (arpeggio)      Supported as long as the base pitch matches a pure note.
1,2,3 (portamento)    Supported, but only for slide values 00-3F and C0-FF.
4,6   (vibrato)       Not supported
5,A   (volume slide)  Supported*
7     (tremolo)       Not supported
9     (sampleoffset)  Supported
B     (positionjump)  Supported
C     (volume)        Supported*
D     (patternbreak)  Supported
E0    (filter)        Not supported
E1,E2 (fineslide)     Supported, except directly on notes.
E3    (glissando)     Not supported
E4    (vibr control)  Not supported
E5    (finetune)      Not supported
E6    (patternloop)   Not supported
E7    (trem control)  Not supported
E9    (retrig)        Supported
EA,EB (finevolume)    Supported*
EC    (notecut)       Supported
ED    (notedelay)     Supported
EE    (patterndelay)  Supported
EF    (invert loop)   Not supported
F     (speed)         Only vblank timing supported.

*: All volumes (0-64) are supported, but volume 64 will be played as 63.

The converter will assign different note IDs to different combinations of
instrument, tone and sampleoffset. Each note is represented in the music data
by its note ID.

The total number of note IDs needed for a song is computed like this: sum the
number of tones between the lowest and highest note (both included) for each
instrument / sampleoffset combination. This number must be at most 512.


THE CONVERSION SCRIPT

Run the ProtrackerConvert.py script with two arguments: your module, and the
binary output file.

The script will let you know if the module contains any violations of the
restrictions mentioned in the previous section, or other nonsensical
constructs.

Even in case of errors, the conversion will run through, but errors are an
indication that something will probably not sound right.

The converter tries to emulate all quirks of Protracker 2.3d and might not be
fully compatible with other versions.


THE REPLAYER

Player source code is provided in the Cinter3.S file. There are three
important routines to know about:

CinterInit:
  Computes the samples and sets up the player state.
  A 7MHz 68000 can typically compute 2-6k of sample data per second,
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

At the end of the music, the player behaves just as when playing the module
in Protracker: If the module contains an F00 command (stop), the music will
stop when reaching this command. Otherwise, it will restart from the
beginning when it reaches the end of the last pattern. The B command can be
used to produce other looping behavior as desired.


VERSION HISTORY

2015-04-21: First public version.

2015-05-19: Fixed conversion of 9 command with argument 00.

2015-10-31: Fixed replay of 9 command with argument >= 80.
            Support conversion of lowercase sample names.
            Fixed bug in sample name generation in the Lua synth.
            Support stopping or restarting at the end of the music.
            Added support for F command with argument 00 (stop).
            Added converter support for commands B, ED and EE.


ACKNOWLEDGEMENTS

Cinter is developed by Aske Simon Christensen, aka Blueberry.

For questions, suggestions or other comments, write to blueberry at
loonies dot dk, or post to the Amiga Demoscene Archive forum thread at:

http://ada.untergrund.net/?p=boardthread&id=953

Thanks to Hoffman, Curt Cool, Wasp and Super-Hans for trying out the synth
during its development and showing its worth.

Cinter may be freely used and modified. Appropriate credit is appreciated.
