# nixpkgs-23.11-darwin 2024-03-15 from https://status.nixos.org/
{ pkgs ? import (fetchTarball "https://github.com/NixOS/nixpkgs/archive/3e6b090c05a4a86e6b9de820f039871ff839d3e7.tar.gz") {}}:

pkgs.mkShell {
  buildInputs = with pkgs; [
		pkg-config
		iconv
		rustc
		rust-analyzer
		rustfmt
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
