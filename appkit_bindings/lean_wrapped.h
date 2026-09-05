#pragma once

#include <lean/lean.h>
#include <objc/runtime.h>

lean_object* mk_lean_wrapped(NSTextField *text_field);

void register_lean_wrapped_classes();
