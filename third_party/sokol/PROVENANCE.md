# sokol, vendored

Vendored for `sc-render` (openspec/changes/engine-stack, design section 4: "How sokol_gfx is
built and bound"). These files are copied unchanged; never edit them. Updating them is one
commit that replaces every file below from one set of revisions, regenerates every shader
(`python3 tools/sokol_shaders.py`) and runs the render tests.

| File | Source | Revision | sha256 |
| --- | --- | --- | --- |
| `sokol_gfx.h` | https://github.com/floooh/sokol | `401f21f` | `e23802d543747b1d24b4ab76e52634fb8b0a4a30afdb9f1437f63814f6bc77d7` |
| `sokol_log.h` | https://github.com/floooh/sokol | `401f21f` | `a8d55de21694410740e752ea8e57c4103d5951d20339ba9712e24232adef9a42` |
| `gfx.rs` | https://github.com/floooh/sokol-rust (`src/gfx.rs`, generated from the headers) | `1a5cb22` | `b20a013ee50535794fd20976deeafb60082856d766fe7ee7f097f71197143994` |
| `log.rs` | https://github.com/floooh/sokol-rust (`src/log.rs`) | `1a5cb22` | `37d3ec29dbf0f7737be12384c04e6164a6de805ef9ff50754fca4fca3343b59b` |

The shader compiler, `sokol-shdc`, is not committed (11.6 MB a platform). `tools/sokol_shaders.py`
fetches it from https://github.com/floooh/sokol-tools-bin at revision `11d0cf6` into
`third_party/sokol-shdc/` (ignored by git) and refuses a binary whose sha256 differs:

| Platform | Path in sokol-tools-bin | sha256 |
| --- | --- | --- |
| Linux x86-64 | `bin/linux/sokol-shdc` | `ed35e89ef381d521a499096ed4ada85e4d135d8011e151cca6b7d893c43b21df` |
| Linux arm64 (a Pi 5) | `bin/linux_arm64/sokol-shdc` | `446b4bcea0c81d3ae529bc0d93533ea661b017f5b9ec2b2293a4c85f5fdcb639` |

sokol, sokol-rust and sokol-tools are by Andre Weissflog, under the zlib/libpng license (sokol) and
the MIT license (sokol-rust, sokol-tools); the license texts are at the top of each header and in
each source repository.
