@[export struct_hello]
def structHello := "world from here"

@[export leans_answer]
def leansAnswer (_ : Unit) : UInt8 := 47

@[extern "rusts_answer"]
opaque rustsAnswer : UInt8 -> UInt8

@[export leans_other_answer]
def leansOtherAnswer (_ : Unit) : UInt8 := rustsAnswer 4

@[export lean_use_callback]
def leanUseCallback (a : UInt8 -> UInt8) : UInt8 := a 60

@[export lean_io_test]
def leanIOTest (a : UInt8) : IO UInt8 := pure a

@[export lean_use_io_callback]
def leanUseIOCallback (a : UInt8 -> IO UInt8) : IO UInt8 := do
  IO.println "printing from Lean's io"
  let r <- a 70
  IO.println s!"Lean's io saw: {r}"
  return r

@[export lean_use_io_string_callback]
def leanUseIOStringCallback (a : String -> IO String) : IO String := do
  IO.println "printing from Lean's string io"
  let str := "world!!!!" ++ "💖"
  let r <- a str
  let r2 <- a r
  IO.println s!"Lean's string io saw: {r}, {r2}"
  IO.println s!"And just reffering to str after callback: {str}"
  return r2 ++ str

-- @[extern "rusts_answer"]
-- opaque rustsAnswer : IO UInt8

-- @[export leans_other_answer]
-- def leansOtherAnswer : IO UInt8 := rustsAnswer
inductive Event where
  | init : Event -- called immediately after leanOnInit with the state provided from that
  | alpha_numeric : Event
  | up : Event
  | down : Event
  deriving Repr

structure State where
  text : String
  deriving Repr

@[export lean_use_on_event]
def leanUseOnEvent(on_event : Event -> IO Uint8) (clear_effects : Event -> IO Uint8) : IO Unit := do
  IO.println "ok, starting"
  _ <- on_event Event.up
  _ <- clear_effects Event.down
  _ <- on_event Event.alpha_numeric
  IO.println "ok, done"

@[export lean_on_event]
def leanOnEvent
    (event : Event)
    (state : State)
    (setAppState : State -> IO Unit)
    (freshColumn : Float -> Float -> IO UInt64)
    : IO Unit := do
  setAppState {text := state.text ++ "!"}
--  _ <- freshColumn 1.0 2.0 -- getting segfault here
  IO.println s!"ok, called leanOnEvent. event: {repr event} with state: {repr state}"

-- maybe think of better name, like initial_state, to distinguish from the on init event
@[export lean_on_init]
def leanOnInit : IO State := do
  return {text := "init"}
