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
def leanUseIOCallback (a : UInt8 -> IO UInt8) : IO UInt8 := a 70

-- @[extern "rusts_answer"]
-- opaque rustsAnswer : IO UInt8

-- @[export leans_other_answer]
-- def leansOtherAnswer : IO UInt8 := rustsAnswer

