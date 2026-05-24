#!/usr/bin/env python3
"""Tiny progress dashboard for eval runs. Hit http://localhost:8777"""

import glob
import http.server
import json
import time
from pathlib import Path

EVALS = [
    {"name": "wmdp_bio", "log_dir": "data/v2/logs_bio", "bf16": 2040, "fp8": 2056},
    {"name": "wmdp_chem", "log_dir": "data/v2/logs_chem", "bf16": 2040, "fp8": 2056},
    {"name": "truthfulqa_mc1", "log_dir": "data/v2/logs_truthfulqa", "bf16": 2040, "fp8": 2056},
]


RATE_WINDOW_S = 300


def get_eval_stats(ev: dict) -> dict:
    cells = glob.glob(f"{ev['log_dir']}/c*/*.eval")
    n = len(cells)
    total = ev["bf16"] + ev["fp8"]
    elapsed = ""
    rate = 0.0
    eta = "?"
    if cells:
        now = time.time()
        mtimes = [Path(c).stat().st_mtime for c in cells]
        first = min(mtimes)
        last = max(mtimes)
        elapsed_s = last - first
        elapsed = f"{int(elapsed_s // 3600)}h {int((elapsed_s % 3600) // 60)}m"
        cutoff = now - RATE_WINDOW_S
        recent = sum(1 for t in mtimes if t >= cutoff)
        window_s = min(RATE_WINDOW_S, now - cutoff)
        rate = recent / (window_s / 60) if window_s > 0 else 0
        remaining = total - n
        eta_min = remaining / rate if rate > 0 else 0
        eta = f"{int(eta_min // 60)}h {int(eta_min % 60)}m"
    return {
        "name": ev["name"],
        "completed": n,
        "bf16": ev["bf16"],
        "fp8": ev["fp8"],
        "total": total,
        "pct": round(100 * n / total, 1) if total else 0,
        "elapsed": elapsed,
        "rate": round(rate, 1),
        "eta": eta,
    }


def get_all_stats() -> dict:
    evals = [get_eval_stats(ev) for ev in EVALS]
    total_done = sum(e["completed"] for e in evals)
    total_cells = sum(e["total"] for e in evals)
    return {
        "evals": evals,
        "total_completed": total_done,
        "total_cells": total_cells,
        "total_pct": round(100 * total_done / total_cells, 1) if total_cells else 0,
        "time": time.strftime("%H:%M:%S"),
    }


COLORS = {"wmdp_bio": "#4ecca3", "wmdp_chem": "#e8a838", "truthfulqa_mc1": "#5b8dee"}


class Handler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == "/json":
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps(get_all_stats(), indent=2).encode())
            return

        s = get_all_stats()
        rows = ""
        bars = ""
        for ev in s["evals"]:
            color = COLORS.get(ev["name"], "#4ecca3")
            status = (
                "done"
                if ev["completed"] >= ev["total"]
                else ("running" if ev["completed"] > 0 else "pending")
            )
            rows += f"""<tr>
<td style="color:{color}">{ev["name"]}</td>
<td>{ev["completed"]} / {ev["total"]}</td>
<td>{ev["pct"]}%</td>
<td>{ev["rate"]} cells/min</td>
<td>{ev["elapsed"] or "-"}</td>
<td>{ev["eta"]}</td>
<td>{status}</td>
</tr>"""
            bars += f"""<div style="margin-bottom:8px">
<div style="color:{color};margin-bottom:2px">{ev["name"]}</div>
<div class="bar"><div class="fill" style="background:{color};width:{int(ev["pct"])}%"></div></div>
</div>"""

        html = f"""<html><head>
<title>mojave tier-1 dashboard</title>
<meta http-equiv="refresh" content="3">
<style>
body {{ font-family: monospace; background: #1a1a2e; color: #e0e0e0; padding: 40px; }}
.bar {{ background: #333; border-radius: 4px; height: 24px; width: 100%; max-width: 600px; }}
.fill {{ height: 100%; border-radius: 4px; transition: width 1s; }}
h1 {{ color: #4ecca3; }}
.big {{ font-size: 48px; color: #4ecca3; }}
table {{ border-collapse: collapse; }}
table td, table th {{ padding: 6px 16px 6px 0; text-align: left; }}
th {{ color: #888; border-bottom: 1px solid #333; }}
</style></head><body>
<h1>mojave &mdash; tier-1 MCQ dashboard</h1>
<div class="big">{s["total_completed"]} / {s["total_cells"]}</div>
<p>Overall: {s["total_pct"]}% &middot; Updated {s["time"]}</p>
{bars}
<br>
<table>
<tr><th>Eval</th><th>Progress</th><th>%</th><th>Rate</th><th>Elapsed</th><th>ETA</th><th>Status</th></tr>
{rows}
</table>
<p style="color:#666">auto-refreshes every 3s &middot; /json for raw data</p>
</body></html>"""
        self.send_response(200)
        self.send_header("Content-Type", "text/html")
        self.end_headers()
        self.wfile.write(html.encode())

    def log_message(self, format, *args):  # noqa: A002
        pass


if __name__ == "__main__":
    server = http.server.HTTPServer(("0.0.0.0", 8777), Handler)
    print("Progress dashboard: http://localhost:8777", flush=True)
    server.serve_forever()
