{
  description = "weshtatistic, a fast disk usage analyzer for Linux";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];

      forAllSystems = function:
        nixpkgs.lib.genAttrs systems (system:
          function (import nixpkgs {
            localSystem = system;
          }));
    in
    {
      packages = forAllSystems (pkgs: {
        default = pkgs.rustPlatform.buildRustPackage {
          pname = "weshtatistic";
          version = "2.2.0";

          src = builtins.path {
            path = ./.;
            name = "weshtatistic-source";
            filter = path: type: true;
          };

          cargoLock = {
            lockFile = ./Cargo.lock;
            outputHashes = {
              "winit-0.30.13" = "sha256-nwiD9X5rP/rrzwvzSQe6fkKzb9ixrVG7aPMtnDjMhT8=";
            };
          };

          nativeBuildInputs = [
            pkgs.makeWrapper
            pkgs.pkg-config
          ];

          buildInputs = with pkgs; [
            libxkbcommon
            wayland
            libX11
            libXcursor
            libXi
            libXrandr
            libXinerama
            libxcb
            libglvnd
            mesa
            vulkan-loader
          ];

          cargoBuildFlags = [ "--package" "weshtatistic" ];
          doCheck = false;

          postInstall = ''
            wrapProgram $out/bin/weshtatistic \
              --prefix LD_LIBRARY_PATH : "${pkgs.lib.makeLibraryPath [
                pkgs.libxkbcommon
                pkgs.wayland
                pkgs.libX11
                pkgs.libXcursor
                pkgs.libXi
                pkgs.libXrandr
                pkgs.libXinerama
                pkgs.libxcb
                pkgs.libglvnd
                pkgs.mesa
                pkgs.vulkan-loader
              ]}:/run/opengl-driver/lib"
          '';

          meta = {
            description = "weshtatistic, a fast, cross-platform disk usage analyzer and deduplicator";
            homepage = "https://github.com/weshford/weshtatistic";
            license = pkgs.lib.licenses.mit;
            mainProgram = "weshtatistic";
            platforms = pkgs.lib.platforms.linux;
          };
        };
      });

      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          packages = with pkgs; [
            rustc
            cargo
            rustfmt
            clippy
            rust-analyzer
            pkg-config
          ];

          buildInputs = with pkgs; [
            libxkbcommon
            wayland
            libX11
            libXcursor
            libXi
            libXrandr
            libXinerama
            libxcb
            libglvnd
            mesa
            vulkan-loader
          ];
        };
      });

      apps = forAllSystems (pkgs: {
        default = {
          type = "app";
          program = "${self.packages.${pkgs.stdenv.hostPlatform.system}.default}/bin/weshtatistic";
          meta = {
            description = "Run weshtatistic";
          };
        };
      });
    };
}