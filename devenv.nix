{ pkgs, ... }:
{
  packages = [
    pkgs.cargo
    pkgs.clippy
    pkgs.ripgrep
    pkgs.rustc
    pkgs.rustfmt
  ];

  cachix.enable = false;

  env.RUST_BACKTRACE = "1";

  enterShell = ''
    echo "devenv: SwitchRecomp"
  '';
}
