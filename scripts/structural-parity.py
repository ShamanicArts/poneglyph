#!/usr/bin/env python3
import argparse, html, json, subprocess
from pathlib import Path

LINE_KIND_MAP = {
    "Empty": "empty",
    "Text": "text",
    "Blockquote": "blockquote",
    "UnorderedList": "unorderedList",
    "OrderedList": "orderedList",
    "CodeBlock": "codeBlock",
    "HorizontalRule": "horizontalRule",
    "Table": "table",
}

ACCEPTED_CLASSIFY = {
    ("text", "table"): "accepted enhancement: Rust recognizes pipe tables structurally; Bun preview treats them as text",
}

parser = argparse.ArgumentParser(description="Compare original Bun markdown structure oracle against Rust")
parser.add_argument("fixtures", nargs="+", help="fixture markdown files")
parser.add_argument("--out", type=Path, default=Path("proofs/structural-parity.html"))
args = parser.parse_args()
root = Path(__file__).resolve().parents[1]

def run_json(cmd):
    return json.loads(subprocess.check_output(cmd, cwd=root, text=True))

def rust_kind(raw):
    kind = raw["kind"]
    if isinstance(kind, dict) and "Heading" in kind:
        return f"heading{kind['Heading']}"
    return LINE_KIND_MAP.get(kind, kind)

def classify_fixture(path: Path):
    bun = run_json(["bun", "run", str(root / "scripts/bun-oracle.ts"), "classify", str(path)])
    rust = run_json([str(root / "target/release/md-editor-rust"), "classify", str(path)])
    lines = path.read_text().split("\n")
    max_len = max(len(bun), len(rust))
    rows = []
    counts = {"match": 0, "accepted": 0, "diff": 0}
    for i in range(max_len):
        bun_type = bun[i]["lineType"] if i < len(bun) else "<missing>"
        rust_type = rust_kind(rust[i]) if i < len(rust) else "<missing>"
        source = lines[i] if i < len(lines) else ""
        accepted = ACCEPTED_CLASSIFY.get((bun_type, rust_type))
        same = bun_type == rust_type
        status = "match" if same else "accepted" if accepted else "diff"
        counts[status] += 1
        rows.append({
            "line": i + 1,
            "source": source,
            "bun": bun_type,
            "rust": rust_type,
            "status": status,
            "note": accepted or "",
        })
    return rows, counts

def outline_fixture(path: Path):
    bun = run_json(["bun", "run", str(root / "scripts/bun-oracle.ts"), "outline", str(path)])
    rust = run_json([str(root / "target/release/md-editor-rust"), "outline", str(path)])
    max_len = max(len(bun), len(rust))
    rows = []
    counts = {"match": 0, "diff": 0}
    for i in range(max_len):
        left = bun[i] if i < len(bun) else None
        right = rust[i] if i < len(rust) else None
        same = left == right
        counts["match" if same else "diff"] += 1
        rows.append({"index": i + 1, "bun": left, "rust": right, "status": "match" if same else "diff"})
    return rows, counts

reports = []
for fixture in args.fixtures:
    path = Path(fixture)
    if not path.is_absolute():
        path = root / path
    classify_rows, classify_counts = classify_fixture(path)
    outline_rows, outline_counts = outline_fixture(path)
    reports.append({
        "fixture": str(path),
        "classifyRows": classify_rows,
        "classifyCounts": classify_counts,
        "outlineRows": outline_rows,
        "outlineCounts": outline_counts,
    })

args.out.parent.mkdir(parents=True, exist_ok=True)
json_out = args.out.with_suffix(".json")
txt_out = args.out.with_suffix(".txt")
json_out.write_text(json.dumps(reports, indent=2))
with txt_out.open("w") as f:
    for report in reports:
        f.write(f"Fixture: {report['fixture']}\n")
        f.write(f"Classify: {report['classifyCounts']}\n")
        f.write(f"Outline:  {report['outlineCounts']}\n\n")
        f.write("Classification rows:\n")
        for row in report["classifyRows"]:
            mark = " " if row["status"] == "match" else "~" if row["status"] == "accepted" else "!"
            note = f" ({row['note']})" if row["note"] else ""
            f.write(f"{row['line']:>3}{mark} bun={row['bun']:<15} rust={row['rust']:<15} src={row['source']!r}{note}\n")
        f.write("\nOutline rows:\n")
        for row in report["outlineRows"]:
            mark = " " if row["status"] == "match" else "!"
            f.write(f"{row['index']:>3}{mark} bun={row['bun']} rust={row['rust']}\n")
        f.write("\n" + "=" * 120 + "\n\n")

sections = []
for report in reports:
    classify = "\n".join(
        f"<tr class='{row['status']}'><td>{row['line']}</td><td><code>{html.escape(row['source'])}</code></td><td>{row['bun']}</td><td>{row['rust']}</td><td>{html.escape(row['note'])}</td></tr>"
        for row in report["classifyRows"]
    )
    outline = "\n".join(
        f"<tr class='{row['status']}'><td>{row['index']}</td><td><pre>{html.escape(json.dumps(row['bun']))}</pre></td><td><pre>{html.escape(json.dumps(row['rust']))}</pre></td></tr>"
        for row in report["outlineRows"]
    )
    sections.append(f"""
<section>
<h2>{html.escape(report['fixture'])}</h2>
<p><b>Classify</b> {html.escape(str(report['classifyCounts']))} &nbsp; <b>Outline</b> {html.escape(str(report['outlineCounts']))}</p>
<h3>Classification</h3>
<table><thead><tr><th>Line</th><th>Source</th><th>Bun</th><th>Rust</th><th>Note</th></tr></thead><tbody>{classify}</tbody></table>
<h3>Outline</h3>
<table><thead><tr><th>#</th><th>Bun</th><th>Rust</th></tr></thead><tbody>{outline}</tbody></table>
</section>
""")
args.out.write_text(f"""<!doctype html><html><head><meta charset='utf-8'><title>Structural Parity</title>
<style>
body {{ background:#111318; color:#d8dee8; font-family:Inter, system-ui, sans-serif; margin:24px; }}
h1,h2 {{ color:#4ec9b0; }} h3 {{ color:#9cdcfe; }}
table {{ border-collapse:collapse; width:100%; margin-bottom:24px; table-layout:fixed; }}
th {{ color:#9cdcfe; border-bottom:1px solid #3a4759; text-align:left; padding:6px; }}
td {{ border-bottom:1px solid #222b37; padding:5px 6px; vertical-align:top; }}
code, pre {{ font-family:'JetBrains Mono','Fira Code',monospace; font-size:13px; white-space:pre-wrap; margin:0; }}
.diff {{ background:rgba(206,111,124,.16); }}
.accepted {{ background:rgba(201,168,106,.15); }}
.match {{ background:transparent; }}
.legend {{ color:#c9a86a; }}
</style></head><body>
<h1>Bun/Rust Structural Parity</h1>
<p class='legend'>Red = unaccepted parity mismatch. Gold = accepted enhancement/divergence.</p>
{''.join(sections)}
</body></html>""")
print(json.dumps({"html": str(args.out), "text": str(txt_out), "json": str(json_out), "fixtures": len(reports)}, indent=2))
