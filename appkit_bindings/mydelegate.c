#include <lean/lean.h>
#include <stdio.h>
#include <objc/message.h>
#include <math.h>
#include "mydelegate.h"
#include "objc_helpers.h"

/// --- HACKY STUFF TO TRY TO GET AWAY FROM silicon.h, some of this probably needs to be shared
// external:
SEL sel_my_button_clicked = NULL;
// objective c class
Class class_delegateclass = NULL;
// lean external clas
lean_external_class *mydelegate_class = NULL;

// ------------------------------------------------

lean_object* mk_delegate(lean_obj_arg lean_closed_over_value) {
  printf("tryign to alloc %p\n", class_delegateclass);
  printf("is this different? %p\n", (id)class_delegateclass);
  id m_data_delegate = my_objc_msgSend_id((my_objc_msgSend_id((id)class_delegateclass, sel_alloc)), sel_init);
  // theses all bus errors too, so something with my alloc/init isn't working, it's not the m_data at fault.
  printf("m_data_delegate %p\n", m_data_delegate);
  object_setInstanceVariable(m_data_delegate, "lean_closed_over_value", lean_closed_over_value);
  lean_object *me = lean_alloc_external(mydelegate_class, (void*)m_data_delegate);
  return me;
}

void mydelegate_m_finalize(void *m_data_delegate) {
  // lean_dec_ref(m_data->lean_object)
  printf("calling finalize m_data_delegate %p\n", m_data_delegate);
  lean_object * val = NULL;
  object_getInstanceVariable(m_data_delegate, "lean_closed_over_value", &val);
  printf("My closed over ref is: %p\n", val);
  printf("My closed over unboxed value is: %u\n", lean_unbox(val));
  // Q: should this bee the objc's finalize override? (if we ever move the delegate to objective-C I guess)

  // I lean_dec_ref fails on boxed(47) so I guess that still counts as a scalar.
  lean_dec(val);
  release((id)m_data_delegate);
}

void none_m_foreach(void *m_data_delegate, b_lean_obj_arg fn) {
  // FIXME have to call this on the closed over value!
  // I think, standard calling convention. The fn is borrowed though; I don't release it.
  // lean_inc_ref(m_data->lean_object)
  // res = lean_apply_1(fn, lean_object)
  // lean_dec_ref(res)
}

void mydelegate_myButtonClicked(id self, SEL _cmd) {
  printf("Button clicked on id %p\n", self);
  lean_object * val = NULL;
  object_getInstanceVariable(self, "lean_closed_over_value", &val);
  printf("My closed over ref is: %p\n", val);
  printf("My closed over unboxed value is: %u\n", lean_unbox(val));
}

void register_delegate_classes() {
  // just trying to rule some things out, i wonder if you can't call sel_registerName twice? (Docs suggest you can)
  printf("allocated codes %i, %i\n", sel_alloc, sel_init);
  Class class_nsobject = objc_getClass("NSObject");
  class_delegateclass = objc_allocateClassPair(class_nsobject, "MyDelegateClass", 0);
  class_addIvar(
    class_delegateclass,
    "lean_closed_over_value",
    sizeof(lean_object*),
    log2(sizeof(lean_object*)),
    "^"  // Pointer type encoding
  );
  sel_my_button_clicked = sel_registerName("myButtonClicked");
  class_addMethod(
    class_delegateclass,
    sel_my_button_clicked,
    (IMP)mydelegate_myButtonClicked,
    "v@:"
  );
  objc_registerClassPair(class_delegateclass);
  mydelegate_class = lean_register_external_class(&mydelegate_m_finalize, &none_m_foreach);
}
