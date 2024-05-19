use crate::lean_experiments;
use crate::lean_experiments::{
    lean_dec_ref, str_from_lean, Closure, LeanBoxedU64, LeanExternalObject, LeanOKCtor,
    LeanOKU64Ctor, LeanObject, LeanString,
};
use crossbeam::atomic::AtomicCell;
use num_enum::TryFromPrimitive;
use std::collections::{BTreeMap, HashMap};
use std::convert::TryFrom;

use super::LeanBoxedFloat;

#[link(name = "Structural")]
extern "C" {
    fn lean_on_event(
        evt: u8,
        st: *mut LeanObject,
        ch: u32,
        set_app_state: *mut Closure<SetAppState>,
        fresh_column: *mut Closure<FreshColumn>,
        push_line: *mut Closure<PushLine>,
        reset_text: *mut Closure<ResetText>,
        io: libc::uintptr_t,
    ) -> *mut LeanOKCtor;
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

// Pull into shared types higher up?
#[derive(Debug)]
pub struct Clip {
    pub pos: Vec2,
    pub size: Vec2,
}

#[derive(Debug)]
pub enum AppendMode {
    Append,
    Replace,
}

type ColID = u64;

#[derive(Debug)]
pub struct Effects {
    pub next_id: u64,
    pub new_columns: BTreeMap<ColID, Vec2>,
    pub text: HashMap<ColID, (AppendMode, Vec<String>)>,
    pub clip: HashMap<ColID, Option<Clip>>,
    pub animate: HashMap<ColID, (Vec2, f32)>,
    pub app_state: *mut LeanObject,
    pub should_quit: bool,
}

#[derive(Debug)]
pub struct Interpreter {
    pub effects: Effects,
    pub committed: bool,
}

#[repr(u8)]
#[derive(Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum Event {
    Init,
    Char,
    Up,
    Down,
}

const LEAN_UNIT: libc::uintptr_t = (0 << 1) | 1;

#[derive(Clone, PartialEq, std::cmp::Eq, std::marker::Copy)]
struct ClassPointer(*const libc::c_void);
unsafe impl Send for ClassPointer {}

static INTERPRETER_CLASS: AtomicCell<Option<ClassPointer>> = AtomicCell::new(None);

extern "C" fn finalize(_this: *mut libc::c_void) {
    println!("finalize called");
}

extern "C" fn for_each(_this: *mut libc::c_void, _obj: *mut LeanObject) {
    println!("for_each called");
}

pub fn register_interpreter() {
    unsafe {
        let _ = INTERPRETER_CLASS.fetch_update(|c| match c {
            None => {
                // I think this is ok if we race register a few times, as long as only one is recorded.
                let ec = lean_experiments::lean_register_external_class(finalize, for_each);
                Some(Some(ClassPointer(ec as *const libc::c_void)))
            }
            _ => None,
        });
    }
}

// todo Result. Also lean will GC these wrappers after use I think, so might have to fuss with lifetimes
pub fn mk_external(interp: &mut Interpreter) -> *mut lean_experiments::LeanExternalObject {
    register_interpreter();
    let cls = INTERPRETER_CLASS.load().unwrap().0 as *mut lean_experiments::LeanExternalClass;
    lean_experiments::mk_external_object(cls, interp as *mut _ as *mut libc::c_void)
}

pub fn send_event_to_lean(interp: &mut Interpreter, evt: u8, ch: u32) {
    // silently doesn't call if these aren't rebuilt. Swallowing some error after GC'd?
    let sap = mk_set_app_state(interp);
    let fc = mk_fresh_column(interp);
    let pl = mk_push_line(interp);
    let rt = mk_reset_text(interp);
    unsafe {
        lean_on_event(
            evt,
            interp.effects.app_state,
            ch,
            sap,
            fc,
            pl,
            rt,
            LEAN_UNIT,
        );
    }
}

pub type EventCallback = extern "C" fn(*mut LeanObject, u8, *mut LeanObject) -> *mut LeanOKCtor;

pub extern "C" fn on_event(
    interp: *mut LeanObject,
    evt: u8,
    _io: *mut LeanObject,
) -> *mut lean_experiments::LeanOKCtor {
    let e: Event = Event::try_from(evt >> 1).unwrap();
    println!("Rust: on_event called with: {:?}", e);
    let o = interp as *mut lean_experiments::LeanExternalObject;
    unsafe {
        let interp = (*o).m_data as *mut Interpreter;
        println!("Found Interpreter: {:?}", (*interp));
        (*interp).committed = !(*interp).committed;
    }
    //    interp.cur_event = e;
    let r = lean_experiments::lean_io_result_mk_ok(0);
    println!("Made ret value");
    r
}

pub fn mk_on_event(interp: &mut Interpreter) -> *mut Closure<EventCallback> {
    lean_experiments::mk_closure_2(on_event, mk_external(interp), 3)
}

pub type SetAppState =
    extern "C" fn(*mut LeanObject, *mut LeanObject, *mut LeanObject) -> *mut LeanOKCtor;

pub extern "C" fn set_app_state(
    interp: *mut LeanObject,
    state: *mut LeanObject,
    _io: *mut LeanObject,
) -> *mut LeanOKCtor {
    let o = interp as *mut LeanExternalObject;
    unsafe {
        let interp = (*o).m_data as *mut Interpreter;
        (*interp).effects.app_state = state;
    }
    lean_experiments::lean_io_result_mk_ok(0)
}

pub fn mk_set_app_state(interp: &mut Interpreter) -> *mut Closure<SetAppState> {
    lean_experiments::mk_closure_2(set_app_state, mk_external(interp), 3)
}

pub type FreshColumn = extern "C" fn(
    *mut LeanObject,
    *mut LeanBoxedFloat,
    *mut LeanBoxedFloat,
    *mut LeanObject,
) -> *mut LeanOKU64Ctor;

pub extern "C" fn fresh_column(
    interp: *mut LeanObject,
    pos_x: *mut LeanBoxedFloat,
    pos_y: *mut LeanBoxedFloat,
    _io: *mut LeanObject,
) -> *mut LeanOKU64Ctor {
    let o = interp as *mut LeanExternalObject;
    unsafe {
        let interp = (*o).m_data as *mut Interpreter;
        let id = (*interp).effects.next_id;
        let ub_pos_x = (*pos_x).m_obj as f32;
        let ub_pos_y = (*pos_y).m_obj as f32;
        lean_dec_ref(pos_x as *mut LeanObject);
        lean_dec_ref(pos_y as *mut LeanObject);
        let old = (*interp).effects.new_columns.insert(
            id,
            Vec2 {
                x: ub_pos_x,
                y: ub_pos_y,
            },
        );
        assert!(old.is_none());
        (*interp).effects.next_id = id + 1;
        // println!("Got to the fresh_column, {},{}", ub_pos_x, ub_pos_y);
        // println!("effects: {:?}", (*interp).effects);
        lean_experiments::lean_io_result_mk_u64_ok(id)
    }
}

pub fn mk_fresh_column(interp: &mut Interpreter) -> *mut Closure<FreshColumn> {
    lean_experiments::mk_closure_2(fresh_column, mk_external(interp), 4)
}

pub type PushLine = extern "C" fn(
    *mut LeanObject,
    *mut LeanBoxedU64,
    *mut LeanString,
    *mut LeanObject,
) -> *mut LeanOKCtor;

pub extern "C" fn push_line(
    interp: *mut LeanObject,
    id: *mut LeanBoxedU64,
    text: *mut LeanString,
    _io: *mut LeanObject,
) -> *mut LeanOKCtor {
    let o = interp as *mut LeanExternalObject;
    unsafe {
        let interp = (*o).m_data as *mut Interpreter;
        let ub_id = (*id).m_obj;
        let entry = (*interp)
            .effects
            .text
            .entry(ub_id)
            .or_insert((AppendMode::Append, vec![]));
        entry.1.push(str_from_lean(text).to_owned());
        lean_dec_ref(id as *mut LeanObject);
        lean_dec_ref(text as *mut LeanObject);
        // println!("push_line: {:?}", (*interp).effects);
        lean_experiments::lean_io_result_mk_ok(0)
    }
}

pub fn mk_push_line(interp: &mut Interpreter) -> *mut Closure<PushLine> {
    lean_experiments::mk_closure_2(push_line, mk_external(interp), 4)
}

pub type ResetText =
    extern "C" fn(*mut LeanObject, *mut LeanBoxedU64, *mut LeanObject) -> *mut LeanOKCtor;

pub extern "C" fn reset_text(
    interp: *mut LeanObject,
    id: *mut LeanBoxedU64,
    _io: *mut LeanObject,
) -> *mut LeanOKCtor {
    let o = interp as *mut LeanExternalObject;
    unsafe {
        let interp = (*o).m_data as *mut Interpreter;
        let ub_id = (*id).m_obj;
        (*interp)
            .effects
            .text
            .insert(ub_id, (AppendMode::Replace, vec![]));
        lean_dec_ref(id as *mut LeanObject);
        // println!("reset_text: {:?}", (*interp).effects);
        lean_experiments::lean_io_result_mk_ok(0)
    }
}

pub fn mk_reset_text(interp: &mut Interpreter) -> *mut Closure<ResetText> {
    lean_experiments::mk_closure_2(reset_text, mk_external(interp), 3)
}

pub type SetClip = extern "C" fn(
    *mut LeanObject,
    *mut LeanBoxedU64,
    *mut LeanBoxedFloat,
    *mut LeanBoxedFloat,
    *mut LeanBoxedFloat,
    *mut LeanBoxedFloat,
    *mut LeanObject,
) -> *mut LeanOKCtor;

pub extern "C" fn set_clip(
    interp: *mut LeanObject,
    id: *mut LeanBoxedU64,
    pos_x: *mut LeanBoxedFloat,
    pos_y: *mut LeanBoxedFloat,
    width: *mut LeanBoxedFloat,
    height: *mut LeanBoxedFloat,
    _io: *mut LeanObject,
) -> *mut LeanOKCtor {
    let o = interp as *mut LeanExternalObject;
    unsafe {
        let interp = (*o).m_data as *mut Interpreter;
        let ub_id = (*id).m_obj;
        let ub_pos_x = (*pos_x).m_obj as f32;
        let ub_pos_y = (*pos_y).m_obj as f32;
        let ub_width = (*width).m_obj as f32;
        let ub_height = (*height).m_obj as f32;
        (*interp).effects.clip.insert(
            ub_id,
            Some(Clip {
                pos: Vec2 {
                    x: ub_pos_x,
                    y: ub_pos_y,
                },
                size: Vec2 {
                    x: ub_width,
                    y: ub_height,
                },
            }),
        );
        lean_dec_ref(id as *mut LeanObject);
        lean_dec_ref(pos_x as *mut LeanObject);
        lean_dec_ref(pos_y as *mut LeanObject);
        lean_dec_ref(width as *mut LeanObject);
        lean_dec_ref(height as *mut LeanObject);
        lean_experiments::lean_io_result_mk_ok(0)
    }
}

pub fn mk_set_clip(interp: &mut Interpreter) -> *mut Closure<SetClip> {
    lean_experiments::mk_closure_2(set_clip, mk_external(interp), 7)
}

pub type RemoveClip =
    extern "C" fn(*mut LeanObject, *mut LeanBoxedU64, *mut LeanObject) -> *mut LeanOKCtor;

pub extern "C" fn remove_clip(
    interp: *mut LeanObject,
    id: *mut LeanBoxedU64,
    _io: *mut LeanObject,
) -> *mut LeanOKCtor {
    let o = interp as *mut LeanExternalObject;
    unsafe {
        let interp = (*o).m_data as *mut Interpreter;
        let ub_id = (*id).m_obj;
        (*interp).effects.clip.insert(ub_id, None);
        lean_dec_ref(id as *mut LeanObject);
        lean_experiments::lean_io_result_mk_ok(0)
    }
}

pub fn mk_remove_clip(interp: &mut Interpreter) -> *mut Closure<RemoveClip> {
    lean_experiments::mk_closure_2(remove_clip, mk_external(interp), 3)
}
