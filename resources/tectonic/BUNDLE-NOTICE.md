# Tectonic runtime and TeX support bundle

mdx release archives include the official Tectonic 0.17.0 executable and a
curated directory bundle generated from Tectonic's default bundle v33. The
curated bundle contains only the TeX support files resolved by
`warmup.tex`; it is intended for mdx's built-in official and research
templates, not as a general TeX Live replacement.

- Tectonic source and releases: <https://github.com/tectonic-typesetting/tectonic>
- Default bundle endpoint: <https://relay.fullyjustified.net/default_bundle_v33.tar>
- Bundle preparation script: `scripts/prepare-tectonic-bundle.sh`
- Tectonic license: `LICENSE-TECTONIC.txt`

The TeX support files originate from TeX Live packages and retain their
respective upstream licenses. Those licenses vary by package (commonly LPPL,
GPL, BSD, MIT, or public-domain terms). mdx does not modify those support files;
the release workflow selects and redistributes the byte-identical files fetched
from Tectonic's official default bundle.

No fonts from mdx's repository `font/` directory are included in release
archives. Users install the required FounderType and JetBrains Mono fonts
separately on the target system.
