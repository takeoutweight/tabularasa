#include <lean/lean.h>
#include <stdio.h>
#include <objc/message.h>
#include <math.h>
#include <silicon.h>

lean_external_class *lean_wrapped_class = NULL;

lean_object* mk_lean_wrapped(NSTextField *text_field) {
  lean_object *me = lean_alloc_external(lean_wrapped_class, (void*)text_field);
  return me;
}

// a lean wrapped objective c delegate object. pulled out via lean_get_external_data
void lean_wrapped_m_finalize(void *m_lean_wrapped) {
  NSRelease((id)m_lean_wrapped);
}

void lean_wrapped_m_foreach(void *m_data_delegate, b_lean_obj_arg fn) {
  // because there are no nested lean things, I think this should be a noop
}

void register_lean_wrapped_classes() {
  lean_wrapped_class = lean_register_external_class(&lean_wrapped_m_finalize, &lean_wrapped_m_foreach);
}
