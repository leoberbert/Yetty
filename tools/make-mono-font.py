#!/usr/bin/env python3
"""Builds assets/fonts/yetty-mono.ttf, the terminal font Yetty uses on Linux when Consolas is not installed.

On Windows the terminal is drawn in Consolas, whose cells are 0.550 em wide. Consolas is Microsoft's and cannot
be shipped, and the monospace fonts that come with egui (Hack, 0.602 em) or Linux (DejaVu Sans Mono, 0.602 em)
give 9% fewer columns per line at the same size. This script takes Hack (MIT / Bitstream Vera licence, see
HACK-LICENSE.txt) and makes the em bigger by 1 / 0.913 without touching the outlines, so every glyph and
every advance is 0.913 times the original: cells of 0.550 em, and capital letters 0.67 em tall (Consolas: 0.64).
The hinting programs depend on the em size and are removed; the font is renamed because it is a modified version.

Usage (needs `pip install fonttools`; Hack-Regular.ttf is in the epaint_default_fonts crate, or at
https://github.com/source-foundry/Hack):
    python3 tools/make-mono-font.py Hack-Regular.ttf assets/fonts/yetty-mono.ttf
"""
import sys

from fontTools.ttLib import TTFont, newTable
from fontTools.ttLib.tables import ttProgram

CONSOLAS_ADVANCE = 1126 / 2048  # em
SCALE = CONSOLAS_ADVANCE / (1233 / 2048)  # Hack's advance is 1233 / 2048 em


def main(src: str, out: str):
    font = TTFont(src)
    upem = font["head"].unitsPerEm
    font["head"].unitsPerEm = round(upem / SCALE)
    # vertical metrics are in font units too: they shrink with the em, which is what is wanted
    for table in ("fpgm", "prep", "cvt ", "gasp"):
        if table in font:
            del font[table]
    for name in font["glyf"].keys():
        glyph = font["glyf"][name]
        if hasattr(glyph, "program"):
            glyph.program = ttProgram.Program()
            glyph.program.fromBytecode(b"")
    names = font["name"]
    for record in list(names.names):
        if record.nameID in (1, 3, 4, 6, 16):
            names.removeNames(nameID=record.nameID)
    names.setName("Yetty Mono", 1, 3, 1, 0x409)
    names.setName("Regular", 2, 3, 1, 0x409)
    names.setName("Yetty Mono (modified Hack)", 3, 3, 1, 0x409)
    names.setName("Yetty Mono", 4, 3, 1, 0x409)
    names.setName("YettyMono-Regular", 6, 3, 1, 0x409)
    font.save(out)
    print(f"wrote {out}: unitsPerEm {upem} -> {font['head'].unitsPerEm}")


if __name__ == "__main__":
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    main(sys.argv[1], sys.argv[2])
