/*
	Based on: https://github.com/gammasoft71/Examples_Cocoa/blob/master/src/HelloWorlds/HelloWorld/README.md
*/

#define GL_SILENCE_DEPRECATION
#define SILICON_IMPLEMENTATION
#include <silicon.h>
#include <lean/lean.h>

// Don't know why this isnt in lean.h?
void lean_initialize_runtime_module();
lean_object * initialize_Structural(uint8_t builtin, lean_object *);
uint8_t leans_answer(uintptr_t unit);

const uintptr_t LEAN_UNIT = (0 << 1) | 1;

NSApplication* NSApp;

bool windowShouldClose(id sender)  {
	NSApplication_terminate(NSApp, sender);
	return true;
}

void mydelegate_myButtonClicked(id self, SEL _cmd) {
  printf("Button clicked on id %p\n", self);
  lean_object * val = NULL;
  object_getInstanceVariable(self, "lean_closed_over_value", &val);
  printf("My closed over ref is: %p\n", val);
  printf("My closed over unboxed value is: %u\n", lean_unbox(val));
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

// lean external class
lean_external_class *mydelegate_class = NULL;

SEL sel_alloc = NULL;
SEL sel_init = NULL;
SEL sel_my_button_clicked = NULL;
// objective c class
Class class_delegateclass = NULL;

void register_external_classes() {
  // just trying to rule some things out, i wonder if you can't call sel_registerName twice? (Docs suggest you can)
  sel_alloc = SI_NS_FUNCTIONS[NS_ALLOC_CODE]; // sel_registerName("alloc");
  sel_init = SI_NS_FUNCTIONS[NS_INIT_CODE]; // sel_registerName("init");
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

lean_object* mk_delegate(lean_obj_arg lean_closed_over_value) {
  printf("tryign to alloc %p\n", class_delegateclass);
  printf("is this different? %p\n", (id)class_delegateclass);
  id m_data_delegate = objc_msgSend_id((objc_msgSend_id((id)class_delegateclass, sel_alloc)), sel_init);
  // theses all bus errors too, so something with my alloc/init isn't working, it's not the m_data at fault.
  printf("m_data_delegate %p\n", m_data_delegate);
  object_setInstanceVariable(m_data_delegate, "lean_closed_over_value", lean_closed_over_value);
  lean_object *me = lean_alloc_external(mydelegate_class, (void*)m_data_delegate);
  return me;
}

int main() {
	// Convert C functions to Objective-C methods (refer to the 'si_func_to_SEL' comment from 'examples/menu.c' for more).
	si_func_to_SEL(SI_DEFAULT, windowShouldClose);

  // this calls si_initNS
	NSApp = NSApplication_sharedApplication();
	NSApplication_setActivationPolicy(NSApp, NSApplicationActivationPolicyRegular);

  lean_initialize_runtime_module();

  // Guessing this has to be before initializing code that might need these types?
  register_external_classes();

  lean_object * res;
  uint8_t builtin = 1;
  res = initialize_Structural(builtin, lean_io_mk_world());
  if (lean_io_result_is_ok(res)) {
      lean_dec_ref(res);
  } else {
      lean_io_result_show_error(res);
      lean_dec(res);
      // Should bail here
  }
  lean_io_mark_end_initialization();

  uint8_t answer = leans_answer(LEAN_UNIT);
  printf("Hello\n");
  printf("leans_answer %u\n", answer);

  lean_object *closed_over_thing = lean_box(answer);
  printf("I'm going to close over %p\n", closed_over_thing);
  lean_object *delegate = mk_delegate(closed_over_thing);
  printf("I just made an %p\n", delegate);
  


  char buffer[256];
  snprintf(buffer, sizeof(buffer), "Hello from Lean: %u", answer);

	NSTextField* label = NSTextField_initWithFrame(NSMakeRect(5, 100,790, 100));
	NSTextField_setStringValue(label, buffer);
	NSTextField_setBezeled(label, false);
	NSTextField_setDrawsBackground(label, false);
	NSTextField_setEditable(label, false);
	NSTextField_setSelectable(label, false);
	NSTextField_setTextColor(label, NSColor_colorWithSRGB(0.0, 0.5, 0.0, 1.0));

	NSFontManager* font_manager = NSFontManager_sharedFontManager();
	NSFont* current_font = NSTextField_font(label);
	current_font = NSFont_init(NSFont_fontName(current_font), 45);
	current_font = NSFontManager_convertFontToHaveTrait(font_manager, current_font, NSFontBoldTrait);
	current_font = NSFontManager_convertFontToHaveTrait(font_manager, current_font, NSFontItalicTrait);
	NSTextField_setFont(label, current_font);

	NSWindow* win = NSWindow_init(NSMakeRect(0, 0, 300, 300), NSWindowStyleMaskTitled | NSWindowStyleMaskClosable | NSWindowStyleMaskMiniaturizable | NSWindowStyleMaskResizable, NSBackingStoreBuffered, false);
	NSWindow_setTitle(win, "Hello world (label)");

  printf("about to create button\n");
  NSButton* button1 = (NSButton *)((id (*)(id, SEL, id, id, SEL))objc_msgSend)
    (SI_NS_CLASSES[NS_BUTTON_CODE], sel_registerName("buttonWithTitle:target:action:"), NSString_stringWithUTF8String("Can I label like this?"), lean_get_external_data(delegate), sel_my_button_clicked);

  // You can deallocate it early, it's gone and no one segfaults!
  // lean_dec(delegate);

  //NSButton* button1 = NSButton_initWithFrame(NSMakeRect(50, 225, 90, 25));
	NSButton_setBezelStyle(button1, NSBezelStyleRounded);
  // I think this works the same as using the buttonWithTitle factory as above
	// NSButton_setTarget(button1, lean_get_external_data(delegate));
	// NSButton_setAction(button1, sel_my_button_clicked);
	NSButton_setAutoresizingMask(button1, NSViewMaxXMargin | NSViewMinYMargin);

  NSView_addSubview(NSWindow_contentView(win), (NSView*)label);
  NSView_addSubview(NSWindow_contentView(win), (NSView*)button1);

	NSWindow_setIsVisible(win, true);
  NSWindow_center(win);

	NSWindow_makeMainWindow(win);
  NSApplication_run(NSApp);

  // Not sure why I don't see the finalizer run here in stdout. Does it close before flushing or something?
  // maybe you have to do this via windowShouldClose or something.
  // could also try experimenting with removeFromSuperview
  lean_dec(delegate);

	return 0;
}
