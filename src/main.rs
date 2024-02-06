use cosmic_text::rustybuzz::ttf_parser::vorg::VerticalOriginMetrics;
use cosmic_text::{
    Attrs, AttrsList, Buffer, BufferLine, CacheKey, Color, FontSystem, Metrics, ShapeBuffer,
    Shaping, SubpixelBin, SwashCache, Wrap,
};
use miniquad::*;
use texture_packer::packer::{Packer, SkylinePacker};
use texture_packer::rect::Rect;
use texture_packer::TexturePackerConfig;
// use texture_packer::importer::
// use image_importer::ImageImporter;
use std::cmp::max;
use std::collections::HashMap;
use swash::scale::image::Content;

#[repr(C)]
struct Vec2 {
    x: f32,
    y: f32,
}
#[repr(C)]
struct Vertex {
    pos: Vec2,
    uv: Vec2,
}

enum Object {
    Node,
    TextLine,
    Rectangle,
}

struct Node {
    pos: Vec2,
    children: Vec<Object>,
}

struct TextLine {
    bindings: Bindings,
    index_count: i32,
}

impl TextLine {
    pub fn new<T: Into<String>>(text: T, ctx: &mut Box<dyn RenderingBackend>) -> TextLine {
        let bl_attrs = Attrs::new();
        let mut buffer_line = BufferLine::new(text, AttrsList::new(bl_attrs), Shaping::Advanced);

        let config = TexturePackerConfig {
            max_width: ATLAS_WIDTH,
            max_height: 40000,
            allow_rotation: false,
            texture_outlines: true,
            border_padding: 2,
            ..Default::default()
        };
        let mut packer = SkylinePacker::new(config);
        let mut glyph_loc: HashMap<CacheKey, (Rect, i32, i32)> = HashMap::new();
        let mut font_system = FontSystem::new();
        let mut swash_cache = SwashCache::new();
        let bl_font_size = 72.0;
        let mut shape_buffer = ShapeBuffer::default();

        let layout_lines = buffer_line.layout_in_buffer(
            &mut shape_buffer,
            &mut font_system,
            bl_font_size,
            500.0,
            Wrap::None,
        );

        // generate atlas locations (no raster yet)
        for line in layout_lines {
            for glyph in line.glyphs.iter() {
                let glyph_key = CacheKey {
                    x_bin: SubpixelBin::Zero,
                    y_bin: SubpixelBin::Zero,
                    ..glyph.physical((0.0, 0.0), 1.0).cache_key
                };
                if let Some((rect, _left, _top)) = glyph_loc.get(&glyph_key) {
                    println!(
                        "cached: {:?}: {},{}: {}x{}",
                        glyph_key, rect.x, rect.y, rect.w, rect.h
                    );
                } else {
                    let maybe_img = swash_cache.get_image(&mut font_system, glyph_key);

                    if let Some(img) = maybe_img {
                        let width = img.placement.width;
                        let height = img.placement.height;

                        let name = "hi";
                        let frame = packer.pack(name, &Rect::new(0, 0, width, height));
                        if let Some(frm) = frame {
                            glyph_loc.insert(
                                glyph_key,
                                (frm.frame, img.placement.left, img.placement.top),
                            );
                            println!(
                                "new:    {:?}: {},{}: {}x{}",
                                glyph_key, frm.frame.x, frm.frame.y, frm.frame.w, frm.frame.h
                            );
                        }
                    }
                }
            }
        }
        let atlas_height = packer
            .skylines
            .iter()
            .fold(0, |h, skyline| max(h, skyline.y));
        println!("max_height: {}", atlas_height);

        let mut atlas_texture =
            vec![0x88_u8; usize::try_from(ATLAS_WIDTH * atlas_height).unwrap() * 4];
        for (glyph_key, (rect, left, top)) in &glyph_loc {
            let maybe_img = swash_cache.get_image(&mut font_system, *glyph_key);
            if let Some(img) = maybe_img {
                println!["img: {:?}", img.placement];
                let w = img.placement.width;
                let h = img.placement.height;
                let len = img.data.len();
                match img.content {
                    Content::Mask => {
                        assert!(
                            usize::try_from(w * h).unwrap() == len,
                            "unexpected img size: {} x {} x {:?} vs {}",
                            w,
                            h,
                            img.content,
                            len
                        );
                        println!("drawing {:?}", (rect, w, h));
                        for y in 0..h {
                            for x in 0..w {
                                let target = usize::try_from(
                                    (rect.y + y) * ATLAS_WIDTH * 4 + ((rect.x + x) * 4),
                                )
                                .unwrap();
                                atlas_texture[target + 0] = 0xff; // r
                                atlas_texture[target + 1] = 0xff;
                                atlas_texture[target + 2] = 0xff;
                                atlas_texture[target + 3] // a
															= img.data[usize::try_from(y*w+x).unwrap()];
                            }
                        }
                    }
                    Content::Color => {
                        assert!(
                            usize::try_from(w * h * 4).unwrap() == len,
                            "unexpected img size: {} x {} x {:?} vs {}",
                            w,
                            h,
                            img.content,
                            len
                        );
                        for y in 0..h {
                            for x in 0..w {
                                for c in 0..4 {
                                    let target = usize::try_from(
                                        (rect.y + y) * ATLAS_WIDTH * 4 + (rect.x + x) * 4 + c,
                                    )
                                    .unwrap();
                                    let source = usize::try_from(y * w * 4 + x * 4 + c).unwrap();
                                    atlas_texture[target] = img.data[source];
                                }
                            }
                        }
                    }
                    x => println!("unknown content {:?}", x),
                }
            }
        }

        let a_height_u =
            u16::try_from(u32::try_from(atlas_texture.len()).unwrap() / (ATLAS_WIDTH * 4)).unwrap();
        let a_w = ATLAS_WIDTH as f32;
        let a_h = a_height_u as f32;
        // showing the text using the atlas:
        let mut vertices: Vec<Vertex> = Vec::new();
        let mut indices: Vec<u16> = Vec::new();

        if let Some(lines) = buffer_line.layout_opt() {
            for line in lines {
                for glyph in line.glyphs.iter() {
                    // todo abstract
                    let real_key = glyph.physical((0.0, 0.0), 1.0).cache_key;
                    let glyph_key = CacheKey {
                        x_bin: SubpixelBin::Zero,
                        y_bin: SubpixelBin::Zero,
                        ..real_key
                    };
                    // This is using the atlas for width, but if I scale it that won't always be true.
                    // just because there's no "height" for glyphs and I'm not sure why.
                    if let Some((rect, left, top)) = glyph_loc.get(&glyph_key) {
                        let pre_length = vertices.len() as u16;
                        // just taking stabs in the dark. This is clearly not right.
                        //  - (real_key.x_bin.as_float() * -1.0)
                        let vx = glyph.x + (*left as f32); //glyph.physical((0.0, 0.0), 1.0).x as f32;
                        let vy = (glyph.y as f32) + (rect.h as f32) - (*top as f32); // glyph.physical((0.0, 0.0), 1.0).y as f32;
                        let vw = rect.w as f32; // glyph.w; //using rect.w makes the characters look right but spaced wrong.
                        let vh = rect.h as f32;
                        let tx = (rect.x as f32) / a_w;
                        let ty = rect.y as f32 / a_h;
                        let tw = rect.w as f32 / a_w;
                        let th = rect.h as f32 / a_h;
                        vertices.push(Vertex {
                            pos: Vec2 { x: vx, y: vy - vh },
                            uv: Vec2 { x: tx, y: ty },
                        });
                        vertices.push(Vertex {
                            pos: Vec2 {
                                x: vx + vw,
                                y: vy - vh,
                            },
                            uv: Vec2 { x: tx + tw, y: ty },
                        });
                        vertices.push(Vertex {
                            pos: Vec2 { x: vx + vw, y: vy },
                            uv: Vec2 {
                                x: tx + tw,
                                y: ty + th,
                            },
                        });
                        vertices.push(Vertex {
                            pos: Vec2 { x: vx, y: vy },
                            uv: Vec2 { x: tx, y: ty + th },
                        });

                        [0, 1, 2, 0, 2, 3].map(|i| indices.push(pre_length + i));
                        // println!("Adding quad: {:?}", (vx, vy, vw, vh, tx, real_key.x_bin.as_float(), ty, tw, th));
                        println!("adding quad: {:?}", glyph);
                    }
                }
            }
        }

        let texture = ctx.new_texture_from_rgba8(
            u16::try_from(ATLAS_WIDTH).unwrap(),
            a_height_u,
            &atlas_texture,
        );

        let vertex_buffer = ctx.new_buffer(
            BufferType::VertexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(&vertices),
        );

        // for one quad
        //let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];
        let index_count = i32::try_from(indices.len()).unwrap();
        let index_buffer = ctx.new_buffer(
            BufferType::IndexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(&indices),
        );

        let bindings = Bindings {
            vertex_buffers: vec![vertex_buffer],
            index_buffer,
            images: vec![texture],
        };
        TextLine {
            bindings,
            index_count,
        }
    }
}

struct Rectangle {
    dim: Vec2,
}

struct Stage {
    ctx: Box<dyn RenderingBackend>,

    pipeline: Pipeline,
    window_width: f32,
    window_height: f32,
    draws_remaining: i32,
    text_line: TextLine,
}

// in texels I.e. not bit array u8 length.
const ATLAS_WIDTH: u32 = 100;

impl Stage {
    pub fn new(window_width: f32, window_height: f32) -> Stage {
        let mut ctx: Box<dyn RenderingBackend> = window::new_rendering_backend();

        let shader = ctx
            .new_shader(
                match ctx.info().backend {
                    Backend::OpenGl => ShaderSource::Glsl {
                        vertex: shader::VERTEX,
                        fragment: shader::FRAGMENT,
                    },
                    Backend::Metal => ShaderSource::Msl {
                        program: shader::METAL,
                    },
                },
                shader::meta(),
            )
            .unwrap();

        if ctx.info().backend == Backend::Metal {
            println!("Backend is metal");
        }
        if ctx.info().backend == Backend::OpenGl {
            println!("Backend is opengl");
        }

        let params = PipelineParams {
            color_blend: Some(BlendState::new(
                Equation::Add,
                BlendFactor::Value(BlendValue::SourceAlpha),
                BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
            )),
            alpha_blend: Some(BlendState::new(
                Equation::Add,
                BlendFactor::Value(BlendValue::SourceAlpha),
                BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
            )),
            ..PipelineParams::default()
        };

        let pipeline = ctx.new_pipeline_with_params(
            &[BufferLayout::default()],
            &[
                VertexAttribute::new("in_pos", VertexFormat::Float2),
                VertexAttribute::new("in_uv", VertexFormat::Float2),
            ],
            shader,
            params,
        );

        let draws_remaining = 600;

        let text_line = TextLine::new(
            "my go Buffered Robin Nola Alden Line 🐧🐧🐧 Why is this so nice?",
            &mut ctx,
        );

        Stage {
            ctx,
            pipeline,
            window_width,
            window_height,
            draws_remaining,
            text_line,
        }
    }
}

impl EventHandler for Stage {
    fn update(&mut self) {}

    fn draw(&mut self) {
        if (self.draws_remaining > 0) {
            // self.draws_remaining -= 1;

            let t = date::now();

            self.ctx.begin_default_pass(Default::default());

            self.ctx.apply_pipeline(&self.pipeline);
            self.ctx.apply_bindings(&self.text_line.bindings);
            for i in 0..10 {
                let t = (t as f64) * 0.05 + (i as f64) * 0.5;

                self.ctx
                    .apply_uniforms(UniformsSource::table(&shader::Uniforms {
                        offset: (
                            (t.sin() as f32 * 500.0) + 500.0,
                            ((t * 3.).cos() as f32 * 500.0) + 500.0,
                        ),
                        window_scale: (
                            1.0 / self.window_width.max(0.1),
                            -1.0 / self.window_height.max(0.1),
                        ),
                    }));
                self.ctx.draw(0, self.text_line.index_count, 1);
            }
            self.ctx.end_render_pass();

            self.ctx.commit_frame();
        }
    }

    fn resize_event(&mut self, w: f32, h: f32) {
        self.window_width = w;
        self.window_height = h;
    }
}

fn draw_rect(arr: &mut [u8; 400 * 200 * 4], x: usize, y: usize, w: usize, h: usize, color: Color) {
    for j in 0..h {
        for i in 0..w {
            arr[((y) + j) * 400 * 4 + (x + i) * 4 + 0] = color.r();
            arr[((y) + j) * 400 * 4 + (x + i) * 4 + 1] = color.g();
            arr[((y) + j) * 400 * 4 + (x + i) * 4 + 2] = color.b();
            arr[((y) + j) * 400 * 4 + (x + i) * 4 + 3] = color.a();
        }
    }
}

fn main() {
    let mut conf = conf::Conf::default();
    let metal = std::env::args().nth(1).as_deref() == Some("metal");
    conf.platform.apple_gfx_api = if metal {
        conf::AppleGfxApi::Metal
    } else {
        conf::AppleGfxApi::OpenGl
    };
    conf.high_dpi = true;
    println!("Conf.width {}x{}", conf.window_width, conf.window_height);

    let window_width = conf.window_width as f32 * 2.0; // not sure we can get dpi_scale before starting
    let window_height = conf.window_height as f32 * 2.0;
    miniquad::start(conf, move || {
        Box::new(Stage::new(window_width, window_height))
    });
}

mod shader {
    use miniquad::*;

    pub const VERTEX: &str = r#"#version 100
    attribute vec2 in_pos;
    attribute vec2 in_uv;

    uniform vec2 offset;
    uniform vec2 window_scale;

    varying lowp vec2 texcoord;

    // precision highp float;

    void main() {
        gl_Position = vec4((window_scale * (in_pos.xy + offset))+vec2(-1,1), 0.0, 1.0);
        texcoord = in_uv;
    }"#;

    pub const FRAGMENT: &str = r#"#version 100
    varying lowp vec2 texcoord;

    uniform sampler2D tex;

    precision highp float;

    void main() {
        vec4 texColor = texture2D(tex, texcoord);
//        if(texColor.a < 0.1)
//          discard;
        gl_FragColor = texColor;
    }"#;

    pub const METAL: &str = r#"
    #include <metal_stdlib>

    using namespace metal;

    struct Uniforms
    {
        float2 offset;
        float2 window_scale;
    };

    struct Vertex
    {
        float2 in_pos   [[attribute(0)]];
        float2 in_uv    [[attribute(1)]];
    };

    struct RasterizerData
    {
        float4 position [[position]];
        float2 uv       [[user(locn0)]];
    };

    vertex RasterizerData vertexShader(
      Vertex v [[stage_in]], 
      constant Uniforms& uniforms [[buffer(0)]])
    {
        RasterizerData out;

        out.position = float4((uniforms.window_scale * (v.in_pos.xy + uniforms.offset)) + float2(-1.0,1.0), 0.0, 1.0);
        out.uv = v.in_uv;

        return out;
    }

    fragment float4 fragmentShader(RasterizerData in [[stage_in]], texture2d<float> tex [[texture(0)]], sampler texSmplr [[sampler(0)]])
    {
        return tex.sample(texSmplr, in.uv);
    }"#;

    pub fn meta() -> ShaderMeta {
        ShaderMeta {
            images: vec!["tex".to_string()],
            uniforms: UniformBlockLayout {
                uniforms: vec![
                    UniformDesc::new("offset", UniformType::Float2),
                    UniformDesc::new("window_scale", UniformType::Float2),
                ],
            },
        }
    }

    #[repr(C)]
    pub struct Uniforms {
        pub offset: (f32, f32),
        pub window_scale: (f32, f32),
    }
}
