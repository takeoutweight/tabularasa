/*
	Based on: https://github.com/gammasoft71/Examples_Cocoa/blob/master/src/HelloWorlds/HelloWorld/README.md
*/

#define GL_SILENCE_DEPRECATION
#define SILICON_IMPLEMENTATION
#include <silicon.h>
#include <lean/lean.h>
#include "mydelegate.h"
#include "lean_wrapped.h"
#include "objc_helpers.h"

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

void register_external_classes() {
  register_objc_helpers();
  register_delegate_classes();
  register_lean_wrapped_classes();
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

  lean_object *lean_label = mk_lean_wrapped(label);

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
