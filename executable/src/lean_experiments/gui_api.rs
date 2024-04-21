use crate::lean_experiments;
use crate::lean_experiments::{Closure, LeanExternalObject, LeanOKCtor, LeanOKU64Ctor, LeanObject};
use crossbeam::atomic::AtomicCell;
use num_enum::TryFromPrimitive;
use std::collections::HashMap;
use std::convert::TryFrom;

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
    pub new_columns: Vec<(ColID, Vec2)>,
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
    ASAP,
    AlphaNumeric,
    Up,
    Down,
}

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

pub type FreshColumn =
    extern "C" fn(*mut LeanObject, f32, f32, *mut LeanObject) -> *mut LeanOKU64Ctor;

pub extern "C" fn fresh_column(
    interp: *mut LeanObject,
    pos_x: f32,
    pos_y: f32,
    _io: *mut LeanObject,
) -> *mut LeanOKU64Ctor {
    let o = interp as *mut LeanExternalObject;
    unsafe {
        let interp = (*o).m_data as *mut Interpreter;
        let id = (*interp).effects.next_id;
        (*interp)
            .effects
            .new_columns
            .push((id, Vec2 { x: pos_x, y: pos_y }));
        (*interp).effects.next_id = id + 1;
        lean_experiments::lean_io_result_mk_u64_ok(id)
    }
}

pub fn mk_fresh_column(interp: &mut Interpreter) -> *mut Closure<FreshColumn> {
    lean_experiments::mk_closure_2(fresh_column, mk_external(interp), 4)
}
