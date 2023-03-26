{
  description = "Glean build environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs";
    android = {
      url = "github:tadfisher/android-nixpkgs";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, android, fenix }:
    let
      supportedSystems = [ "x86_64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forAllSystems = f: nixpkgs.lib.genAttrs supportedSystems (system: f system);
    in
    {
      devShells = forAllSystems
        (system:
          let
            pkgs = import nixpkgs {
              inherit system;
            };
            android-sdk = android.sdk.${system} (sdkPkgs: with sdkPkgs;
              [
                build-tools-30-0-3
                cmdline-tools-latest
                platform-tools
                platforms-android-33
                ndk-25-2-9519653
              ]);
            fenixToolchain = with fenix.packages.${system};
              [
                stable.cargo
                stable.clippy
                stable.rust-src
                stable.rustc
                stable.rustfmt
                targets.aarch64-linux-android.stable.rust-std
                targets.armv7-linux-androideabi.stable.rust-std
                targets.i686-linux-android.stable.rust-std
                targets.x86_64-linux-android.stable.rust-std
              ];
          in
          {
            default = with pkgs; mkShell
              ({
                ANDROID_SDK_ROOT = "${android-sdk}/share/android-sdk";
                JAVA_HOME = jdk11.home;
                name = "glean-dev";
                packages = [
                  android-sdk
                  jdk19_headless
                  clang
                  python310
                  python310Packages.pip
                  fenixToolchain
                ] ++ lib.optionals stdenv.isDarwin [
                  libiconv
                ];
              });
          }
        );
    };
}
