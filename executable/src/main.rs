use cosmic_text::{
    Attrs, AttrsList, BufferLine, CacheKey, Color, FontSystem, LayoutGlyph, ShapeBuffer, Shaping,
    SubpixelBin, SwashCache, Wrap,
};
use fontdb::Family;
use miniquad::*;
use texture_packer::packer::{Packer, SkylinePacker};
use texture_packer::rect::Rect;
use texture_packer::TexturePackerConfig;
// use texture_packer::importer::
// use image_importer::ImageImporter;
use lean_experiments::gui_api::{send_event_to_lean, AppendMode, Interpreter};
use std::cmp::{max, min};
use std::collections::{BTreeMap, HashMap};
use swash::scale::image::Content;

mod lean_experiments;
mod shader;

#[repr(C)]
#[derive(Copy, Clone)]
struct Vec2 {
    x: f32,
    y: f32,
}
#[repr(C)]
struct Vertex {
    pos: Vec2,
    uv: Vec2,
}

struct Animating {
    prev_pos: Vec2,
    duration: f32,
    start_time: f64,
}

#[derive(Copy, Clone)]
struct Clip {
    pos: Vec2,
    size: Vec2,
}

struct Column {
    pos: Vec2,
    animation: Option<Animating>,
    clip: Option<Clip>,
    offset: usize,
    length: usize,
}

struct Atlas {
    id: TextureId,
    width: f32,
    height: f32,
}

// combine with TextComponent?
struct TextData {
    laid_out_lines: Vec<BufferLine>,
    unbound_laid_out_offset: usize, // todo make a range list
    unbound_laid_out_length: usize,
    bound_lines: Vec<TextLine>,
    columns: Vec<Column>,
}

struct TextComponent {
    texture_atlas: Option<Atlas>,
    glyph_loc: HashMap<CacheKey, (Rect, i32, i32)>,
    font_system: FontSystem,
    swash_cache: SwashCache,
    shape_buffer: ShapeBuffer,
}

impl TextComponent {
    pub fn new() -> TextComponent {
        let texture_atlas = None;
        let glyph_loc: HashMap<CacheKey, (Rect, i32, i32)> = HashMap::new();
        let font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let shape_buffer = ShapeBuffer::default();
        TextComponent {
            texture_atlas,
            glyph_loc,
            font_system,
            swash_cache,
            shape_buffer,
        }
    }
}

struct TextLine {
    bindings: Bindings,
    index_count: i32,
}

fn layout<T: Into<String>>(text: T, text_component: &mut TextComponent) -> BufferLine {
    let bl_attrs = Attrs::new().family(Family::Name("Menlo"));
    let mut buffer_line = BufferLine::new(text, AttrsList::new(bl_attrs), Shaping::Advanced);
    let bl_font_size = 82.0;

    buffer_line.layout_in_buffer(
        &mut text_component.shape_buffer,
        &mut text_component.font_system,
        bl_font_size,
        500.0,
        Wrap::None,
    );
    return buffer_line;
}

fn glyphs(buffer_line: &BufferLine) -> impl Iterator<Item = &LayoutGlyph> {
    buffer_line
        .layout_opt()
        .iter()
        .flat_map(|lines| lines.iter().flat_map(|line| line.glyphs.iter()))
}

impl TextLine {
    pub fn new(
        atlas_id: TextureId,
        atlas_w: f32,
        atlas_h: f32,
        buffer_line: &BufferLine,
        ctx: &mut Box<dyn RenderingBackend>,
        text_component: &mut TextComponent,
    ) -> TextLine {
        // showing the text using the atlas:
        let mut vertices: Vec<Vertex> = Vec::new();
        let mut indices: Vec<u16> = Vec::new();

        for glyph in glyphs(buffer_line) {
            // todo abstract
            let real_key = glyph.physical((0.0, 0.0), 1.0).cache_key;
            let glyph_key = CacheKey {
                x_bin: SubpixelBin::Zero,
                y_bin: SubpixelBin::Zero,
                ..real_key
            };
            // This is using the atlas for width, but if I scale it that won't always be true.
            // just because there's no "height" for glyphs and I'm not sure why.
            if let Some((rect, left, top)) = text_component.glyph_loc.get(&glyph_key) {
                let pre_length = vertices.len() as u16;
                // just taking stabs in the dark. This is clearly not right.
                //  - (real_key.x_bin.as_float() * -1.0)
                let vx = glyph.x + (*left as f32); //glyph.physical((0.0, 0.0), 1.0).x as f32;
                let vy = (glyph.y as f32) + (rect.h as f32) - (*top as f32); // glyph.physical((0.0, 0.0), 1.0).y as f32;
                                                                             // Not sure why I need to add 2.5 width, the eg underscores don't line up quite right w/o it.
                                                                             // could be drawing the textures a little off and this is just compensating.
                                                                             // Also it's definitely not right, eg it will double width of a narrow character etc
                                                                             // 1.045 leaves tiny gaps in underscores at pt 82
                let vw = rect.w as f32 * 1.05; // glyph.w; //using rect.w makes the characters look right but spaced wrong.d
                let vh = rect.h as f32 * 1.05;
                let tx = (rect.x as f32 + 0.5) / atlas_w;
                let ty = (rect.y as f32 + 0.5) / atlas_h;
                let tw = (rect.w as f32 - 1.0 + 0.5) / atlas_w;
                let th = (rect.h as f32 - 1.0 + 0.5) / atlas_h;
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
                // println!("adding quad: {:?}", glyph);
            } else {
                // Can maybe tupule the atlas info with the glyphs
                panic!("atlas does not have expected glyph");
            }
        }

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
            images: vec![atlas_id],
        };
        TextLine {
            bindings,
            index_count,
        }
    }
}

impl Stage {
    pub fn invalidate_atlas(&mut self) {
        self.text_component.glyph_loc.clear();
        if let Some(texture) = &self.text_component.texture_atlas {
            self.ctx.delete_texture(texture.id)
        }
        self.text_component.texture_atlas = None;
    }
    pub fn regenerate_atlas(&mut self) {
        println!("Regenerating atlas");
        let config = TexturePackerConfig {
            max_width: ATLAS_WIDTH,
            max_height: 40000,
            allow_rotation: false,
            texture_outlines: true,
            border_padding: 2,
            ..Default::default()
        };
        let mut packer = SkylinePacker::new(config);

        for glyph in self.text_data.laid_out_lines.iter().flat_map(glyphs) {
            let glyph_key = CacheKey {
                x_bin: SubpixelBin::Zero,
                y_bin: SubpixelBin::Zero,
                ..glyph.physical((0.0, 0.0), 1.0).cache_key
            };
            if let Some((_rect, _left, _top)) = self.text_component.glyph_loc.get(&glyph_key) {
                /*
                println!(
                    "cached: {:?}: {},{}: {}x{}",
                    glyph_key, rect.x, rect.y, rect.w, rect.h
                );*/
            } else {
                let maybe_img = self
                    .text_component
                    .swash_cache
                    .get_image(&mut self.text_component.font_system, glyph_key);

                if let Some(img) = maybe_img {
                    let width = img.placement.width;
                    let height = img.placement.height;

                    let name = "hi";
                    let frame = packer.pack(name, &Rect::new(0, 0, width, height));
                    if let Some(frm) = frame {
                        self.text_component.glyph_loc.insert(
                            glyph_key,
                            (frm.frame, img.placement.left, img.placement.top),
                        );
                        /*
                        println!(
                            "new:    {:?}: {},{}: {}x{}",
                            glyph_key, frm.frame.x, frm.frame.y, frm.frame.w, frm.frame.h
                        );*/
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
        for (glyph_key, (rect, _left, _top)) in &self.text_component.glyph_loc {
            let maybe_img = self
                .text_component
                .swash_cache
                .get_image(&mut self.text_component.font_system, *glyph_key);
            if let Some(img) = maybe_img {
                // println!["img: {:?}", img.placement];
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
                        // println!("drawing {:?}", (rect, w, h));
                        for y in 0..h {
                            for x in 0..w {
                                let target = usize::try_from(
                                    (rect.y + y) * ATLAS_WIDTH * 4 + ((rect.x + x) * 4),
                                )
                                .unwrap();
                                atlas_texture[target + 0] = TEXT_R; // r
                                atlas_texture[target + 1] = TEXT_G;
                                atlas_texture[target + 2] = TEXT_B;
                                // a
                                atlas_texture[target + 3] =
                                    img.data[usize::try_from(y * w + x).unwrap()];
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

        let texture = self.ctx.new_texture_from_rgba8(
            u16::try_from(ATLAS_WIDTH).unwrap(),
            a_height_u,
            &atlas_texture,
        );

        let a_w = ATLAS_WIDTH as f32;
        let a_h = a_height_u as f32;

        let atlas = Atlas {
            id: texture,
            width: a_w,
            height: a_h,
        };

        assert!(
            match self.text_component.texture_atlas {
                None => true,
                _ => false,
            },
            "Atlas regenerated when still valid"
        );
        self.text_component.texture_atlas = Some(atlas);
        self.text_data.bound_lines.clear();
        self.text_data
            .bound_lines
            .extend(self.text_data.laid_out_lines.iter().map(|buffer_line| {
                TextLine::new(
                    texture,
                    a_w,
                    a_h,
                    buffer_line,
                    &mut self.ctx,
                    &mut self.text_component,
                )
            }));
    }

    pub fn bind_text(&mut self) {
        let offset = self.text_data.unbound_laid_out_offset;
        let length = self.text_data.unbound_laid_out_length;
        let incomplete_atlas = self.text_data.laid_out_lines[offset..offset + length]
            .iter()
            .flat_map(glyphs)
            .any(|glyph| {
                let glyph_key = CacheKey {
                    x_bin: SubpixelBin::Zero,
                    y_bin: SubpixelBin::Zero,
                    ..glyph.physical((0.0, 0.0), 1.0).cache_key
                };
                !self.text_component.glyph_loc.contains_key(&glyph_key)
            });

        if incomplete_atlas {
            self.regenerate_atlas();
        } else {
            if let Some(atlas) = &self.text_component.texture_atlas {
                let a_id = atlas.id;
                let a_w = atlas.width;
                let a_h = atlas.height;
                self.text_data.bound_lines.splice(
                    offset..min(offset + length, self.text_data.bound_lines.len()),
                    self.text_data.laid_out_lines[offset..offset + length]
                        .iter()
                        .map(|buffer_line| {
                            TextLine::new(
                                a_id,
                                a_w,
                                a_h,
                                buffer_line,
                                &mut self.ctx,
                                &mut self.text_component,
                            )
                        }),
                );
            }
        };
        self.text_data.unbound_laid_out_offset = self.text_data.laid_out_lines.len();
        self.text_data.unbound_laid_out_length = 0;
    }
}

// not a str because the bufferline owns them. maybe better to copy inside somewhere?
fn insert_text(
    text_data: &mut TextData,
    text_component: &mut TextComponent,
    pos: Vec2,
    clip: Option<Clip>,
    texts: &'_ [String],
) -> usize {
    let new_offset = text_data.laid_out_lines.len();
    let mut new_size = 0;

    {
        let mut cur_offset = new_offset;
        text_data.columns.push(Column {
            pos,
            animation: None,
            clip,
            length: texts.len(),
            offset: cur_offset,
        });
        cur_offset += texts.len();
        new_size += texts.len();
        text_data
            .laid_out_lines
            .extend(texts.iter().map(|text| layout(text, text_component)));
    }
    let unbound_offset = text_data.unbound_laid_out_offset.min(new_offset);
    text_data.unbound_laid_out_offset = unbound_offset;
    text_data.unbound_laid_out_length = new_offset + new_size - unbound_offset;
    let col_id = text_data.columns.len() - 1;
    col_id
}

fn replace_text(
    text_data: &mut TextData,
    text_component: &mut TextComponent,
    col_id: usize,
    texts: &'_ [String],
) {
    let col = &text_data.columns[col_id];
    assert!(col.length == texts.len());
    text_data.laid_out_lines.splice(
        col.offset..col.offset + col.length,
        texts.iter().map(|text| layout(text, text_component)),
    );
    let unbound_offset = text_data.unbound_laid_out_offset.min(col.offset);
    let unbound_length = max(
        col.offset + col.length,
        text_data.unbound_laid_out_offset + text_data.unbound_laid_out_length,
    ) - unbound_offset;
    text_data.unbound_laid_out_offset = unbound_offset;
    text_data.unbound_laid_out_length = unbound_length;
}

struct Stage {
    ctx: Box<dyn RenderingBackend>,
    pipeline: Pipeline,
    window_width: f32,
    window_height: f32,
    draws_remaining: i32,
    interp: Interpreter,
    text_component: TextComponent,
    text_data: TextData,
}

// in texels I.e. not bit array u8 length.
const ATLAS_WIDTH: u32 = 100;
const BACKGROUND_COLOR: (f32, f32, f32, f32) = (1.0, 0.9, 0.9, 1.0);
const TEXT_R: u8 = 0x10;
const TEXT_G: u8 = 0x10;
const TEXT_B: u8 = 0x30;

impl Stage {
    pub fn new(window_width: f32, window_height: f32, interp: Interpreter) -> Stage {
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

        let pipeline = ctx.new_pipeline(
            &[BufferLayout::default()],
            &[
                VertexAttribute::new("in_pos", VertexFormat::Float2),
                VertexAttribute::new("in_uv", VertexFormat::Float2),
            ],
            shader,
            params,
        );

        let draws_remaining = 600;

        let text_component = TextComponent::new();

        let unbound_laid_out_offset = 0;
        let unbound_laid_out_length = 0;
        let laid_out_lines = Vec::new();
        let bound_lines = Vec::new();
        let columns = Vec::new();
        let text_data = TextData {
            unbound_laid_out_offset,
            unbound_laid_out_length,
            laid_out_lines,
            bound_lines,
            columns,
        };

        Stage {
            ctx,
            pipeline,
            window_width,
            window_height,
            draws_remaining,
            interp,
            text_component,
            text_data,
        }
    }
}

const LINE_HEIGHT: f32 = 80.0;

impl Column {
    fn cur_pos(&self, at_time: f64) -> Vec2 {
        match &self.animation {
            Some(anim) => {
                if at_time <= anim.start_time {
                    anim.prev_pos
                } else if at_time >= anim.start_time + anim.duration as f64 {
                    self.pos
                } else {
                    let t = (at_time - anim.start_time) as f32 / anim.duration;
                    let x = (1.0 - t) * anim.prev_pos.x + t * self.pos.x;
                    let y = (1.0 - t) * anim.prev_pos.y + t * self.pos.y;
                    Vec2 { x, y }
                }
            }
            None => self.pos,
        }
    }
}

fn draw_column(
    text_data: &TextData,
    window_width: f32,
    window_height: f32,
    ctx: &mut Box<dyn RenderingBackend>,
    column: &Column,
    at_time: f64,
) {
    match column.clip {
        Some(clip) => {
            ctx.apply_scissor_rect(
                // This is in "real" pixels i.e. not the halved coarse pixels as reported by eg screen shot tool
                clip.pos.x as i32,
                (window_height - (clip.pos.y + clip.size.y)) as i32,
                clip.size.x as i32,
                clip.size.y as i32,
            );
        }
        None => {}
    }
    let pos = column.cur_pos(at_time);
    let mut cur_y = pos.y;
    for text_line in text_data.bound_lines[column.offset..column.offset + column.length].iter() {
        ctx.apply_bindings(&text_line.bindings);
        ctx.apply_uniforms(UniformsSource::table(&shader::Uniforms {
            offset: (pos.x, cur_y),
            window_scale: (2.0 / window_width.max(0.1), -2.0 / window_height.max(0.1)),
        }));
        ctx.draw(0, text_line.index_count, 1);
        cur_y += LINE_HEIGHT;
    }
    match column.clip {
        Some(_clip) => {
            ctx.apply_scissor_rect(0, 0, window_width as i32, window_height as i32);
        }
        None => {}
    }
}

impl EventHandler for Stage {
    fn update(&mut self) {}

    fn draw(&mut self) {
        if self.draws_remaining <= 0 {
            return;
        }
        // self.draws_remaining -= 1;

        self.bind_text();

        let t = date::now();

        self.ctx.begin_default_pass(Default::default());

        self.ctx.clear(Some(BACKGROUND_COLOR), None, None);

        self.ctx.apply_pipeline(&self.pipeline);
        for column in self.text_data.columns.iter() {
            draw_column(
                &self.text_data,
                self.window_width,
                self.window_height,
                &mut self.ctx,
                column,
                t,
            );
        }
        self.ctx.end_render_pass();

        self.ctx.commit_frame();
    }

    fn resize_event(&mut self, w: f32, h: f32) {
        self.window_width = w;
        self.window_height = h;
    }

    fn char_event(&mut self, character: char, _keymods: KeyMods, _repeat: bool) {
        send_event_to_lean(&mut self.interp, 1, character as u32);
        perform_effects(self);
    }

    fn key_down_event(&mut self, keycode: KeyCode, _keymods: KeyMods, _repeat: bool) {
        match keycode {
            KeyCode::Up => {
                let t = date::now();
                if self.text_data.columns.len() > 0 {
                    self.text_data.columns[0].animation = Some(Animating {
                        prev_pos: self.text_data.columns[0].cur_pos(t),
                        duration: 0.1,
                        start_time: t,
                    });
                    self.text_data.columns[0].pos.y -= 50.0;
                }
            }
            KeyCode::Down => {
                let t = date::now();
                if self.text_data.columns.len() > 0 {
                    self.text_data.columns[0].animation = Some(Animating {
                        prev_pos: self.text_data.columns[0].cur_pos(t),
                        duration: 1.0,
                        start_time: t,
                    });
                    self.text_data.columns[0].pos.y += 50.0;
                }
            }
            KeyCode::Q => {
                window::quit();
            }
            _ => {}
        }
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

fn perform_effects(stage: &mut Stage) {
    //let interp = &mut stage.interp;
    for (id, pos) in stage.interp.effects.new_columns.iter() {
        let sid = match stage.interp.effects.text.get(id) {
            None => insert_text(
                &mut stage.text_data,
                &mut stage.text_component,
                Vec2 { x: pos.x, y: pos.y },
                None,
                &vec![],
            ),
            Some((_app, lines)) => insert_text(
                &mut stage.text_data,
                &mut stage.text_component,
                Vec2 { x: pos.x, y: pos.y },
                None,
                lines,
            ),
        };

        assert!(sid == *id as usize, "{},{}", sid, id);
    }
    for (id, (app, lines)) in stage.interp.effects.text.iter() {
        let nc = stage.interp.effects.new_columns.get(id);

        // i.e. we're adjusting text on a column that wasn't introduced this event
        if nc.is_none() {
            // don't support appending pre-existing text yet
            assert!(matches!(app, AppendMode::Replace));
            // don't support changing length yet
            replace_text(
                &mut stage.text_data,
                &mut stage.text_component,
                *id as usize,
                lines,
            );
        }
    }
    stage.interp.effects.new_columns = BTreeMap::new();
    stage.interp.effects.text = HashMap::new();
    stage.interp.effects.clip = HashMap::new();
    stage.interp.effects.animate = HashMap::new();
    stage.interp.effects.should_quit = false;
}

fn main() {
    let mut interp = lean_experiments::test_lean();
    send_event_to_lean(&mut interp, 1, 12);

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
        Box::new({
            let mut stage = Stage::new(window_width, window_height, interp);
            insert_text(
                &mut stage.text_data,
                &mut stage.text_component,
                Vec2 { x: 100.0, y: 100.0 },
                Some(Clip {
                    pos: Vec2 { x: 108.0, y: 100.0 },
                    size: Vec2 { x: 560.0, y: 560.0 },
                }),
                &vec![
                    String::from("my go Buffered Robin Nola Alden Line"),
                    String::from("A Second Line"),
                    String::from("A Third Line"),
                    String::from("A Forth Line"),
                    String::from("A Fifth Line"),
                    String::from("A Sixth Line"),
                    String::from("A Seventh Line"),
                    String::from("A Eighth Line"),
                ],
            );
            let col_id = insert_text(
                &mut stage.text_data,
                &mut stage.text_component,
                Vec2 { x: 200.0, y: 200.0 },
                None,
                &vec![String::from("Old value.")],
            );
            replace_text(
                &mut stage.text_data,
                &mut stage.text_component,
                col_id,
                vec![String::from("________________🐧🐧🐧 New value!")].as_slice(),
            );
            perform_effects(&mut stage);
            stage
        })
    });
}
