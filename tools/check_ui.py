#!/usr/bin/env python3
"""Static checks for the Transmutation Table screen.

No Rust toolchain exists in the environment this screen was written in, so these
scripts stand in for the checks a compiler would do. They are not a substitute
for `cargo build` and `cargo test`; they catch the classes of error that are
mechanical to detect, and nothing more.

Usage:  python3 tools/check_ui.py [path/to/pinned/horizon/source]

With no path, the Horizon-source checks are skipped and only the in-repo ones
run (which need nothing but the standard library plus tree_sitter for the
parse).
"""
import json
import os
import re
import sys
import glob

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
UI = glob.glob(os.path.join(ROOT, "engine/src/ui/*.rs"))
ALL = glob.glob(os.path.join(ROOT, "engine/src/**/*.rs"), recursive=True)

fails = []


def check(name, ok, detail=""):
    print(("PASS  " if ok else "FAIL  ") + name + (("  " + detail) if detail and not ok else ""))
    if not ok:
        fails.append(name)


def read(path):
    with open(path, encoding="utf-8") as f:
        return f.read()


# --- 1. every file parses ---------------------------------------------------
def parses():
    try:
        import tree_sitter_rust as tsr
        from tree_sitter import Language, Parser
    except ImportError:
        print("SKIP  tree-sitter parse (tree_sitter not installed)")
        return
    parser = Parser(Language(tsr.language()))
    bad = []
    for path in ALL:
        if parser.parse(open(path, "rb").read()).root_node.has_error:
            bad.append(os.path.relpath(path, ROOT))
    check("all engine/src/**/*.rs parse (%d files)" % len(ALL), not bad, ", ".join(bad))


# --- 2. the layout is pure geometry -----------------------------------------
def layout_is_pure():
    src = read(os.path.join(ROOT, "engine/src/ui/layout.rs"))
    check("layout.rs is pure geometry (no horizon, no drawing)",
          "horizon" not in src and "mod_api" not in src)


def no_cycles():
    layout = read(os.path.join(ROOT, "engine/src/ui/layout.rs"))
    atlas = read(os.path.join(ROOT, "engine/src/ui/atlas.rs"))
    # A doc-comment cross-reference is fine; a `use` or a call is not.
    code = re.sub(r"//[^\n]*", "", layout)  # drop comments first
    uses_atlas = re.search(r"\batlas\s*::", code) or \
        re.search(r"\buse\s+[^;]*\batlas\b", code)
    check("layout.rs does not depend on atlas.rs", uses_atlas is None)
    check("atlas.rs takes the menu's shape from layout.rs",
          "pub(crate) use super::layout::{INPUT_SLOTS, MENU_SLOTS, SLOT_COUNT, STORAGE_SLOTS}" in atlas)


# --- 3. slot counts agree ---------------------------------------------------
def slots_agree():
    layout = read(os.path.join(ROOT, "engine/src/ui/layout.rs"))
    atlas = read(os.path.join(ROOT, "engine/src/ui/atlas.rs"))
    lib = read(os.path.join(ROOT, "engine/src/lib.rs"))
    m = re.search(r"const MENU_SLOTS: usize = (\d+);", layout)
    check("layout declares MENU_SLOTS", m is not None)
    n = int(m.group(1)) if m else -1
    check("MENU_SLOTS is 9 (minecraft:generic_9x1)", n == 9, str(n))
    check("the container declares the same slot count", "slots: ui::atlas::MENU_SLOTS" in lib)
    check("the menu type is vanilla's single row",
          'const MENU: &str = "minecraft:generic_9x1";' in atlas)


# --- 4. the panel fits its parts --------------------------------------------
def geometry():
    layout = read(os.path.join(ROOT, "engine/src/ui/layout.rs"))

    def num(name, default):
        m = re.search(r"const %s: f32 = ([0-9.]+);" % name, layout)
        return float(m.group(1)) if m else default

    panel_h = num("PANEL_H", 0)
    inv_y = num("INVENTORY_Y", 0)
    check("the player's hotbar fits in the panel", inv_y + 58 + 18 <= panel_h,
          "hotbar bottom %s > panel %s" % (inv_y + 58 + 18, panel_h))
    # The engine centres with `trunc`, never rescales.
    check("Layout::fit draws at 1:1", "scale: 1.0," in layout)
    check("Layout::fit centres with trunc, as centered_origin does", ".trunc()" in layout)


# --- 5. the language tables are complete ------------------------------------
def languages():
    loc = read(os.path.join(ROOT, "engine/src/ui/localization.rs"))
    keys_block = re.search(r"pub\(crate\) const KEYS: \[&str; (\d+)\] = \[(.*?)\n\];", loc, re.S)
    check("KEYS is declared", keys_block is not None)
    if keys_block:
        declared = int(keys_block.group(1))
        listed = re.findall(r"\bKEY_[A-Z_]+\b", keys_block.group(2))
        check("KEYS length matches its list", declared == len(listed),
              "%d vs %d" % (declared, len(listed)))
        for table in ("EN_US", "RU_RU"):
            block = re.search(r"pub\(crate\) const %s:.*?= &\[(.*?)\n\];" % table, loc, re.S)
            if not block:
                check("%s is declared" % table, False)
                continue
            entries = re.findall(r"\(\s*(KEY_[A-Z_]+)", block.group(1))
            missing = sorted(set(listed) - set(entries))
            check("%s covers every key" % table, not missing, ", ".join(missing))


# --- 6. against the pinned Horizon source -----------------------------------
def horizon_source(path):
    hooks_json = os.path.join(path, "native-hooks.json")
    native_path = os.path.join(path, "crates/horizon-client/src/game/native_hooks.rs")
    if not (os.path.exists(hooks_json) and os.path.exists(native_path)):
        print("SKIP  Horizon-source checks (no native-hooks.json / native_hooks.rs under %s)" % path)
        return
    # The source text, not the path: `native_sig` searches inside it.
    native = read(native_path)
    catalog = json.load(open(hooks_json))
    hooks = {h["name"]: h for h in catalog["hooks"]}
    helpers = {h["name"]: h for h in catalog["helpers"]}
    for frag in glob.glob(os.path.join(path, "tools/native-hook-fragments/*.json")):
        data = json.load(open(frag))
        for h in data.get("hooks", []):
            hooks.setdefault(h["name"], h)
        for h in data.get("helpers", []):
            helpers.setdefault(h["name"], h)

    lib = read(os.path.join(ROOT, "engine/src/lib.rs"))
    called = set()
    for path_ in ALL:
        called |= set(re.findall(r"host\.([a-z_][a-z0-9_]*)\s*\(", read(path_)))
    missing = sorted(c for c in called if c not in helpers)
    check("every Host method called exists in the pinned catalog (%d)" % len(called),
          not missing, ", ".join(missing))

    implemented = set(re.findall(r"\n    fn ([a-z_][a-z0-9_]*)\(&self", lib))
    unknown = sorted(h for h in implemented if h not in hooks)
    check("every implemented hook exists in the pinned catalog (%d)" % len(implemented),
          not unknown, ", ".join(unknown))

    # Parameter names and types, including the out-argument rule.
    def native_sig(name):
        m = re.search(r"fn native_" + re.escape(name) + r"\s*\(", native)
        if not m:
            return None
        i = m.end() - 1
        depth = 0
        for j in range(i, len(native)):
            if native[j] == "(":
                depth += 1
            elif native[j] == ")":
                depth -= 1
                if depth == 0:
                    break
        head = re.sub(r"//.*", "", native[i + 1:j])
        parts, cur, d = [], "", 0
        for ch in head:
            if ch in "(<[":
                d += 1
            if ch in ")>]":
                d -= 1
            if ch == "," and d == 0:
                parts.append(cur.strip())
                cur = ""
            else:
                cur += ch
        if cur.strip():
            parts.append(cur.strip())
        return parts

    # The generator turns a native helper's arguments into *event fields*; the
    # trait method itself is `(&self, [host,] event)`. So compare the receiver
    # and the host's mutability and type only, and require exactly one event.
    def impl_shape(name):
        m = re.search(r"\n    fn " + re.escape(name) + r"\((.*?)\)\s*(->\s*[^\{]+?)?\s*\{",
                      lib, re.S)
        if not m:
            return None
        raw = re.sub(r"//.*", "", m.group(1))
        pieces, cur, d = [], "", 0
        for ch in raw:
            if ch in "(<[{":
                d += 1
            elif ch in ")>]}":
                d -= 1
            if ch == "," and d == 0:
                pieces.append(cur)
                cur = ""
            else:
                cur += ch
        if cur.strip():
            pieces.append(cur)
        self_mut, host, rest = False, None, []
        for piece in pieces:
            piece = piece.strip()
            if piece.replace(" ", "") == "&self":
                continue
            if piece.replace(" ", "") == "&mutself":
                self_mut = True
                continue
            if ":" in piece:
                k, v = piece.split(":", 1)
                v = re.sub(r"\s+", "", v)
                key = k.strip().lstrip("_")
                if key == "host":
                    host = v
                elif key == "event":
                    pass  # the event itself, not a separate argument
                else:
                    rest.append(k.strip())
        return self_mut, host, rest

    def native_shape(name):
        args = native_sig(name)
        if args is None:
            return None
        if args and args[0].replace(" ", "").startswith("&"):
            recv = args[0].replace(" ", "")
            mutable = recv.startswith("&mut")
            rest = args[1:]
        else:
            mutable, rest = None, args
        out = hooks[name]["mod_api"].get("out")
        if out:
            rest = [a for a in rest if a.split(":")[0].strip() != out]
        return mutable, rest

    bad = []
    for name in sorted(implemented & set(hooks)):
        shape = impl_shape(name)
        if shape is None:
            bad.append(name + " (impl unparsed)")
            continue
        self_mut, host, fields = shape
        want = native_shape(name)
        if want is None:
            bad.append(name + " (no native fn)")
            continue
        mutable, native_args = want
        if self_mut:
            bad.append(name + " (impl takes &mut self)")
        if mutable is None:
            if host is not None:
                bad.append(name + " (native takes no host)")
        elif mutable:
            if host != "&mutdynHost":
                bad.append(name + " (host is %s, native wants &mut dyn Host)" % host)
        elif host != "&dynHost":
            bad.append(name + " (host is %s, native wants &dyn Host)" % host)
        # Every native argument becomes one event field, and the impl takes
        # exactly one event: no argument may be a separate parameter.
        if fields:
            bad.append(name + " (takes arguments outside the event: %s)" % ", ".join(fields))

    check("every hook's parameters match the pinned signature", not bad, "; ".join(bad))


def main():
    parses()
    layout_is_pure()
    no_cycles()
    slots_agree()
    geometry()
    languages()
    if len(sys.argv) > 1:
        horizon_source(sys.argv[1])
    else:
        print("SKIP  Horizon-source checks (pass the pinned source's path)")
    print()
    if fails:
        print("%d check(s) failed:" % len(fails))
        for f in fails:
            print("  - " + f)
        return 1
    print("all checks passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
