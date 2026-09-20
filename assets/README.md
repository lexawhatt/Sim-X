# Sim;X Bundled Assets

The new main menu embeds its assets at compile time. It does not depend on
runtime file paths, installed fonts, or asset downloads.

- `fonts/IBMPlexSans-Medium.ttf`: unmodified static IBM Plex Sans Medium for
  the interface, including Russian Cyrillic. Its three 14/18/26 px styles share
  one parsed face and source allocation through Sim;Logic's font-style API.
- `fonts/SpaceGrotesk-Medium.ttf`: unmodified static Space Grotesk Medium for
  the 88 px `Sim;X` wordmark. It is not a Cyrillic UI fallback.
- Both fonts use SIL Open Font License 1.1. Exact upstream commits, checksums,
  and redistribution notices are recorded in `fonts/README.md` and the adjacent
  `OFL-*.txt` files. IBM Plex Mono is reserved for future numeric/code UI; the
  current menu does not load or ship an unused Mono face.
- `icons/github.svg`, `icons/youtube.svg`, `icons/telegram.svg`: the three
  user-supplied social marks, rasterized once at startup into separate bounded
  Sim;Logic image assets. Aspect ratios are preserved.
- `illustrations/physics.svg`: original immutable orbit artwork, decoded once
  and rotated using Sim;Logic's native image API. Animated motif lines and
  circles are retained Sim;Logic visuals defined in `menu/backdrop`.
  They are decorative metaphors, not scientific simulations.
- The soft glow is generated once by `src/menu/assets/halo.rs`: a white Gaussian
  alpha texture with deterministic dithering, tinted by the active domain.
  This replaces the banded low-opacity SVG gradient without per-frame blur.
- `audio/hacking_to_the_gate.mp3`: retained historical asset, unused by the
  rewrite. No audio bytes or playback code are included in the new executable.

Required font notices must accompany distributed binaries. Presence of an
asset here does not establish redistribution permission; the retained music
must not be distributed without the relevant rights. Local product policy
lives in `workflow/product/DISTRIBUTION_AND_MONETIZATION.md`.
