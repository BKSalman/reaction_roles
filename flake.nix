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
      in
    with pkgs; {
      devShell = mkShell.override {
          stdenv = pkgs.stdenvAdapters.useMoldLinker pkgs.clangStdenv;
        } {
          packages = [
            # shuttle deployment
            cargo-shuttle

            # database
            postgresql
            docker
          ];
          
          nativeBuildInputs = [
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
            # run dev servers
            alias lcr='RUST_LOG=reaction_roles=debug,tower=debug cargo run'
            alias lcw='RUST_LOG=reaction_roles=debug,tower=debug cargo watch -x run'

            # start dev database if available, if not create, and run it on port 5445
            docker start reaction-roles-dev || \
              docker run \
              --name reaction-roles-dev \
              -p 5445:5432 \
              -e POSTGRES_PASSWORD=123 \
              -d postgres

            # add DATABASE_URL to .env file if not already there
            grep DATABASE_URL .env || echo "DATABASE_URL=postgres://postgres:123@localhost:5445/postgres" >> .env

            # export environment variables
            export $(cat .env)
          '';
        };

      formatter.x86_64-linux = legacyPackages.${system}.nixpkgs-fmt;
    });
}
