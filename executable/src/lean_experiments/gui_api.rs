use crate::lean_experiments;
use crossbeam::atomic::AtomicCell;
use num_enum::TryFromPrimitive;
use std::collections::HashMap;
use std::convert::TryFrom;

#[repr(C)]
#[derive(Copy, Clone)]
struct Vec2 {
    x: f32,
    y: f32,
}

// Pull into shared types higher up?
struct Clip {
    pos: Vec2,
    size: Vec2,
}

enum AppendMode {
    Append,
    Replace,
}

type ColID = u32;

struct Effects {
    next_id: u32,
    text: HashMap<ColID, (AppendMode, Vec<String>)>,
    clip: HashMap<ColID, Option<Clip>>,
    animate: HashMap<ColID, (Vec2, f32)>,
    should_quit: bool,
}

struct Interpreter {
    cur_event: Event,
    effects: HashMap<Event, Effects>,
    committed: bool,
}

#[repr(u8)]
#[derive(Debug, Eq, PartialEq, TryFromPrimitive)]
enum Event {
    Init,
    AlphaNumeric,
    Up,
    Down,
}

#[derive(Clone, PartialEq, std::cmp::Eq, std::marker::Copy)]
struct ClassPointer(*const libc::c_void);
unsafe impl Send for ClassPointer {}

static INTERPRETER_CLASS: AtomicCell<Option<ClassPointer>> = AtomicCell::new(None);

extern "C" fn finalize(this: *mut libc::c_void) {}

extern "C" fn for_each(this: *mut libc::c_void, obj: *mut lean_experiments::LeanObject) {}

pub fn register_interpreter() {
    unsafe {
        let _ = INTERPRETER_CLASS.fetch_update(|c| match c {
            None => {
                // I think this is ok if we register many times, as long as only one wins.
                let ec = lean_experiments::lean_register_external_class(finalize, for_each);
                Some(Some(ClassPointer(ec as *const libc::c_void)))
            }
            _ => None,
        });
    }
}

extern "C" fn on_event(
    interp: *mut lean_experiments::LeanObject,
    evt: u8,
    _io: *mut lean_experiments::LeanObject,
) -> *mut lean_experiments::LeanOKCtor {
    let e: Event = Event::try_from(evt).unwrap();
    print!("Rust: on_event called with {:?}", e);
    //    interp.cur_event = e;
    lean_experiments::lean_io_result_mk_ok(0)
}

fn mk_on_event_closure() -> *mut lean_experiments::LeanOnEventClosure {
    unsafe {
        let m = lean_experiments::lean_alloc_small(24, (24 / 8) - 1)
            as *mut lean_experiments::LeanOnEventClosure;
        (*m).m_header.m_rc = 1;
        (*m).m_header.m_tag = 245; // LeanClosure
        (*m).m_header.m_other = 0;
        (*m).m_header.m_cs_sz = 0;
        (*m).m_fun = on_event;
        (*m).m_arity = 2;
        (*m).m_num_fixed = 0;
        m
    }
}
