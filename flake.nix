{
  description = "Rust development shell for Bevy and wgpu";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      forAllSystems = nixpkgs.lib.genAttrs systems;
    in {
      devShells = forAllSystems (system:
        let
          pkgs = import nixpkgs { inherit system; };
          isLinux = pkgs.stdenv.isLinux;

          linuxLibraries = with pkgs; lib.optionals isLinux [
            alsa-lib
            libxkbcommon
            wayland
            vulkan-loader
            udev
            libGL
            xorg.libX11
            xorg.libXcursor
            xorg.libXi
            xorg.libXrandr
          ];
        in {
          default = pkgs.mkShell {
            packages = with pkgs; [
              cargo
              rustc
              rustfmt
              clippy
              pkg-config
              cmake
              clang
            ];

            # Bevy/wgpu uses these libraries at runtime on Linux.
            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath linuxLibraries;
            LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";

            shellHook = ''
              export RUST_BACKTRACE=1
              export CARGO_TERM_COLOR=always
            '';
          };
        });
    };
}
