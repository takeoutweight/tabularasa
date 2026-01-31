#pragma once

#include <lean/lean.h>
#include <objc/runtime.h>

lean_object* mk_delegate(lean_obj_arg lean_closed_over_value);

void register_delegate_classes();

extern SEL sel_my_button_clicked;
