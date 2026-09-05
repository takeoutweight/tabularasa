ARGS = -g -o0 -Wall

# library:
#   clang appkit_bindings.c -dynamiclib -o appkitbinding.dylib -install_name

hello-lean:
	clang $(ARGS) hello-lean.c mydelegate.c lean_wrapped.c objc_helpers.c -I./ -I/nix/store/2yfwp87yicb7kz80n70iv0clq924iqrj-lean4-4.3.0/include -L. -lleanshared -lStructural -Wl,-rpath,. -framework Cocoa -o build/hello-lean
