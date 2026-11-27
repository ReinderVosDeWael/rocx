# Layout test font

`DejaVuSans-subset.ttf` is a 52,736-byte test subset of DejaVu Sans 2.37
(Debian `fonts-dejavu-core` 2.37-8, regular face). Source:
<https://dejavu-fonts.github.io/>. See [LICENSE.txt](LICENSE.txt).

The subset was generated with HarfBuzz 10.2.0 using
`hb_subset_input_unicode_set`, with otherwise default subset options:
U+0020–007E, U+00A0–00FF, U+0300–036F, U+2010–2026, and U+1F600.
The subset retains OpenType substitutions and positioning for the tests.
It is shipped solely as fixture data; no system font lookup is required.

SHA-256:

- Source: `ae7b7855e115a5966d8b1b3f80f254ccc117ec86f9965e202ee2940453837280`
- Subset: `ac6e1cd9abc68c06cd7a56ea128d79b55fc0af4bc0be247b9f3737bf01ee11e4`
