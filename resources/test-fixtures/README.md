# Audio regression fixture

`reservoir.mp3` is generated test audio: two seconds of silence, two seconds of a
440 Hz sine tone, then two seconds of silence (mono, 44.1 kHz, LAME MP3 at 64 kbit/s).
It contains no recorded or third-party music.

It exercises MP3 bit-reservoir dependencies during waveform analysis. The test
checks the waveform, progressive updates, and absence of decoder warnings on this
valid input; logging remains enabled.
