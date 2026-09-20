# Bundled Menu Fonts

These are unmodified, static TTF files from their official upstream projects.
There is no system-font discovery, runtime download, variable-font conversion,
or fallback. Keep both original OFL notices with distributed binaries.

## IBM Plex Sans Medium

- Role: all current UI labels, with shared 14, 18, and 26 logical-pixel styles.
- Source: [IBM/plex](https://github.com/IBM/plex).
- Commit: `78cd4223d8de9fcb78cba84eadecb269c56093c5`.
- Upstream file:
  [`packages/plex-sans/fonts/complete/ttf/IBMPlexSans-Medium.ttf`](https://github.com/IBM/plex/blob/78cd4223d8de9fcb78cba84eadecb269c56093c5/packages/plex-sans/fonts/complete/ttf/IBMPlexSans-Medium.ttf).
- Original notice: `OFL-IBMPlexSans.txt`, copied from the adjacent upstream
  `license.txt`; SIL OFL 1.1, IBM copyright and reserved name Plex preserved.
- Size: 202,460 bytes.
- SHA-256: `331c8639d7598b2cde62a911a71db195e30cb655cd6bdf2e324a7e984955f907`.
- Local `fc-scan` identifies IBM Plex Sans Medium, and its character map
  includes U+0400 through U+045F. A regression test shapes both cases of the
  complete Russian alphabet, including Yo, at every UI size. This verifies
  glyph availability, not completion of application localization.

## Space Grotesk Medium

- Role: the `Sim;X` wordmark, 88 logical pixels. Other headings retain Plex.
- Source: [floriankarsten/space-grotesk](https://github.com/floriankarsten/space-grotesk).
- Commit: `03507d024a01282884232081fc6011c09ff4e849`.
- Upstream file:
  [`fonts/ttf/static/SpaceGrotesk-Medium.ttf`](https://github.com/floriankarsten/space-grotesk/blob/03507d024a01282884232081fc6011c09ff4e849/fonts/ttf/static/SpaceGrotesk-Medium.ttf).
- Original notice: `OFL-SpaceGrotesk.txt`, copied from upstream `OFL.txt`;
  SIL OFL 1.1, copyright attribution preserved.
- Size: 116,664 bytes.
- SHA-256: `3183b7eb0b4241476360e2cd8e527868616cc16d3ce332c4376505989b772b44`.
- Local `fc-scan` identifies Space Grotesk Medium. The regression test shapes
  the complete `Sim;X` wordmark, including its semicolon. This face lacks the
  Russian alphabet and is deliberately not used for localized UI text.

## Resource Contract

The menu registers four font styles backed by two unique source allocations:
319,124 source bytes total. Atlas identities remain per style. The three Plex
styles must report `shares_face_with`; the logo must not share their face.

IBM Plex Mono remains the planned companion for parameters, numbers, and code.
There is no such scientific UI in the current milestone, so it is not bundled
or registered yet. Downloading another face later requires an explicit source,
license notice, size budget, and glyph/rendering check.
