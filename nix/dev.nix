{
  rustToolchain,
  rustPlatform,
  pkg-config,
  libclang,
  clang,
  lib,
}:
let
  cargoToml = fromTOML (builtins.readFile ../Cargo.toml);
in
rustPlatform.buildRustPackage {
  pname = "poneglyph";
  version = cargoToml.package.version;
  src = ../.;

  buildType = "debug";

  nativeBuildInputs = [
    pkg-config
    clang
  ];

  LIBCLANG_PATH = "${libclang.lib}/lib";

  cargoLock.lockFile = ../Cargo.lock;

  rustToolchain = rustToolchain;

  meta = with lib; {
    description = "A tiny, beautiful terminal markdown editor ";
    homepage = "https://github.com/ShamanicArts/poneglyph";
    license = licenses.mit;
    maintainers = with maintainers; [ Immelancholy ];
    mainProgram = "poneglyph";
  };
}
