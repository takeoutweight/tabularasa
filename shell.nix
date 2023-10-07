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
		pkg-config
		iconv
		rustc
		rust-analyzer
		cargo
		darwin.apple_sdk_11_0.frameworks.Foundation
		darwin.apple_sdk_11_0.frameworks.ImageIO
		darwin.apple_sdk_11_0.frameworks.AppKit
		darwin.apple_sdk_11_0.frameworks.Vision
		darwin.apple_sdk_11_0.frameworks.CoreGraphics
		darwin.apple_sdk_11_0.frameworks.Metal
		darwin.apple_sdk_11_0.frameworks.AVFoundation
		darwin.apple_sdk_11_0.frameworks.CoreMIDI
		darwin.apple_sdk_11_0.frameworks.MetalKit
		darwin.libobjc
  ];
	shellHook = ''
	'';
}
