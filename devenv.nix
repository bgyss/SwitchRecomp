{ pkgs, ... }:
{
  packages = [
    pkgs.cargo
    pkgs.clippy
    pkgs.prek
    pkgs.ripgrep
    pkgs.rustc
    pkgs.rustfmt
  ] ++ pkgs.lib.optionals (!pkgs.stdenv.isDarwin) [
    pkgs.pre-commit
  ];

  cachix.enable = false;

  env.RUST_BACKTRACE = "1";

  enterShell = ''
    echo "devenv: SwitchRecomp"
  '';
}
