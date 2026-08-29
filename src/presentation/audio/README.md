# Audio Adapter

This directory is the only place allowed to import `rodio` or own an operating
system audio device. Audio is presentation state and must never affect domain
stepping, canonical state, or deterministic results.

The Sim;Time easter-egg track is embedded into the executable. Audio device and
decoder failures are non-fatal: the hidden screen must still open silently.
