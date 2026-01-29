{ pkgs, ... }:
{
  devenv.root = builtins.toString ./.;

  packages = [
    pkgs.cargo
    pkgs.clippy
    pkgs.ripgrep
    pkgs.rustc
    pkgs.rustfmt
  ];

  env.RUST_BACKTRACE = "1";

  enterShell = ''
    echo "devenv: SwitchRecomp"
  '';
}
