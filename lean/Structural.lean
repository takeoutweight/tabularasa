@[export struct_hello]
def structHello := "world from here"

@[export leans_answer]
def leansAnswer (_ : Unit) : UInt8 := 47

-- @[extern "rusts_answer"]
-- opaque rustsAnswer : IO UInt8
-- 
-- @[export leans_other_answer]
-- def leansOtherAnswer : IO UInt8 := rustsAnswer
