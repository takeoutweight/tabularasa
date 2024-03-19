import Lake
open Lake DSL

package «structural» {
  precompileModules := true
}

lean_lib «Structural» {
  defaultFacets := #[LeanLib.sharedFacet]
--  moreLeancArgs := #["-mmacos-version-min=12.0"] -- doesn't seem to make errors go away
-- MACOSX_DEPLOYMENT_TARGET=12.0 lake build -- seems to work
-- otool -L .lake/build/lib/libStructural.dylib
-- "@rpath/libStructural-1.dylib" seems to be what you get when you build the executable,
  moreLinkArgs := #["-install_name", "@rpath/libStructural.dylib", "-L../drawing/target/debug/", "-ltabularasa_drawing"]
}

@[default_target]
lean_exe «structural» {
  moreLinkArgs := #["-L../drawing/target/debug/", "-ltabularasa_drawing"]
  root := `Main
}
