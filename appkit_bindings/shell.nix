# nixpkgs-25.05-darwin 2025-11-22 from https://status.nixos.org/
{ pkgs ? import (fetchTarball "https://github.com/NixOS/nixpkgs/archive/c58bc7f5459328e4afac201c5c4feb7c818d604b.tar.gz") {}}:

pkgs.mkShell {
  buildInputs = with pkgs; [
  ];
	shellHook = ''
	'';
}
