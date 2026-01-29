{
  description = "SwitchRecomp dev environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    devenv.url = "github:cachix/devenv";
    devenv.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = inputs@{ devenv, ... }:
    devenv.lib.mkFlake { inherit inputs; };
}
