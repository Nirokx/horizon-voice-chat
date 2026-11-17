# Vendored libopus

The Opus codec, compiled into the mod by `../../build.rs`, so voice chat needs
no system libopus: Android (Starling) has no `opus` package to install, and
the mod used to fail there with `libopus not found`.

- **Version:** libopus 1.5.2, the sources as released by Xiph.Org
  (https://downloads.xiph.org/releases/opus/opus-1.5.2.tar.gz), byte-identical
  to that tarball for the files kept here.
- **What is kept:** the float build only, as libopus builds it by default:
  CELT, SILK (float) and the Opus layer, 137 `.c` files, plus the headers
  they include. Left out: the deep PLC network (`dnn/`, off by default in
  libopus), the DRED and OSCE sources, the SIMD variants (plain C builds for
  every target), the tests, the demos and the build files.
- **License:** BSD, see `COPYING` (Xiph.Org Foundation, Jean-Marc Valin and
  the other authors named there). The mod's own MIT license does not cover
  this directory.
- **Updating:** replace this directory with the same subset of a newer
  libopus release, and update the `PACKAGE_VERSION` in `../build.rs` and the
  version named here.
