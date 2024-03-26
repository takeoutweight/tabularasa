use crate::lean_experiments;
use crate::lean_experiments::{Closure, LeanOKCtor, LeanObject};
use crossbeam::atomic::AtomicCell;
use num_enum::TryFromPrimitive;
use std::collections::HashMap;
use std::convert::TryFrom;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct Vec2 {
    x: f32,
    y: f32,
}

// Pull into shared types higher up?
#[derive(Debug)]
struct Clip {
    pos: Vec2,
    size: Vec2,
}

#[derive(Debug)]
enum AppendMode {
    Append,
    Replace,
}

type ColID = u32;

#[derive(Debug)]
pub struct Effects {
    next_id: u32,
    text: HashMap<ColID, (AppendMode, Vec<String>)>,
    clip: HashMap<ColID, Option<Clip>>,
    animate: HashMap<ColID, (Vec2, f32)>,
    should_quit: bool,
}

#[derive(Debug)]
pub struct Interpreter {
    pub cur_event: Event,
    pub effects: HashMap<Event, Effects>,
    pub committed: bool,
}

#[repr(u8)]
#[derive(Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum Event {
    Init,
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
pub fn mk_event_external(interp: &mut Interpreter) -> *mut lean_experiments::LeanExternalObject {
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

pub fn mk_on_event(interp: &mut Interpreter) -> *mut Closure<EventCallback>{
    lean_experiments::mk_closure_2(on_event, mk_event_external(interp), 3)
}

pub type ClearEffects = extern "C" fn(*mut LeanObject, u8, *mut LeanObject) -> *mut LeanOKCtor;

pub extern "C" fn clear_effects(
    interp: *mut LeanObject,
    evt: u8,
    _io: *mut LeanObject,
) -> *mut lean_experiments::LeanOKCtor {
    let e: Event = Event::try_from(evt >> 1).unwrap();
    println!("Rust: clear_effects called with: {:?}", e);
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

pub fn mk_clear_effects(interp: &mut Interpreter) -> *mut Closure<EventCallback>{
    lean_experiments::mk_closure_2(clear_effects, mk_event_external(interp), 3)
}
