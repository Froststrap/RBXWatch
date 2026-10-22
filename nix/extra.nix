{
  typos,
  callPackage,
}:
let
  inherit (callPackage ./devshell-tools.nix { }) mkFragment;
in
mkFragment {
  name = "extra";
  buildInputs = [
    typos
  ];
}
