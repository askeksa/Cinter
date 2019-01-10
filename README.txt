
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
1. Locate the VST plugin for your OS in the vst directory and install it in
   your favorite VST host. For example, in Renoise, you need to place it in a
   directory mentioned under VST Plugins in the Plug/Misc settings.
   The VST host must provide a GUI for generic VST parameters which can adjust
   parameter values at 0.01 precision.
2. Select the VST for an instrument and adjust parameters to your liking.
   Test the sound using the notes C-1 to B-3, which correspond to the same
   notes in Protracker on the Amiga.
3. When satisfied with an instrument, play an E-4 note. This will open a
   directory selection dialog, where you can choose where to save the sample.
   A raw 8-bit sample will be saved into the chosen directory. The name of
   the file contains an encoding of all the parameters.
4. Use these samples to make music in Protracker (or another tracker capable of
   saving in Protracker format).
5. Run the CinterConvert.py script on the Protracker module. It will output
   binary files to be included in your intro.
6. Include the Cinter4.S source file and the binary output files from the
   conversion script in your intro and use them as prescribed. See the
   Cinter4Test.S file for a usage example.


USING THE VST

Cinter is a simple, two-oscillator phase modulation synth. It has the following
parameters:

attack/decay:
  The durations for which the volume envelope of the sound rises and falls.
mpitch/bpitch (Modulation Pitch / Base Pitch):
  The pitch of the oscillators. Pitches can be adjusted in semitone increments.
  To get an in-tune sound, use a transpose of a whole number of octaves.
mpitchdecay/bpitchdecay (Modulation Pitch Decay / Base Pitch Decay):
  Time development of the oscillator pitches. The pitches can either have
  exponential falloff (values below the middle) or exponential growth (values
  above the middle).
mod (Modulation):
  How strongly the modulation oscillator modulates the base oscillator.
moddecay (Modulation Decay):
  Time development of the modulation strength. The modulation strength can
  either have exponential falloff (values below the middle) or exponential
  growth (values above the middle).
mdist/bdist (Modulation Distortion / Base Distortion):
  Distort the oscillator waveforms from a sine towards a square.
vpower (Volume envelope Power):
  How quickly the volume envelope falls off.
fdist (Final Distortion):
  Amplifies and distorts the sound after application of the volume envelope.

You can easily convert back from a saved sample to the original parameters:
- The first character is a version indicator. If this is a number, the sample
  was produced by Cinter 4, otherwise by Cinter 3.
- For the first 8 parameters, divide two digits by 100. XX means 1.
- For the last 4 parameters, divide one digit by 10. X means 1.
Enter this result as the underlying parameter value (not the one shown).


PROTRACKER GUIDELINES

You can use a combination of Cinter and non-Cinter ("raw") instruments in your
module. The Cinter instruments are recognized by their special sample names.
These sample names must be left intact, in order to communicate the instrument
parameters to the conversion script.

Samples produced by Cinter 3 and Cinter 4 can be freely used together in the
same module, as long as the Cinter 4 converter and player are used.

You can write whatever you like in the names of raw and unused instruments,
so the traditional module info can be placed here.

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

Run the CinterConvert.py script with two or three arguments: your module,
the binary songdata output file, and (if you are using any raw instruments)
the raw sampledata output file.

The script will let you know if the module contains any violations of the
restrictions mentioned in the previous section, or other nonsensical
constructs.

Even in case of errors, the conversion will run through, but errors are an
indication that something will probably not sound right.

The converter tries to emulate all quirks of Protracker 2.3d and might not be
fully compatible with other versions.


THE REPLAYER

Player source code is provided in the Cinter4.S file. There are three
important routines to know about:

CinterInit:
  Computes the samples and sets up the player state.
  A 7MHz 68000 can typically compute 2-6k of sample data per second,
  depending on the values of the parameters. Non-neutral Pitch Decay
  and Modulation Decay values take longer time, and higher distortion
  and vpower values take longer time.

CinterPlay1:
  Call as the very first thing in your vblank interrupt.
  Stops previously playing samples in channels where a new sample is to be
  triggered.

CinterPlay2:
  Call as the very last thing in your vblank interrupt.
  Modifies volumes and periods according to music.
  Waits until enough time has passed since previously playing samples
  were stopped (7.5 rasterlines), then triggers the new samples.

Alternatively, you can take over responsibility of writing the trigger mask
to the DMA enable register:
 - Set CINTER_MANUAL_DMA to 1.
 - Pick up the trigger mask, returned in D0 from CinterPlay1.
 - OR the mask with $8000 to produce a DMA enable mask.
 - Write this mask to $DFF096 at least 7.5 scanlines after CinterPlay1 returns.
 - Make sure CinterPlay2 has completed execution before you write the mask.
The Cinter4Test.S example code shows how to do this using the copper.

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

2016-08-13: Added support for raw samples.
            Treat F command as speed/tempo (only tempo 125 allowed).
            Disallow fineslide on note (never worked).
            Fixed trimming of trailing silence.
            Fixed printing of max note for instrument.

2018-03-15: Version bump to Cinter 4, due to parameter changes.
            Pitch Decay and Modulation Decay can grow upwards.
            Pitch values are adjusted in semitone increments.
            Parameters have sensible default values.
            Parameter values have descriptive display text.
            Player option to handle the DMA write manually.
            More accurate estimation of precalc time.

2018-11-18: Re-implemented the synth as a stand-alone VST in Rust.
            VST builds available for Windows, Mac and Linux.

2019-01-10: Fixed broken pitch conversion for Cinter 4 instruments.
            Open a directory dialog when saving a sample.


ACKNOWLEDGEMENTS

Cinter is developed by Aske Simon Christensen, aka Blueberry.

For questions, suggestions or other comments, write to blueberry at
loonies dot dk, or post to the Amiga Demoscene Archive forum thread at:

http://ada.untergrund.net/?p=boardthread&id=953

Thanks to Hoffman, Curt Cool, Wasp and Super-Hans for trying out the synth
during its development and showing its worth.

Cinter may be freely used and modified. Appropriate credit is appreciated.

Example modules are copyright of their individual authors.
