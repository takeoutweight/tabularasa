import Lake
open Lake DSL

package «structural» {
  precompileModules := true
}

lean_lib «Structural» {
  defaultFacets := #[LeanLib.sharedFacet]
--  moreLeancArgs := #["-mmacos-version-min=12.0"] -- doesn't seem to make errors go away
-- MACOSX_DEPLOYMENT_TARGET=12.0 lake build -- seems to work
  moreLinkArgs := #["-install_name", "@rpath/libStructural-1.dylib"]
}

@[default_target]
lean_exe «structural» {
  root := `Main
}
