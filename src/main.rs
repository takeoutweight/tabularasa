use cosmic_text::rustybuzz::ttf_parser::vorg::VerticalOriginMetrics;
use miniquad::*;
use cosmic_text::{Attrs, AttrsList, Buffer, BufferLine, Color, FontSystem, Metrics, ShapeBuffer, Shaping, SwashCache, CacheKey, SubpixelBin, Wrap};
use texture_packer::packer::{Packer, SkylinePacker};
use texture_packer::rect::Rect;
use texture_packer::frame::Frame;
use texture_packer::{TexturePacker, TexturePackerConfig};
// use texture_packer::importer::
// use image_importer::ImageImporter;
use std::collections::HashMap;
use std::cmp::max;
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

struct Stage {
    ctx: Box<dyn RenderingBackend>,

    pipeline: Pipeline,
    bindings: Bindings,
		window_width: f32,
		window_height: f32,
}

impl Stage {
    pub fn new(window_width: f32, window_height:f32, bitmap: [u8; 400*200*4]) -> Stage {
        let mut ctx: Box<dyn RenderingBackend> = window::new_rendering_backend();

				let bwidth = 200.0;
				let bheight = 200.0;
        #[rustfmt::skip]
        let vertices: [Vertex; 4] = [
            Vertex { pos : Vec2 { x: -0.5*bwidth, y: -0.5*bheight }, uv: Vec2 { x: 0., y: 0. } },
            Vertex { pos : Vec2 { x:  0.5*bwidth, y: -0.5*bheight }, uv: Vec2 { x: 1., y: 0. } },
            Vertex { pos : Vec2 { x:  0.5*bwidth, y:  0.5*bheight }, uv: Vec2 { x: 1., y: 1. } },
            Vertex { pos : Vec2 { x: -0.5*bwidth, y:  0.5*bheight }, uv: Vec2 { x: 0., y: 1. } },
        ];
        let vertex_buffer = ctx.new_buffer(
            BufferType::VertexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(&vertices),
        );

        let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];
        let index_buffer = ctx.new_buffer(
            BufferType::IndexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(&indices),
        );

        let pixels: [u8; 4 * 4 * 4] = [
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00,
            0x00, 0xFF, 0xFF, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0x00, 0x00, 0xFF, 0xFF, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        ];
        // let texture = ctx.new_texture_from_rgba8(4, 4, &pixels);
				let texture = ctx.new_texture_from_rgba8(400, 200, &bitmap);

        let bindings = Bindings {
            vertex_buffers: vec![vertex_buffer],
            index_buffer: index_buffer,
            images: vec![texture],
        };

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

				let params = PipelineParams{
						color_blend: Some(BlendState::new(
            Equation::Add,
            BlendFactor::Value(BlendValue::SourceAlpha),
            BlendFactor::OneMinusValue(BlendValue::SourceAlpha))
        ),
						alpha_blend: Some(BlendState::new(
            Equation::Add,
            BlendFactor::Value(BlendValue::SourceAlpha),
            BlendFactor::OneMinusValue(BlendValue::SourceAlpha)
        )),
				..PipelineParams::default()};

        let pipeline = ctx.new_pipeline_with_params(
            &[BufferLayout::default()],
            &[
                VertexAttribute::new("in_pos", VertexFormat::Float2),
                VertexAttribute::new("in_uv", VertexFormat::Float2),
            ],
            shader,
						params,
        );

        Stage {
            pipeline,
            bindings,
            ctx,
						window_width,
						window_height,
        }
    }
}

impl EventHandler for Stage {
    fn update(&mut self) {}

    fn draw(&mut self) {
        let t = date::now();

        self.ctx.begin_default_pass(Default::default());

        self.ctx.apply_pipeline(&self.pipeline);
        self.ctx.apply_bindings(&self.bindings);
        for i in 0..10 {
            let t = (t as f64)*0.05 + (i as f64)*0.5;

            self.ctx
                .apply_uniforms(UniformsSource::table(&shader::Uniforms {
                    offset: ((t.sin() as f32 * 500.0) + 500.0, ((t * 3.).cos() as f32 * 500.0) + 500.0),
										window_scale: (1.0/self.window_width.max(0.1), -1.0/self.window_height.max(0.1)),
                }));
            self.ctx.draw(0, 6, 1);
        }
        self.ctx.end_render_pass();

        self.ctx.commit_frame();
    }

		fn resize_event(&mut self, w:f32, h:f32) {
				self.window_width = w;
				self.window_height = h;
		}
}

fn draw_rect(arr: &mut [u8; 400*200*4], x:usize, y:usize,w:usize,h:usize, color:Color) {
		for j in 0..h {
				for i in 0..w {
						arr[((y)+j)*400*4 + (x+i)*4 + 0] = color.r();
						arr[((y)+j)*400*4 + (x+i)*4 + 1] = color.g();
						arr[((y)+j)*400*4 + (x+i)*4 + 2] = color.b();
						arr[((y)+j)*400*4 + (x+i)*4 + 3] = color.a();
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
		println!("Conf.width {}x{}",conf.window_width, conf.window_height);

		let mut texture: [u8; 400 * 200 * 4] = [0; 400 * 200 * 4];
		draw_rect(&mut texture,0,0,400,200,Color::rgba(0,0xff,0,30));
		//draw_rect(&mut texture,0,0,200,50,Color(0xffff5000));
		let mut font_system = FontSystem::new();
		let mut swash_cache = SwashCache::new();
		let metrics = Metrics::new(14.0*4.0, 20.0*4.0);
		let mut buffer = Buffer::new(&mut font_system, metrics);
		let attrs = Attrs::new();
		let text_color = Color::rgb(0xFF, 0xFF, 0xFF);
		let width = 80u16;
    let height = 25u16;
    buffer.set_size(&mut font_system, 80.0*4.0, 25.0*4.0);
		buffer.set_text(&mut font_system, " Hi, Rust! 🦀", attrs, Shaping::Advanced);
		buffer.draw(&mut font_system, &mut swash_cache, text_color, |x,y,w,h,color| {
				draw_rect(&mut texture,
   							 usize::try_from(x).unwrap(),
 								 usize::try_from(y).unwrap(),
 								 usize::try_from(w).unwrap(),
 								 usize::try_from(h).unwrap(),color);
 		});

    // Fussing with texture atlases
		let mut shape_buffer = ShapeBuffer::default();
		let mut buffer_line = BufferLine::new("Buffered Line 🐧🐧🐧", AttrsList::new(attrs), Shaping::Advanced);
		// let shape = buffer_line.shape_in_buffer(&mut shape_buffer, &mut font_system);
		let layout_lines = buffer_line.layout_in_buffer(&mut shape_buffer,  &mut font_system, 25.0, 500.0, Wrap::None);
		// let glyph_key = layout_lines[0].glyphs[1].physical((0.0,0.0), 1.0).cache_key;

		let config = TexturePackerConfig {
          max_width: 1028,
          max_height: 40000,
          allow_rotation: false,
          texture_outlines: true,
          border_padding: 2,
          ..Default::default()
        };
		let mut packer = SkylinePacker::new(config);
		let mut glyph_loc: HashMap<CacheKey, Rect> = HashMap::new();

		// generate atlas locations (no raster yet)
		for line in layout_lines {
				for glyph in line.glyphs.iter() {
						let glyph_key = CacheKey {
								x_bin: SubpixelBin::Zero,
								y_bin: SubpixelBin::Zero,
     						..glyph.physical((0.0,0.0), 1.0).cache_key};
						let maybe_img = swash_cache.get_image(&mut font_system, glyph_key);

					  if let Some(img) = maybe_img {
      				let width = img.placement.width;
      				let height = img.placement.height;
      
      				let name = "hi";
      				let frame = packer.pack(name, &Rect::new(0,0,width,height));
      				if let Some(frm) = frame {
									if let Some(rect) = glyph_loc.get(&glyph_key) {
											println!("cached {:?}: {},{}: {}x{}", glyph_key, rect.x
      										 , rect.y, rect.h, rect.h );
									} else {
      						  glyph_loc.insert(glyph_key, frm.frame);
      					  	println!("new    {:?}: {},{}: {}x{}", glyph_key, frm.frame.x
      										 , frm.frame.y, frm.frame.h, frm.frame.h );
									}
      				}
      		}
				}
		}
		let atlas_height = packer.skylines.iter().fold(0, {|h, skyline| max(h, skyline.y)});
		println!("max_height: {}", atlas_height);

		let mut atlas_texture = vec![0x0_u8; 1028 * usize::try_from(atlas_height).unwrap() * 4];
		for (glyph_key, rect) in &glyph_loc {
				let maybe_img = swash_cache.get_image(&mut font_system, *glyph_key);
				if let Some(img)= maybe_img {
						let w = img.placement.width;
						let h = img.placement.height;
						let len = img.data.len();
						match img.content {
								Content::Mask => {
									assert!(usize::try_from(w * h).unwrap() == len,
													"unexpected img size: {} x {} x {:?} vs {}", w, h, img.content, len);
									for y in 0..h {
											for x in 0..w {
													let target = usize::try_from(y*1028*4+x*4).unwrap();
													atlas_texture[target + 0] = 0xff; // r
													atlas_texture[target + 1] = 0xff;
													atlas_texture[target + 2] = 0xff;
													atlas_texture[target + 3] // a
															= img.data[usize::try_from(y*w+x).unwrap()];
											}
							   	}
								},
								Content::Color => {
										assert!(usize::try_from(w * h * 4).unwrap() == len,
													"unexpected img size: {} x {} x {:?} vs {}", w, h, img.content, len);
										for y in 0..h {
												for x in 0..w {
														for c in 0..4 {
																let target = usize::try_from(y*1028*4+x*c).unwrap();
      													let source = usize::try_from(y*w+x*4+c).unwrap();
			      										atlas_texture[target] = img.data[source];
			      										}
											}
							   	}
								},
								x => println!("unknown content {:?}", x)
						}
				}
		}

		// how do I operate over references like this?
		//let img_w = img.clone().expect("no image").placement.width;
		// let line = shape.spans[0].words[0].glyphs[0].physical();
		//swash_cache.get_image(&mut font_system, 
		
		
    let window_width = conf.window_width as f32;
		let window_height = conf.window_height as f32;
    miniquad::start(conf, move || Box::new(Stage::new(window_width, window_height, texture)));
}

mod shader {
    use miniquad::*;

    pub const VERTEX: &str = r#"#version 100
    attribute vec2 in_pos;
    attribute vec2 in_uv;

    uniform vec2 offset;
    uniform vec2 window_scale;

    varying lowp vec2 texcoord;

    void main() {
        gl_Position = vec4(in_pos + offset, 0, 1);
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
                uniforms: vec![UniformDesc::new("offset", UniformType::Float2)],
            },
        }
    }

    #[repr(C)]
    pub struct Uniforms {
        pub offset: (f32, f32),
				pub window_scale:(f32,f32),
    }
}
