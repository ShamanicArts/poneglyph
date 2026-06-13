{
  rustToolchain,
  writeShellApplication,
}:
writeShellApplication {
  name = "poneglyph";
  runtimeInputs = [
    rustToolchain
  ];
  text = ''
    cargo run
  '';
}
