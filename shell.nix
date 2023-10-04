# 2023-09-26 lean4: 4.0.0 -> 4.1.0 #257422 https://github.com/NixOS/nixpkgs/pull/257422
# (fetchTarball "https://github.com/NixOS/nixpkgs/archive/85e0d60613ee638751089708b8ac77f83b6f37d2.tar.gz")
{ pkgs ? import (fetchGit {
                   url = "/Users/nathan/src/nix/nixpkgs";
									 ref = "takeoutweight/mac-vulkan-validation";
									 shallow = true;
                 })
  {config.allowUnfree = false;}}:

pkgs.mkShell {
  buildInputs = with pkgs; [
		vulkan-headers
		vulkan-loader
		vulkan-validation-layers
		pkg-config
		shaderc
		iconv
		rustc
		rust-analyzer
		cargo
		darwin.apple_sdk.frameworks.AppKit
		darwin.apple_sdk.frameworks.CoreGraphics
		darwin.libobjc
  ];
	# reading https://matklad.github.io/2022/03/14/rpath-or-why-lld-doesnt-work-on-nixos.html
	# makes me thing I shouldn't have to do this, and maybe am calling the wrong thing somewhere.
	shellHook = ''
	  export SHADERC_LIB_DIR=${pkgs.shaderc.lib}/lib &&
		## Not sure I needed this one:
		## VULKAN_SDK=${pkgs.vulkan-loader} &&
		export RUSTFLAGS="-C link-arg=-Wl,-rpath,${pkgs.vulkan-loader}/lib" &&
		export VK_ADD_LAYER_PATH=${pkgs.vulkan-validation-layers}/share/vulkan/explicit_layer.d
	'';
}
