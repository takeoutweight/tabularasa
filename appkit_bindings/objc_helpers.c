#include <objc/message.h>
#include "objc_helpers.h"

SEL sel_alloc = NULL;
SEL sel_init = NULL;
SEL ns_release_code = NULL;

void release(id obj) { objc_msgSend_void(obj, ns_release_code); }

void register_objc_helpers() {
  ns_release_code = sel_registerName("release");
  sel_alloc = sel_registerName("alloc");
  sel_init = sel_registerName("init");
}
