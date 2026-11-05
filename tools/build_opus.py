"""Build opus.dll (Windows x86_64) from the official libopus source with MinGW gcc.

    python tools/build_opus.py <opus-x.y.z source dir> <gcc.exe> <out dir>

Plain C, floating point, no SIMD, no deep-learning features: the same codec the
Simple Voice Chat Java mod uses. libgcc is linked in statically, so the DLL needs
nothing but Windows itself. libopus is BSD-3-Clause (its COPYING goes next to the DLL).
"""
import re
import shutil
import subprocess
import sys
from pathlib import Path


def sources(src: Path, mk: str, var: str) -> list[str]:
    """The files listed under `var = \\` in one of libopus' *_sources.mk files."""
    text = (src / mk).read_text(encoding="utf-8").replace("\\\n", " ")
    m = re.search(rf"^{var}\s*=(.*)$", text, re.M)
    if not m:
        sys.exit(f"{var} not found in {mk}")
    return m.group(1).split()


def main() -> None:
    src, gcc, out = Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3])
    version = src.name.removeprefix("opus-")
    files = (
        sources(src, "celt_sources.mk", "CELT_SOURCES")
        + sources(src, "silk_sources.mk", "SILK_SOURCES")
        + sources(src, "silk_sources.mk", "SILK_SOURCES_FLOAT")
        + sources(src, "opus_sources.mk", "OPUS_SOURCES")
        + sources(src, "opus_sources.mk", "OPUS_SOURCES_FLOAT")
    )
    obj = out / "obj"
    obj.mkdir(parents=True, exist_ok=True)
    flags = [
        "-O2", "-std=gnu99", "-fvisibility=hidden",
        "-DOPUS_BUILD", "-DDLL_EXPORT", "-DUSE_ALLOCA", "-DHAVE_LRINT", "-DHAVE_LRINTF",
        f'-DPACKAGE_VERSION="{version}"',
        "-I" + str(src / "include"), "-I" + str(src / "celt"),
        "-I" + str(src / "silk"), "-I" + str(src / "silk" / "float"), "-I" + str(src),
    ]
    objects = []
    for f in files:
        o = obj / (f.replace("/", "_").removesuffix(".c") + ".o")
        subprocess.run([str(gcc), *flags, "-c", str(src / f), "-o", str(o)], check=True)
        objects.append(str(o))
    dll = out / "opus.dll"
    subprocess.run(
        [str(gcc), "-shared", "-static-libgcc", "-s", "-o", str(dll), *objects,
         f"-Wl,--out-implib,{out / 'libopus.dll.a'}"],
        check=True,
    )
    shutil.copy(src / "COPYING", out / "COPYING.libopus")
    print(f"{dll} ({dll.stat().st_size} bytes) from {len(files)} files, libopus {version}")


if __name__ == "__main__":
    main()
