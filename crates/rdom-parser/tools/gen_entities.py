#!/usr/bin/env python3
"""Regenerate `src/entities.rs` from the WHATWG named character reference
table.

    curl -sL https://html.spec.whatwg.org/entities.json > /path/to/entities.json
    python3 tools/gen_entities.py /path/to/entities.json > src/entities.rs

The table is the normative list from HTML §13.5 (2 231 entries, 106 of
them legacy names without a trailing `;`). Keys are stored without the
leading `&`, sorted bytewise so the decoder can binary-search prefixes.
"""
import json
import sys

src = sys.argv[1]
data = json.load(open(src, encoding="utf-8"))
rows = sorted((k[1:], v["characters"]) for k, v in data.items())
legacy = sum(1 for k, _ in rows if not k.endswith(";"))

def rs_str(s: str) -> str:
    out = []
    for ch in s:
        cp = ord(ch)
        if ch == '"' or ch == "\\":
            out.append("\\" + ch)
        elif 0x20 <= cp < 0x7F:
            out.append(ch)
        else:
            out.append(f"\\u{{{cp:X}}}")
    return '"' + "".join(out) + '"'

print("//! HTML §13.5 named character references — GENERATED, do not edit.")
print("//!")
print(f"//! Source: <https://html.spec.whatwg.org/entities.json> ({len(rows)} entries,")
print(f"//! {legacy} legacy names without `;`). Regenerate with")
print("//! `tools/gen_entities.py`. Keys omit the leading `&` and are sorted")
print("//! bytewise; `parser::named_reference` binary-searches them.")
print()
print("/// `(name, replacement)` pairs, sorted by `name`. Names that end in")
print("/// `;` are the modern form; the others are the legacy no-semicolon")
print("/// references that HTML still decodes in text (with the attribute")
print("/// caveat in §13.2.5.73).")
print("#[rustfmt::skip]")
print(f"pub(crate) static NAMED_REFERENCES: [(&str, &str); {len(rows)}] = [")
for k, v in rows:
    print(f"    ({rs_str(k)}, {rs_str(v)}),")
print("];")
print()
print("/// Longest table name, in bytes (bounds the prefix scan).")
print(f"pub(crate) const LONGEST_NAME: usize = {max(len(k) for k, _ in rows)};")
print()
print("/// Longest legacy (no `;`) name, in bytes: bounds the prefix loop for")
print("/// references that did not match with a semicolon.")
print(f"pub(crate) const LONGEST_LEGACY_NAME: usize = {max(len(k) for k, _ in rows if not k.endswith(';'))};")
