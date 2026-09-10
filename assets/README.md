# Sim;X Bundled Assets

This directory keeps compile-time media out of the repository root while
preserving deterministic builds.

- `audio/hacking_to_the_gate.mp3` is embedded by the presentation-only audio
  adapter for the Sim;Time easter egg.
- `icons/github.svg`, `icons/youtube.svg`, and `icons/telegram.svg` are parsed
  once into the retained Sim;Engine social-icon atlas.

These source assets are required at compile time and must not be deleted or
ignored merely because the resulting bytes are embedded in the executable.

Presence in this repository is not proof of redistribution or commercial-use
permission. Release rights and required notices are governed by
`docs/product/DISTRIBUTION_AND_MONETIZATION.md`. In particular, the current
music track is development-only for release planning until explicit rights are
documented.
