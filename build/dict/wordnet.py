#!/usr/bin/env python3
"""WordNet 3.0 -> dict.tsv (word<TAB>definition, \\n escaped, sorted). Usage: wordnet.py WordNet-3.0/dict out.tsv"""
import sys, re, os
d, out = sys.argv[1], sys.argv[2]
POS = {"noun": "noun", "verb": "verb", "adj": "adjective", "adv": "adverb"}
MAXS = 6  # senses per part of speech
gl = {}
for p in POS:
    for line in open(f"{d}/data.{p}", encoding="utf8"):
        if line.startswith("  "): continue
        head, _, g = line.rstrip("\n").partition(" | ")
        parts = head.split()
        # strip examples: definition is before the first '; "'
        defn, _, rest = g.partition('; "')
        ex = rest.split('"')[0] if rest else ""
        gl[(p, parts[0])] = (defn.strip(), ex)
entries = {}
for p, name in POS.items():
    for line in open(f"{d}/index.{p}", encoding="utf8"):
        if line.startswith("  "): continue
        f = line.split()
        lemma, cnt, pc = f[0], int(f[2]), int(f[3])
        offs = f[6 + pc:]
        if "_" in lemma: continue
        senses = []
        for o in offs[:MAXS]:
            df, ex = gl[(p, o)]
            s = df + (f' — “{ex}”' if ex else "")
            senses.append(s)
        txt = f"({name})\n" + "\n".join(f"{i+1}. {s}" for i, s in enumerate(senses))
        entries.setdefault(lemma.lower(), []).append(txt)
res = {k: "\n".join(v) for k, v in entries.items()}
# irregular forms (went -> go): point at the base word's entry
for p in POS:
    fn = f"{d}/{p}.exc"
    if not os.path.exists(fn): continue
    for line in open(fn, encoding="utf8"):
        f = line.split()
        if len(f) >= 2 and "_" not in f[0] and f[0].lower() not in res and f[1].lower() in res:
            res[f[0].lower()] = f"form of “{f[1]}”\n" + res[f[1].lower()]
with open(out, "w", encoding="utf8") as o:
    for k in sorted(res):
        o.write(k + "\t" + res[k].replace("\n", "\\n") + "\n")
print(len(res), "words")
