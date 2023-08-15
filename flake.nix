{
  description = "Reaction roles bot flake";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = {nixpkgs, flake-utils, rust-overlay, ...}:
    flake-utils.lib.eachDefaultSystem (system: 
      let 
        pkgs = import nixpkgs { inherit system; overlays = [ rust-overlay.overlays.default ]; };

        # super hacky way to use mold
        bintools-wrapper = "${nixpkgs}/pkgs/build-support/bintools-wrapper";
        mold' = pkgs.symlinkJoin {
          name = "mold";
          paths = [ pkgs.mold ];
          nativeBuildInputs = [ pkgs.makeWrapper ];
          suffixSalt = pkgs.lib.replaceStrings ["-" "."] ["_" "_"] pkgs.targetPlatform.config;
          postBuild = ''
            for bin in ${pkgs.mold}/bin/*; do
              rm $out/bin/"$(basename "$bin")"

              export prog="$bin"
              substituteAll "${bintools-wrapper}/ld-wrapper.sh" $out/bin/"$(basename "$bin")"
              chmod +x $out/bin/"$(basename "$bin")"

              mkdir -p $out/nix-support
              substituteAll "${bintools-wrapper}/add-flags.sh" $out/nix-support/add-flags.sh
              substituteAll "${bintools-wrapper}/add-hardening.sh" $out/nix-support/add-hardening.sh
              substituteAll "${bintools-wrapper}/../wrapper-common/utils.bash" $out/nix-support/utils.bash
            done
          '';
        };
      in
    with pkgs; {
      devShell = mkShell {
          NIX_CFLAGS_LINK = "-fuse-ld=mold";
          packages = [
            # shuttle deployment
            cargo-shuttle
            
            # general utilities
            exa
            fd
            bat
            lazygit
            
            # database
            postgresql
            docker
          ];
          
          nativeBuildInputs = [
            mold'
            clang
          ];
          
          buildInputs = [
            # backend
            (rust-bin.stable.latest.default.override {
              extensions = [ "rust-src" "rust-analyzer" ];
            })
            diesel-cli
            # auto reload server on save
            # cargo watch -x run
            cargo-watch
            # needed for openssl dependant packages
            openssl
            pkg-config
          ];

          shellHook = ''
            # cool aliases
            alias ls='exa --time-style=long-iso --group-directories-first --icons --no-permissions --no-user -l --git'
            alias ll="exa --time-style=long-iso --group-directories-first --icons -la"
            alias find=fd
            alias cat=bat

            # run dev servers
            alias lcr='RUST_LOG=musawarah=debug,tower=debug cargo run'
            alias lcw='RUST_LOG=musawarah=debug,tower=debug cargo watch -x run'

            # start dev database if available, if not create, and run it on port 5445
            docker start reaction-roles-dev || \
              docker run \
              --name reaction-roles-dev \
              -p 5445:5432 \
              -e POSTGRES_PASSWORD=123 \
              -d postgres

            # add DATABASE_URL to .env file if not already there
            # grep DATABASE_URL .env || echo "DATABASE_URL=postgres://postgres:123@localhost:5445/postgres" >> .env

            # export environment variables
            export $(cat .env)
          '';
        };

      formatter.x86_64-linux = legacyPackages.${system}.nixpkgs-fmt;
    });
}
