#pragma once

#include <objc/runtime.h>
#include <objc/message.h>

extern SEL sel_alloc;
extern SEL sel_init;

#define objc_msgSend_void	((void (*)(id, SEL))objc_msgSend)
#define my_objc_msgSend_id  ((id (*)(id, SEL))objc_msgSend)

void release(id obj);

void register_objc_helpers();
