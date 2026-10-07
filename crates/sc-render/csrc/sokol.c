/* The one C translation unit of sokol_gfx and sokol_log (vendored in third_party/sokol, see its
 * PROVENANCE.md), compiled by sc-render's build script with SOKOL_GLES3 (the Pi 5, Linux desktops,
 * the browser) or SOKOL_GLCORE (Windows, macOS), and SOKOL_DEBUG for the validation layer in debug
 * builds (engine-stack design section 4). No sokol_app and no sokol_glue: the platform layer makes
 * the GL context current and sc-render describes the framebuffer itself. */
#define SOKOL_IMPL
#include "sokol_log.h"
#include "sokol_gfx.h"
