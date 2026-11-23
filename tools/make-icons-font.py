#!/usr/bin/env python3
"""Builds assets/fonts/yetty-icons.ttf, the icon font Yetty uses on Linux.

On Windows the toolbar and menu icons are glyphs of the system font "Segoe MDL2 Assets" (private-use code
points U+E7xx-U+E9xx). That font is Microsoft's and cannot be shipped with the program, so on Linux the
same code points are served from this font, which holds glyphs of Microsoft's open-source Fluent System
Icons (MIT, see FLUENT-ICONS-LICENSE.txt) placed at the very same code points. The Rust code is the same
on both systems: it asks for U+E713 and gets the gear from whichever font is available.

If the machine has the real font (segmdl2.ttf, "Segoe MDL2 Assets" or "Segoe Fluent Icons") in a font folder or
in <config dir>/fonts, Yetty uses it instead of this one, and the icons are then exactly the Windows ones.

Usage (needs `pip install fonttools`):
    curl -LO https://github.com/microsoft/fluentui-system-icons/raw/main/fonts/FluentSystemIcons-Regular.ttf
    curl -LO https://github.com/microsoft/fluentui-system-icons/raw/main/fonts/FluentSystemIcons-Regular.json
    curl -LO https://github.com/microsoft/fluentui-system-icons/raw/main/fonts/FluentSystemIcons-Filled.ttf
    curl -LO https://github.com/microsoft/fluentui-system-icons/raw/main/fonts/FluentSystemIcons-Filled.json
    python3 tools/make-icons-font.py <folder with the four files> assets/fonts/yetty-icons.ttf
"""
import json
import sys
from pathlib import Path

from fontTools.fontBuilder import FontBuilder
from fontTools.pens.transformPen import TransformPen
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.ttLib import TTFont

# MDL2 code point -> (Segoe MDL2 name, Fluent icon name, style). Every code point the program draws.
MAP = {
    0xE70B: ("QuickNote", "notepad", "Regular"),
    0xE70D: ("ChevronDown", "chevron_down", "Regular"),
    0xE710: ("Add", "add", "Regular"),
    0xE711: ("Cancel", "dismiss", "Regular"),
    0xE713: ("Setting", "settings", "Regular"),
    0xE718: ("Pin", "pin", "Regular"),
    0xE71B: ("Link", "link", "Regular"),
    0xE721: ("Search", "search", "Regular"),
    0xE749: ("Print", "print", "Regular"),
    0xE756: ("CommandPrompt", "window_console", "Regular"),
    0xE765: ("KeyboardClassic", "keyboard", "Regular"),
    0xE77F: ("Paste", "clipboard_paste", "Regular"),
    0xE839: ("Ethernet", "plug_connected", "Regular"),
    0xE840: ("Pinned", "pin", "Filled"),
    0xE897: ("Help", "question_circle", "Regular"),
    0xE8A7: ("OpenInNewWindow", "open", "Regular"),
    0xE8C6: ("Cut", "cut", "Regular"),
    0xE8C8: ("Copy", "copy", "Regular"),
    0xE8CD: ("DisconnectDrive", "plug_disconnected", "Regular"),
    0xE8D7: ("Permissions", "key", "Regular"),
    0xE8EE: ("RepeatAll", "arrow_repeat_all", "Regular"),
    0xE8F4: ("NewFolder", "folder_add", "Regular"),
    0xE945: ("LightningBolt", "flash", "Regular"),
}

UPEM = 2048
SIZE = 20  # Fluent size variant used (a 20 px design grid; the icon itself lives in a 450 unit box of the 500 unit em)
GRID = 450
# Placement inside the em box (fractions of an em), measured against the Windows rendering: the picture is
# centred on the row (ASCENT - 0.5 above the baseline) and drawn ICON_EM wide, as the Segoe icons are.
ASCENT, DESCENT = 0.80, 0.20
ICON_EM = 1.10


def main(src: Path, out: Path):
    fonts = {}
    for style in ("Regular", "Filled"):
        ttf = TTFont(src / f"FluentSystemIcons-{style}.ttf")
        names = json.loads((src / f"FluentSystemIcons-{style}.json").read_text())
        fonts[style] = (ttf, ttf.getGlyphSet(), ttf.getBestCmap(), names)

    order, cmap, glyphs, metrics = [".notdef"], {}, {}, {}
    empty = TTGlyphPen(None).glyph()
    glyphs[".notdef"] = empty
    metrics[".notdef"] = (UPEM, 0)

    for cp, (mdl2, fluent, style) in sorted(MAP.items()):
        ttf, gset, fcmap, names = fonts[style]
        key = f"ic_fluent_{fluent}_{SIZE}_{style.lower()}"
        gname = fcmap[names[key]]
        # source units: the picture sits in a GRID unit box; ICON_EM em is that box
        scale = ICON_EM * UPEM / GRID
        # the box of the picture is centred on the em box horizontally and on the row vertically
        centre = GRID / 2 * scale
        dx = UPEM / 2 - centre
        dy = (ASCENT - 0.5) * UPEM - centre
        pen = TTGlyphPen(None)
        gset[gname].draw(TransformPen(pen, (scale, 0, 0, scale, dx, dy)))
        glyph = pen.glyph()
        glyph.recalcBounds(None)
        name = f"uni{cp:04X}"
        order.append(name)
        cmap[cp] = name
        glyphs[name] = glyph
        metrics[name] = (UPEM, glyph.xMin)
        print(f"U+{cp:04X} {mdl2:16s} <- {key}")

    fb = FontBuilder(UPEM, isTTF=True)
    fb.setupGlyphOrder(order)
    fb.setupCharacterMap(cmap)
    fb.setupGlyf(glyphs)
    fb.setupHorizontalMetrics(metrics)
    fb.setupHorizontalHeader(ascent=int(ASCENT * UPEM), descent=-int(DESCENT * UPEM))
    fb.setupNameTable(
        {
            "familyName": "Yetty Icons",
            "styleName": "Regular",
            "uniqueFontIdentifier": "Yetty Icons (Fluent System Icons, MIT)",
            "fullName": "Yetty Icons",
            "psName": "YettyIcons-Regular",
            "licenseDescription": "MIT License, Copyright (c) 2020 Microsoft Corporation (Fluent System Icons)",
        }
    )
    fb.setupOS2(
        sTypoAscender=int(ASCENT * UPEM),
        sTypoDescender=-int(DESCENT * UPEM),
        usWinAscent=int(ASCENT * UPEM),
        usWinDescent=int(DESCENT * UPEM),
    )
    fb.setupPost()
    fb.save(out)
    print(f"wrote {out} ({out.stat().st_size} bytes)")


if __name__ == "__main__":
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    main(Path(sys.argv[1]), Path(sys.argv[2]))
