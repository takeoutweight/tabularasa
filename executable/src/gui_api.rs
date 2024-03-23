use crate::lean_experiments;
use std::collections::HashMap;

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
#[derive(Debug)]
enum Event {
    Init,
    AlphaNumeric,
    Up,
    Down,
}

extern "C" fn on_event(interp: &mut Interpreter, evt: u8) {
    let e = evt as Event;
    print!("Rust: on_event called with {:?}", e);
    interp.cur_event = e;
    lean_experiments::lean_io_result_mk_ok(0);
}

fn mk_on_event_closure() -> *mut lean_experiments::LeanIOClosure {
    unsafe {
        let m = lean_experiments::lean_alloc_small(24, (24 / 8) - 1)
            as *mut lean_experiments::LeanIOClosure;
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
