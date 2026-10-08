//! `sc-render`: the renderer, on sokol_gfx over OpenGL ES 3.0 (openspec/changes/engine-stack,
//! design sections 3, 4 and 7).
//!
//! It owns sokol_gfx's resources and pipelines, the shaders (one `sokol-shdc` source each, in
//! `shaders/`), the deck vertex layout (built from `sc-core`'s table, never retyped), the 3D target
//! and the blit to the output. It is handed a current GL context and a framebuffer size by the
//! platform layer and never calls SDL; it reads core types and never decides a gameplay outcome.
//!
//! Every call into sokol_gfx is made on the thread that owns the GL context, between `Renderer::new`
//! and drop.

#[allow(clippy::all, missing_docs, unused_imports, dead_code)]
#[path = "../../../third_party/sokol/gfx.rs"]
pub mod gfx;
#[allow(clippy::all, missing_docs, unused_imports, dead_code)]
#[path = "../../../third_party/sokol/log.rs"]
mod sokol_log;

/// The generated shader modules (tools/sokol_shaders.py; never edited by hand).
#[allow(clippy::all, missing_docs, dead_code)]
pub mod shaders {
    pub mod blit;
    pub mod deck;
    pub mod sky;
}

use gfx as sg;
use sc_core::data::RenderConfig;
use sc_core::vertex::{AttrFormat, DECK_ATTRIBUTES, DECK_VERTEX_BYTES};
use std::collections::HashMap;

/// The colour and depth formats of every target and of the output (engine-stack design 7: 8-bit
/// colour, no float targets assumed).
const COLOR_FORMAT: sg::PixelFormat = sg::PixelFormat::Rgba8;
const DEPTH_FORMAT: sg::PixelFormat = sg::PixelFormat::DepthStencil;

/// Which deck program draws a mesh.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum DeckProgram {
    /// Baked colours times a texture array layer.
    Textured,
    /// Baked colours only (no texture fetch).
    Flat,
}

/// The width of a mesh's indices.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum IndexWidth {
    /// 16-bit: a draw of at most 65,535 vertices (deck-pipeline section 5).
    U16,
    /// 32-bit.
    U32,
}

/// What a deck draw is lit with.
#[derive(Copy, Clone, Debug)]
pub struct DeckParams {
    /// Model, view and projection, column-major.
    pub mvp: glam::Mat4,
    /// Weights of the normal, red alert and emergency colour sets (they sum to 1).
    pub state_weights: [f32; 3],
    /// A dynamic light: direction towards it, and its strength (0: none).
    pub flash_dir: glam::Vec3,
    /// The dynamic light's strength.
    pub flash: f32,
    /// The first texture layer whose alpha is an emission mask (`u32::MAX`: none).
    pub panel_first: u32,
    /// How bright emission masks glow now (1: as the texel).
    pub panel_glow: f32,
}

/// The sky's numbers (deck-pipeline 13b; `data/space/exterior.json` through the client): directions are ship
/// coordinates, colours linear. See `shaders/sky.glsl` for each lane.
#[derive(Copy, Clone, Debug)]
pub struct SkyParams {
    /// The inverse of the projection times the view's rotation (no translation).
    pub inv_vp: glam::Mat4,
    /// The way to the sun, and the cosine of the disc's angular radius.
    pub sun: [f32; 4],
    /// The sun's colour, and the glow's strength.
    pub sun_colour: [f32; 4],
    /// The way to the planet's centre, and the sine of its angular radius.
    pub planet: [f32; 4],
    /// Sea, and the share of land.
    pub ocean: [f32; 4],
    /// Land, and the share of cloud.
    pub land: [f32; 4],
    /// Cloud, and the night side's brightness.
    pub cloud: [f32; 4],
    /// The atmosphere's rim, and its thickness as a share of the radius.
    pub atmosphere: [f32; 4],
    /// Star density, brightness, and a pixel's size in radians.
    pub stars: [f32; 4],
}

/// A deck mesh on the GPU: 28-byte vertices and their indices. Dropping it frees its buffers.
pub struct Mesh {
    vbuf: sg::Buffer,
    ibuf: sg::Buffer,
    index_count: usize,
    width: IndexWidth,
}

impl Drop for Mesh {
    fn drop(&mut self) {
        // A mesh dropped after the renderer has nothing left to free.
        if sg::isvalid() {
            sg::destroy_buffer(self.vbuf);
            sg::destroy_buffer(self.ibuf);
        }
    }
}

impl Mesh {
    /// Triangles in the mesh.
    pub fn triangles(&self) -> usize {
        self.index_count / 3
    }
}

/// A texture array: one layer per material (surface-materials), RGBA8, square.
pub struct TextureArray {
    view: sg::View,
}

/// The 3D target: colour (multisampled when asked) and depth, resolved to a texture the blit reads.
pub struct Target {
    width: u32,
    height: u32,
    samples: i32,
    color: sg::View,
    resolve: Option<sg::View>,
    depth: sg::View,
    texture: sg::View,
}

impl Target {
    /// Pixel size.
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
    /// MSAA samples (1: none).
    pub fn samples(&self) -> i32 {
        self.samples
    }
}

/// Counts from the frame sokol_gfx last committed (engine-stack design 11, scene 8).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct FrameCounts {
    /// Draw calls.
    pub draws: u32,
    /// Pipelines applied.
    pub pipelines: u32,
    /// Bindings applied.
    pub bindings: u32,
    /// Uniform blocks applied.
    pub uniforms: u32,
    /// Passes.
    pub passes: u32,
}

/// The renderer: sokol_gfx set up with the pools `data/engine/render.json` gives, and its pipelines.
pub struct Renderer {
    pipes: HashMap<(DeckProgram, IndexWidth, i32), sg::Pipeline>,
    shaders: HashMap<DeckProgram, sg::Shader>,
    /// The sky and screen programs, and their pipelines by sample count (false: sky, true: screen).
    sky_shader: sg::Shader,
    screen_shader: sg::Shader,
    outside_pipes: HashMap<(bool, i32), sg::Pipeline>,
    blit: sg::Pipeline,
    nearest: sg::Sampler,
    linear: sg::Sampler,
    deck_sampler: sg::Sampler,
    white: TextureArray,
}

impl Renderer {
    /// Set sokol_gfx up on the current GL context with `cfg`'s pools (allocated now, CLAUDE.md 2).
    pub fn new(cfg: &RenderConfig) -> Self {
        let p = &cfg.pools;
        sg::setup(&sg::Desc {
            buffer_pool_size: p.buffers as i32,
            image_pool_size: p.images as i32,
            sampler_pool_size: p.samplers as i32,
            shader_pool_size: p.shaders as i32,
            pipeline_pool_size: p.pipelines as i32,
            view_pool_size: p.views as i32,
            uniform_buffer_size: cfg.uniform_buffer_bytes as i32,
            logger: sg::Logger { func: Some(sokol_log::slog_func), ..Default::default() },
            environment: sg::Environment {
                defaults: sg::EnvironmentDefaults {
                    color_format: COLOR_FORMAT,
                    depth_format: DEPTH_FORMAT,
                    sample_count: 1,
                },
                ..Default::default()
            },
            ..Default::default()
        });
        assert!(sg::isvalid(), "sokol_gfx did not set up on this GL context");
        sg::enable_stats();
        let backend = sg::query_backend();
        let mut shaders = HashMap::new();
        shaders.insert(DeckProgram::Textured, sg::make_shader(&shaders::deck::deck_shader_desc(backend)));
        shaders.insert(DeckProgram::Flat, sg::make_shader(&shaders::deck::deck_flat_shader_desc(backend)));
        let blit_shader = sg::make_shader(&shaders::blit::blit_shader_desc(backend));
        let mut blit = sg::PipelineDesc { shader: blit_shader, label: c"blit".as_ptr(), ..Default::default() };
        blit.depth.pixel_format = DEPTH_FORMAT;
        blit.colors[0].pixel_format = COLOR_FORMAT;
        let blit = sg::make_pipeline(&blit);
        let sampler = |f| {
            sg::make_sampler(&sg::SamplerDesc {
                min_filter: f,
                mag_filter: f,
                wrap_u: sg::Wrap::ClampToEdge,
                wrap_v: sg::Wrap::ClampToEdge,
                ..Default::default()
            })
        };
        let nearest = sampler(sg::Filter::Nearest);
        let linear = sampler(sg::Filter::Linear);
        // Deck textures repeat (world-projected coordinates run across many spans, deck-pipeline 5a),
        // are sampled nearest up close and from their mipmaps at a distance (surface-materials).
        let deck_sampler = sg::make_sampler(&sg::SamplerDesc {
            min_filter: sg::Filter::Linear,
            mag_filter: sg::Filter::Nearest,
            mipmap_filter: sg::Filter::Linear,
            wrap_u: sg::Wrap::Repeat,
            wrap_v: sg::Wrap::Repeat,
            ..Default::default()
        });
        let white = make_texture_array(1, &[255u8; 4]);
        let sky_shader = sg::make_shader(&shaders::sky::sky_shader_desc(backend));
        let screen_shader = sg::make_shader(&shaders::sky::screen_shader_desc(backend));
        Self {
            pipes: HashMap::new(),
            shaders,
            sky_shader,
            screen_shader,
            outside_pipes: HashMap::new(),
            blit,
            nearest,
            linear,
            deck_sampler,
            white,
        }
    }

    /// The sky's pipeline (no depth test or write: it is drawn first) or a screen's (depth tested, both faces).
    fn outside_pipeline(&mut self, screen: bool, samples: i32) -> sg::Pipeline {
        let shader = if screen { self.screen_shader } else { self.sky_shader };
        *self.outside_pipes.entry((screen, samples)).or_insert_with(|| {
            let mut d = sg::PipelineDesc {
                shader,
                cull_mode: sg::CullMode::None,
                sample_count: samples,
                label: if screen { c"screen".as_ptr() } else { c"sky".as_ptr() },
                ..Default::default()
            };
            d.depth.compare = if screen { sg::CompareFunc::LessEqual } else { sg::CompareFunc::Always };
            d.depth.write_enabled = screen;
            d.depth.pixel_format = DEPTH_FORMAT;
            d.colors[0].pixel_format = COLOR_FORMAT;
            sg::make_pipeline(&d)
        })
    }

    /// Draw the sky over the whole of `t`'s current pass (first, before anything else).
    pub fn draw_sky(&mut self, t: &Target, p: &SkyParams) {
        let pip = self.outside_pipeline(false, t.samples);
        // The sky reads no buffer or texture, and sokol_gfx refuses empty bindings: none are applied.
        sg::apply_pipeline(pip);
        let vs = shaders::sky::SkyVsParams { inv_vp: p.inv_vp.to_cols_array() };
        sg::apply_uniforms(shaders::sky::UB_SKY_VS_PARAMS, &sg::value_as_range(&vs));
        let fs = shaders::sky::SkyFsParams {
            sun: p.sun,
            sun_colour: p.sun_colour,
            planet: p.planet,
            ocean: p.ocean,
            land: p.land,
            cloud: p.cloud,
            atmosphere: p.atmosphere,
            stars: p.stars,
        };
        sg::apply_uniforms(shaders::sky::UB_SKY_FS_PARAMS, &sg::value_as_range(&fs));
        sg::draw(0, 3, 1);
    }

    /// Draw a screen in `t`'s current pass: a quad whose corners (-1..1, z 0) `mvp` places, showing `source`'s
    /// picture (rendered earlier this frame, in its own pass). `look`: scan lines' depth, the source's height in
    /// pixels, brightness.
    pub fn draw_screen(&mut self, t: &Target, mvp: glam::Mat4, source: &Target, look: [f32; 4]) {
        let pip = self.outside_pipeline(true, t.samples);
        sg::apply_pipeline(pip);
        let mut b = sg::Bindings::new();
        b.views[shaders::sky::VIEW_TEX] = source.texture;
        b.samplers[shaders::sky::SMP_SMP] = self.linear;
        sg::apply_bindings(&b);
        let vs = shaders::sky::ScreenVsParams { mvp: mvp.to_cols_array() };
        sg::apply_uniforms(shaders::sky::UB_SCREEN_VS_PARAMS, &sg::value_as_range(&vs));
        let fs = shaders::sky::ScreenFsParams { look };
        sg::apply_uniforms(shaders::sky::UB_SCREEN_FS_PARAMS, &sg::value_as_range(&fs));
        sg::draw(0, 6, 1);
    }

    /// The deck pipeline for a program, an index width and a sample count, made once.
    fn deck_pipeline(&mut self, program: DeckProgram, width: IndexWidth, samples: i32) -> sg::Pipeline {
        let shader = self.shaders[&program];
        *self.pipes.entry((program, width, samples)).or_insert_with(|| {
            let mut d = sg::PipelineDesc {
                shader,
                index_type: match width {
                    IndexWidth::U16 => sg::IndexType::Uint16,
                    IndexWidth::U32 => sg::IndexType::Uint32,
                },
                cull_mode: sg::CullMode::Back,
                face_winding: sg::FaceWinding::Ccw,
                sample_count: samples,
                label: c"deck".as_ptr(),
                ..Default::default()
            };
            d.layout.buffers[0].stride = DECK_VERTEX_BYTES as i32;
            for (i, a) in DECK_ATTRIBUTES.iter().enumerate() {
                d.layout.attrs[i].offset = a.offset as i32;
                d.layout.attrs[i].format = vertex_format(a.format);
            }
            d.depth.compare = sg::CompareFunc::LessEqual;
            d.depth.write_enabled = true;
            d.depth.pixel_format = DEPTH_FORMAT;
            d.colors[0].pixel_format = COLOR_FORMAT;
            sg::make_pipeline(&d)
        })
    }

    /// A 3D target of `width` x `height` pixels with `samples` MSAA samples (1: none).
    pub fn make_target(&mut self, width: u32, height: u32, samples: i32) -> Target {
        let image = |usage: sg::ImageUsage, format, samples| {
            sg::make_image(&sg::ImageDesc {
                usage,
                width: width as i32,
                height: height as i32,
                pixel_format: format,
                sample_count: samples,
                ..Default::default()
            })
        };
        let att = |img| {
            sg::make_view(&sg::ViewDesc {
                color_attachment: sg::ImageViewDesc { image: img, ..Default::default() },
                ..Default::default()
            })
        };
        let color_img = image(sg::ImageUsage { color_attachment: true, ..Default::default() }, COLOR_FORMAT, samples);
        let depth_img =
            image(sg::ImageUsage { depth_stencil_attachment: true, ..Default::default() }, DEPTH_FORMAT, samples);
        let depth = sg::make_view(&sg::ViewDesc {
            depth_stencil_attachment: sg::ImageViewDesc { image: depth_img, ..Default::default() },
            ..Default::default()
        });
        let tex = |img| {
            sg::make_view(&sg::ViewDesc {
                texture: sg::TextureViewDesc { image: img, ..Default::default() },
                ..Default::default()
            })
        };
        let (resolve, texture) = if samples > 1 {
            let r = image(sg::ImageUsage { resolve_attachment: true, ..Default::default() }, COLOR_FORMAT, 1);
            let rv = sg::make_view(&sg::ViewDesc {
                resolve_attachment: sg::ImageViewDesc { image: r, ..Default::default() },
                ..Default::default()
            });
            (Some(rv), tex(r))
        } else {
            (None, tex(color_img))
        };
        Target { width, height, samples, color: att(color_img), resolve, depth, texture }
    }

    /// Upload a deck mesh: vertices packed by `sc_core::vertex::pack_deck_vertex`, and indices.
    ///
    /// # Panics
    /// When the vertex bytes are not whole vertices, or 16-bit indices are given for more than
    /// 65,535 vertices.
    pub fn make_mesh(&self, vertices: &[u8], indices: Indices<'_>) -> Mesh {
        assert_eq!(vertices.len() % DECK_VERTEX_BYTES, 0, "vertex bytes are whole 28-byte vertices");
        let vbuf = sg::make_buffer(&sg::BufferDesc {
            usage: sg::BufferUsage { vertex_buffer: true, ..Default::default() },
            data: sg::slice_as_range(vertices),
            ..Default::default()
        });
        let (data, index_count, width) = match indices {
            Indices::U16(i) => {
                assert!(vertices.len() / DECK_VERTEX_BYTES <= 65_536, "16-bit indices reach 65,536 vertices");
                (sg::slice_as_range(i), i.len(), IndexWidth::U16)
            }
            Indices::U32(i) => (sg::slice_as_range(i), i.len(), IndexWidth::U32),
        };
        let ibuf = sg::make_buffer(&sg::BufferDesc {
            usage: sg::BufferUsage { index_buffer: true, ..Default::default() },
            data,
            ..Default::default()
        });
        Mesh { vbuf, ibuf, index_count, width }
    }

    /// A texture array of square RGBA8 layers, `rgba` holding every layer in order.
    pub fn make_texture_array(&self, size_px: u32, rgba: &[u8]) -> TextureArray {
        make_texture_array(size_px, rgba)
    }

    /// A texture array with its mip chain: `levels[k]` holds every layer at `size_px >> k`.
    ///
    /// # Panics
    /// When a level is not whole layers of its size, or there are more than 16 levels.
    pub fn make_texture_array_mips(&self, size_px: u32, layers: u32, levels: &[&[u8]]) -> TextureArray {
        assert!(!levels.is_empty() && levels.len() <= 16, "1 to 16 mip levels");
        let mut data = sg::ImageData::new();
        for (k, l) in levels.iter().enumerate() {
            let px = (size_px >> k).max(1) as usize;
            assert_eq!(l.len(), px * px * 4 * layers as usize, "mip level {k} is whole layers of {px} px");
            data.mip_levels[k] = sg::slice_as_range(l);
        }
        let img = sg::make_image(&sg::ImageDesc {
            _type: sg::ImageType::Array,
            width: size_px as i32,
            height: size_px as i32,
            num_slices: layers as i32,
            num_mipmaps: levels.len() as i32,
            pixel_format: COLOR_FORMAT,
            data,
            ..Default::default()
        });
        TextureArray {
            view: sg::make_view(&sg::ViewDesc {
                texture: sg::TextureViewDesc { image: img, ..Default::default() },
                ..Default::default()
            }),
        }
    }

    /// Begin the 3D pass into `t`, cleared to `clear` (linear RGBA) and depth 1.
    pub fn begin_3d(&mut self, t: &Target, clear: [f32; 4]) {
        let mut action = sg::PassAction::new();
        action.colors[0].load_action = sg::LoadAction::Clear;
        action.colors[0].clear_value = sg::Color { r: clear[0], g: clear[1], b: clear[2], a: clear[3] };
        action.depth.load_action = sg::LoadAction::Clear;
        action.depth.clear_value = 1.0;
        let mut att = sg::Attachments::new();
        att.colors[0] = t.color;
        if let Some(r) = t.resolve {
            att.resolves[0] = r;
        }
        att.depth_stencil = t.depth;
        sg::begin_pass(&sg::Pass { action, attachments: att, label: c"3d".as_ptr(), ..Default::default() });
    }

    /// Draw a deck mesh in the current 3D pass of `t`. `texture` defaults to one white layer.
    pub fn draw_deck(
        &mut self,
        t: &Target,
        mesh: &Mesh,
        program: DeckProgram,
        params: &DeckParams,
        texture: Option<&TextureArray>,
    ) {
        self.draw_deck_range(t, mesh, program, params, texture, 0, mesh.index_count);
    }

    /// Draw `count` indices of a deck mesh from `first` (a draw call of part of a buffer).
    #[allow(clippy::too_many_arguments)]
    pub fn draw_deck_range(
        &mut self,
        t: &Target,
        mesh: &Mesh,
        program: DeckProgram,
        params: &DeckParams,
        texture: Option<&TextureArray>,
        first: usize,
        count: usize,
    ) {
        let pip = self.deck_pipeline(program, mesh.width, t.samples);
        sg::apply_pipeline(pip);
        let mut b = sg::Bindings::new();
        b.vertex_buffers[0] = mesh.vbuf;
        b.index_buffer = mesh.ibuf;
        if program == DeckProgram::Textured {
            b.views[shaders::deck::VIEW_TEX] = texture.unwrap_or(&self.white).view;
            b.samplers[shaders::deck::SMP_SMP] = self.deck_sampler;
        }
        sg::apply_bindings(&b);
        let u = shaders::deck::DeckVsParams {
            mvp: params.mvp.to_cols_array(),
            state_weights: [params.state_weights[0], params.state_weights[1], params.state_weights[2], 0.0],
            flash: [params.flash_dir.x, params.flash_dir.y, params.flash_dir.z, params.flash],
        };
        sg::apply_uniforms(shaders::deck::UB_DECK_VS_PARAMS, &sg::value_as_range(&u));
        if program == DeckProgram::Textured {
            let first_layer = if params.panel_first == u32::MAX { 1.0e9 } else { params.panel_first as f32 };
            let fs = shaders::deck::DeckFsParams { glow: [first_layer, params.panel_glow, 0.0, 0.0] };
            sg::apply_uniforms(shaders::deck::UB_DECK_FS_PARAMS, &sg::value_as_range(&fs));
        }
        sg::draw(first, count, 1);
    }

    /// End the current pass.
    pub fn end_pass(&mut self) {
        sg::end_pass();
    }

    /// Draw `t` over the whole output of `width` x `height` pixels (framebuffer 0), scaled with
    /// linear filtering or nearest.
    pub fn present(&mut self, t: &Target, width: u32, height: u32, linear: bool) {
        let mut action = sg::PassAction::new();
        action.colors[0].load_action = sg::LoadAction::Dontcare;
        action.depth.load_action = sg::LoadAction::Dontcare;
        let swapchain = sg::Swapchain {
            width: width as i32,
            height: height as i32,
            sample_count: 1,
            color_format: COLOR_FORMAT,
            depth_format: DEPTH_FORMAT,
            gl: sg::GlSwapchain { framebuffer: 0 },
            ..Default::default()
        };
        sg::begin_pass(&sg::Pass { action, swapchain, label: c"output".as_ptr(), ..Default::default() });
        sg::apply_pipeline(self.blit);
        let mut b = sg::Bindings::new();
        b.views[shaders::blit::VIEW_TEX] = t.texture;
        b.samplers[shaders::blit::SMP_SMP] = if linear { self.linear } else { self.nearest };
        sg::apply_bindings(&b);
        sg::draw(0, 3, 1);
        sg::end_pass();
    }

    /// Finish the frame.
    pub fn commit(&mut self) {
        sg::commit();
    }

    /// Counts from the frame committed last.
    pub fn last_frame_counts(&self) -> FrameCounts {
        let f = sg::query_stats().prev_frame;
        FrameCounts {
            draws: f.num_draw + f.num_draw_ex,
            pipelines: f.num_apply_pipeline,
            bindings: f.num_apply_bindings,
            uniforms: f.num_apply_uniforms,
            passes: f.num_passes,
        }
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        sg::shutdown();
    }
}

/// A mesh's indices.
pub enum Indices<'a> {
    /// 16-bit.
    U16(&'a [u16]),
    /// 32-bit.
    U32(&'a [u32]),
}

fn vertex_format(f: AttrFormat) -> sg::VertexFormat {
    match f {
        AttrFormat::Short4 => sg::VertexFormat::Short4,
        AttrFormat::Short2 => sg::VertexFormat::Short2,
        AttrFormat::Int10N2 => sg::VertexFormat::Int10N2,
        AttrFormat::UByte4N => sg::VertexFormat::Ubyte4n,
    }
}

fn make_texture_array(size_px: u32, rgba: &[u8]) -> TextureArray {
    let layer = (size_px * size_px * 4) as usize;
    assert!(layer > 0 && rgba.len() % layer == 0, "texture array data is whole {size_px} px RGBA8 layers");
    let mut data = sg::ImageData::new();
    data.mip_levels[0] = sg::slice_as_range(rgba);
    let img = sg::make_image(&sg::ImageDesc {
        _type: sg::ImageType::Array,
        width: size_px as i32,
        height: size_px as i32,
        num_slices: (rgba.len() / layer) as i32,
        pixel_format: COLOR_FORMAT,
        data,
        ..Default::default()
    });
    TextureArray {
        view: sg::make_view(&sg::ViewDesc {
            texture: sg::TextureViewDesc { image: img, ..Default::default() },
            ..Default::default()
        }),
    }
}

extern "C" {
    fn glReadPixels(x: i32, y: i32, width: i32, height: i32, format: u32, kind: u32, pixels: *mut core::ffi::c_void);
    fn glFinish();
}

/// Wait until the GPU has finished every command issued so far. The probe calls it each frame so a
/// frame's time is its whole cost, not only its submission (engine-stack design 11); the game never
/// does, since it would stall the CPU on the GPU.
pub fn finish() {
    // SAFETY: called with the renderer's GL context current; glFinish has no other precondition.
    unsafe { glFinish() };
}

/// Read the output (framebuffer 0) back as RGBA8 rows from the top, for captures and render tests.
/// Never for gameplay (CLAUDE.md: nothing authoritative reads the GPU back).
pub fn read_output(width: u32, height: u32) -> Vec<u8> {
    const GL_RGBA: u32 = 0x1908;
    const GL_UNSIGNED_BYTE: u32 = 0x1401;
    let row = width as usize * 4;
    let mut px = vec![0u8; row * height as usize];
    // SAFETY: the GL context is current on this thread (the Renderer's contract), the buffer holds
    // width x height RGBA8 pixels, and pack alignment 4 matches rows of 4-byte pixels.
    unsafe {
        glFinish();
        glReadPixels(0, 0, width as i32, height as i32, GL_RGBA, GL_UNSIGNED_BYTE, px.as_mut_ptr().cast());
    }
    // GL's rows run bottom up.
    let mut out = vec![0u8; px.len()];
    for y in 0..height as usize {
        let src = (height as usize - 1 - y) * row;
        out[y * row..(y + 1) * row].copy_from_slice(&px[src..src + row]);
    }
    out
}
