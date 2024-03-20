use cosmic_text::{
    Attrs, AttrsList, BufferLine, CacheKey, Color, FontSystem, LayoutGlyph, ShapeBuffer, Shaping,
    SubpixelBin, SwashCache, Wrap,
};
use fontdb::Family;
use miniquad::window::quit;
use miniquad::*;
use texture_packer::packer::{Packer, SkylinePacker};
use texture_packer::rect::Rect;
use texture_packer::TexturePackerConfig;
// use texture_packer::importer::
// use image_importer::ImageImporter;
use memoffset::raw_field;
use std::cmp::{max, min};
use std::collections::HashMap;
use std::{ffi, mem, ptr, slice, str, thread, time};
use swash::scale::image::Content;

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

#[repr(C)]
struct LeanObject {
    m_rc: libc::c_int,
    m_cs_sz: libc::c_ushort,
    m_other: libc::c_uchar,
    m_tag: libc::c_uchar,
}

#[repr(C)]
struct LeanString {
    m_header: LeanObject,
    m_size: usize, // byte length including \0 terminator
    m_capacity: usize,
    m_length: usize, //utf8 length
    m_data: [u8; 0], // libc::c_char is i8
}

#[repr(C)]
pub struct LeanOKCtor {
    m_header: LeanObject,
    m_objs_0: u8,
    m_objs_1: libc::uintptr_t,
}

#[repr(C)]
pub struct LeanOKStringCtor {
    m_header: LeanObject,
    m_objs_0: *mut LeanString,
    m_objs_1: libc::uintptr_t,
}

#[repr(C)]
pub struct LeanClosure {
    m_header: LeanObject,
    m_fun: extern "C" fn(u8) -> u8,
    m_arity: u16,
    m_num_fixed: u16,
}

#[repr(C)]
pub struct LeanIOClosure {
    m_header: LeanObject,
    m_fun: extern "C" fn(*mut LeanObject, *mut LeanObject) -> *mut LeanOKCtor,
    m_arity: u16,
    m_num_fixed: u16,
}

#[repr(C)]
pub struct LeanIOStringClosure {
    m_header: LeanObject,
    m_fun: extern "C" fn(*mut LeanObject, *mut LeanObject) -> *mut LeanOKStringCtor,
    m_arity: u16,
    m_num_fixed: u16,
}

const LEAN_UNIT: libc::uintptr_t = (0 << 1) | 1;

#[link(name = "leanshared")]
extern "C" {
    fn lean_initialize_runtime_module();
    fn lean_init_task_manager(); // for Task
    fn lean_initialize_thread();
    fn lean_finalize_thread();
    fn lean_io_mark_end_initialization();
    fn lean_io_result_show_error(o: *mut LeanObject);
    fn lean_dec_ref_cold(o: *mut LeanObject);
    fn lean_alloc_small(sz: u8, slot_idx: u8) -> *mut libc::c_void;
    fn lean_alloc_object(sz: usize) -> *mut libc::c_void;
}

// #[link(name = "Structural-1")]
#[link(name = "Structural")]
extern "C" {
    fn initialize_Structural(builtin: u8, io: libc::uintptr_t) -> *mut LeanObject;
    fn leans_answer(unit: libc::uintptr_t) -> u8;
    fn leans_other_answer(_: u8) -> u8;
    fn lean_use_callback(a: *mut LeanClosure) -> u8;
    fn lean_use_io_callback(a: *mut LeanIOClosure) -> *mut LeanObject;
    fn lean_use_io_string_callback(a: *mut LeanIOStringClosure) -> *mut LeanObject;
}

fn lean_dec_ref(o: *mut LeanObject) {
    unsafe {
        if (*o).m_rc > 1 {
            (*o).m_rc -= 1;
        } else if (*o).m_rc != 0 {
            lean_dec_ref_cold(o);
        }
    }
}

extern "C" fn rust_callback(a: u8) -> u8 {
    let unboxed = a >> 1;
    println!("I'm being called with {} = {}", a, unboxed);
    unboxed + 7
}

fn mk_closure() -> *mut LeanClosure {
    unsafe {
        let m = lean_alloc_small(24, (24 / 8) - 1) as *mut LeanClosure;
        (*m).m_header.m_rc = 1;
        (*m).m_header.m_tag = 245; // LeanClosure
        (*m).m_header.m_other = 0;
        (*m).m_header.m_cs_sz = 0;
        (*m).m_fun = rust_callback;
        (*m).m_arity = 1;
        (*m).m_num_fixed = 0;
        m
    }
}

extern "C" fn rust_io_callback(a: *mut LeanObject, _io: *mut LeanObject) -> *mut LeanOKCtor {
    let unboxed = a as u8 >> 1;
    println!("I'm io called with {}", unboxed);
    lean_io_result_mk_ok(unboxed + 8)
}

fn mk_io_closure() -> *mut LeanIOClosure {
    unsafe {
        let m = lean_alloc_small(24, (24 / 8) - 1) as *mut LeanIOClosure;
        (*m).m_header.m_rc = 1;
        (*m).m_header.m_tag = 245; // LeanClosure
        (*m).m_header.m_other = 0;
        (*m).m_header.m_cs_sz = 0;
        (*m).m_fun = rust_io_callback;
        (*m).m_arity = 2;
        (*m).m_num_fixed = 0;
        m
    }
}

fn str_from_lean(lstring: *mut LeanString) -> &'static str {
    let ptr = raw_field!(lstring, LeanString, m_data) as *const u8;
    unsafe {
        println!("Size we're about to pull {}", (*lstring).m_size);
        let slice: &[u8] = slice::from_raw_parts(ptr, (*lstring).m_size);
        let cstr = ffi::CStr::from_bytes_with_nul_unchecked(slice);
        str::from_utf8_unchecked(cstr.to_bytes())
    }
}

extern "C" fn rust_io_string_callback(
    a: *mut LeanObject,
    _io: *mut LeanObject,
) -> *mut LeanOKStringCtor {
    let ls = a as *mut LeanString;
    let string = str_from_lean(ls);
    println!("I'm io string called with {}", string);
    let out = format!("{string} but from rust 🦀");
    unsafe {
        println!("FYI the refcount is: {}", (*a).m_rc);
        lean_dec_ref(a);
        lean_io_result_mk_string_ok(out.as_str())
    }
}

fn mk_io_string_closure() -> *mut LeanIOStringClosure {
    unsafe {
        let m = lean_alloc_small(24, (24 / 8) - 1) as *mut LeanIOStringClosure;
        (*m).m_header.m_rc = 1;
        (*m).m_header.m_tag = 245; // LeanClosure
        (*m).m_header.m_other = 0;
        (*m).m_header.m_cs_sz = 0;
        (*m).m_fun = rust_io_string_callback;
        (*m).m_arity = 2;
        (*m).m_num_fixed = 0;
        m
    }
}

fn lean_io_result_mk_ok(res: u8) -> *mut LeanOKCtor {
    unsafe {
        let m = lean_alloc_small(24, (24 / 8) - 1) as *mut LeanOKCtor;
        (*m).m_header.m_rc = 1;
        (*m).m_header.m_tag = 0;
        (*m).m_header.m_other = 2;
        (*m).m_header.m_cs_sz = 0;
        (*m).m_objs_0 = (res << 1) | 1;
        (*m).m_objs_1 = LEAN_UNIT;
        println!("got here in mk_ok");
        m
    }
}

fn lean_io_result_mk_string_ok(string: &str) -> *mut LeanOKStringCtor {
    unsafe {
        let s = mk_lean_string(string);
        let m = lean_alloc_small(24, (24 / 8) - 1) as *mut LeanOKStringCtor;
        (*m).m_header.m_rc = 1;
        (*m).m_header.m_tag = 0;
        (*m).m_header.m_other = 2;
        (*m).m_header.m_cs_sz = 0;
        (*m).m_objs_0 = s;
        (*m).m_objs_1 = LEAN_UNIT;
        m
    }
}

// copies the string to Lean's memory.
fn mk_lean_string(string: &str) -> *mut LeanString {
    let cstring = ffi::CString::new(string.to_string()).unwrap();
    let num_bytes = cstring.to_bytes_with_nul().len();
    unsafe {
        let m = lean_alloc_object(mem::size_of::<LeanString>() + string.len()) as *mut LeanString; // 32
        (*m).m_header.m_rc = 1;
        (*m).m_header.m_tag = 249; // #define LeanString      249
        (*m).m_header.m_other = 0;
        (*m).m_header.m_cs_sz = 0;
        (*m).m_size = num_bytes;
        (*m).m_capacity = num_bytes;
        (*m).m_length = string.chars().count();
        let ptr = raw_field!(m, LeanString, m_data) as *mut i8;
        ptr::copy(cstring.as_ptr(), ptr, num_bytes);
        m
    }
}

#[no_mangle]
pub extern "C" fn rusts_answer() -> *mut LeanOKCtor {
    lean_io_result_mk_ok(90)
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

    // not a str because the bufferline owns them. maybe better to copy inside somewhere?
    pub fn insert_text(&mut self, pos: Vec2, clip: Option<Clip>, texts: &'_ [String]) -> usize {
        let new_offset = self.text_data.laid_out_lines.len();
        let mut new_size = 0;

        {
            let mut cur_offset = new_offset;
            self.text_data.columns.push(Column {
                pos,
                animation: None,
                clip,
                length: texts.len(),
                offset: cur_offset,
            });
            cur_offset += texts.len();
            new_size += texts.len();
            self.text_data.laid_out_lines.extend(
                texts
                    .iter()
                    .map(|text| layout(text, &mut self.text_component)),
            );
        }
        let unbound_offset = self.text_data.unbound_laid_out_offset.min(new_offset);
        self.text_data.unbound_laid_out_offset = unbound_offset;
        self.text_data.unbound_laid_out_length = new_offset + new_size - unbound_offset;
        let col_id = self.text_data.columns.len() - 1;
        col_id
    }

    pub fn replace_text(&mut self, col_id: usize, texts: &'_ [String]) {
        let col = &self.text_data.columns[col_id];
        assert!(col.length == texts.len());
        self.text_data.laid_out_lines.splice(
            col.offset..col.offset + col.length,
            texts
                .iter()
                .map(|text| layout(text, &mut self.text_component)),
        );
        let unbound_offset = self.text_data.unbound_laid_out_offset.min(col.offset);
        let unbound_length = max(
            col.offset + col.length,
            self.text_data.unbound_laid_out_offset + self.text_data.unbound_laid_out_length,
        ) - unbound_offset;
        self.text_data.unbound_laid_out_offset = unbound_offset;
        self.text_data.unbound_laid_out_length = unbound_length;
    }
}

struct Stage {
    ctx: Box<dyn RenderingBackend>,
    pipeline: Pipeline,
    window_width: f32,
    window_height: f32,
    draws_remaining: i32,
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

fn main() {
    println!("size of LEANOKCtor: {}", mem::size_of::<LeanOKCtor>());
    println!("size of LEANClosure {}", mem::size_of::<LeanClosure>());
    println!("size of LEANString {}", mem::size_of::<LeanString>());

    unsafe {
        lean_initialize_runtime_module();
        let res = initialize_Structural(1, LEAN_UNIT);
        if (*res).m_tag == 0 {
            lean_dec_ref(res);
        } else {
            println!("failed to load lean: {:?}", res);
            lean_io_result_show_error(res);
            lean_dec_ref(res);
            return;
        }
        lean_io_mark_end_initialization();

        let a = leans_answer(LEAN_UNIT);
        println!("Lean's answer: {}", a);
        // let b = leans_other_answer(12);
        // println!("Lean's other answer: {}", b);
        let cb = mk_closure();
        let r = lean_use_callback(cb);
        println!("Lean's callback: {}", r);

        let cbio = mk_io_closure();
        let r2 = lean_use_io_callback(cbio) as *mut LeanOKCtor; // todo case check?
        println!("Lean's io callback: {}", (*r2).m_objs_0 >> 1); // toodo unwrap
        lean_dec_ref(r2 as *mut LeanObject);

        let cbios = mk_io_string_closure();
        let r3 = lean_use_io_string_callback(cbios) as *mut LeanOKStringCtor;
        println!("Lean's io string: {}", str_from_lean((*r3).m_objs_0));
        println!(
            "Lean's refcounts: {}, {}",
            (*r3).m_header.m_rc,
            (*(*r3).m_objs_0).m_header.m_rc
        );
        lean_dec_ref(r3 as *mut LeanObject);
    }

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
            let mut stage = Stage::new(window_width, window_height);
            stage.insert_text(
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
            let col_id = stage.insert_text(
                Vec2 { x: 200.0, y: 200.0 },
                None,
                &vec![String::from("Old value.")],
            );
            stage.replace_text(
                col_id,
                vec![String::from("________________🐧🐧🐧 New value!")].as_slice(),
            );
            stage
        })
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
