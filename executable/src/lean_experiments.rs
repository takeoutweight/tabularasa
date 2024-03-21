use memoffset::raw_field;
use std::{ffi, mem, ptr, slice, str};

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

// This is is a str referencing the string living in Lean's memory, so careful of its lifespan w.r.t. refcounting.
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

pub fn test_lean() {
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
}
