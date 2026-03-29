#!/usr/bin/env python3
import argparse
import json
import re
import subprocess
import sys
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import Callable, Dict, List, Tuple

PROMPT_TOKEN = "arookieofcDB> "
ELAPSED_RE = re.compile(r"^\(elapsed:\s+.+\)$")
VERIFY_RE = re.compile(
    r"issues:\s*total=(\d+),\s*only_in_cache=(\d+),\s*only_in_disk=(\d+),\s*value_mismatch=(\d+)"
)


@dataclass
class Scenario:
    name: str
    commands: List[str]
    check: Callable[[dict], Tuple[bool, List[str]]]


def parse_status_line(line: str) -> Dict[str, str]:
    out: Dict[str, str] = {}
    for part in line.split(", "):
        if "=" in part:
            k, v = part.split("=", 1)
            out[k.strip()] = v.strip()
    return out


def normalize_lines(raw: str) -> List[str]:
    lines: List[str] = []
    for ln in raw.splitlines():
        ln = ln.replace(PROMPT_TOKEN, "").rstrip("\r")
        if ln.startswith("arookieofcDB CLI v"):
            continue
        if ln.startswith("type `help` to see commands"):
            continue
        if not ln.strip():
            continue
        lines.append(ln)
    return lines


def chunk_by_elapsed(lines: List[str]) -> Tuple[List[List[str]], List[str]]:
    chunks: List[List[str]] = []
    cur: List[str] = []
    prelude: List[str] = []
    seen_elapsed = False

    for ln in lines:
        if ELAPSED_RE.match(ln):
            chunks.append(cur)
            cur = []
            seen_elapsed = True
            continue
        if not seen_elapsed and not cur:
            prelude.append(ln)
            continue
        cur.append(ln)

    if cur:
        chunks.append(cur)
    return chunks, prelude


def run_exe(root: Path, exe_path: Path, input_text: str) -> subprocess.CompletedProcess:
    return subprocess.run(
        [str(exe_path)],
        input=input_text,
        cwd=str(root),
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        encoding="utf-8",
        errors="replace",
    )


def clean_data(root: Path) -> None:
    for rel in ["data/wal.log", "data/wal.snap", "data/wal.tmp", "data/wal.snap.tmp"]:
        p = root / rel
        if p.exists():
            try:
                p.unlink()
            except Exception:
                pass


def run_scenario(root: Path, exe_path: Path, scenario: Scenario, out_dir: Path, isolate: bool) -> dict:
    if isolate:
        clean_data(root)

    in_path = out_dir / f"{scenario.name}.in.txt"
    out_path = out_dir / f"{scenario.name}.out.txt"

    commands = scenario.commands + ["exit"]
    input_text = "\n".join(commands) + "\n"
    in_path.write_text(input_text, encoding="utf-8")

    proc = run_exe(root, exe_path, input_text)
    raw = proc.stdout or ""
    out_path.write_text(raw, encoding="utf-8")

    lines = normalize_lines(raw)
    chunks, prelude = chunk_by_elapsed(lines)

    cmd_results = []
    for i, cmd in enumerate(commands):
        chunk = chunks[i] if i < len(chunks) else []
        cmd_results.append({"command": cmd, "output_lines": chunk, "output_text": "\n".join(chunk)})

    verify_issues = []
    status_snapshots = []
    for chunk in chunks:
        for ln in chunk:
            m = VERIFY_RE.search(ln)
            if m:
                verify_issues.append(
                    {
                        "total": int(m.group(1)),
                        "only_in_cache": int(m.group(2)),
                        "only_in_disk": int(m.group(3)),
                        "value_mismatch": int(m.group(4)),
                    }
                )
            if ln.startswith("engine="):
                status_snapshots.append(parse_status_line(ln))

    result = {
        "name": scenario.name,
        "exit_code": proc.returncode,
        "commands": commands,
        "cmd_results": cmd_results,
        "prelude": prelude,
        "verify_issues": verify_issues,
        "status_snapshots": status_snapshots,
        "raw_log": str(out_path),
    }

    ok, messages = scenario.check(result)
    result["passed"] = ok
    result["check_messages"] = messages
    return result


def as_int(d: Dict[str, str], key: str, default: int = -1) -> int:
    try:
        return int(d.get(key, str(default)))
    except Exception:
        return default


def check_hybrid_basic(r: dict) -> Tuple[bool, List[str]]:
    ok = True
    msgs: List[str] = []

    if r["exit_code"] != 0:
        ok = False
        msgs.append(f"process exit_code != 0 ({r['exit_code']})")

    base_hits = sum(1 for i in [2, 3, 4] if "base" in r["cmd_results"][i]["output_text"])
    if base_hits < 3:
        ok = False
        msgs.append("get/select/select --disk did not all contain base")

    if not r["status_snapshots"]:
        ok = False
        msgs.append("status output missing")
    elif r["status_snapshots"][-1].get("engine") != "hybrid":
        ok = False
        msgs.append(f"status engine is not hybrid: {r['status_snapshots'][-1].get('engine')}")

    return ok, msgs


def check_hybrid_off_fault(r: dict) -> Tuple[bool, List[str]]:
    ok = True
    msgs: List[str] = []

    if r["exit_code"] != 0:
        ok = False
        msgs.append(f"process exit_code != 0 ({r['exit_code']})")

    if "stale" not in r["cmd_results"][4]["output_text"]:
        ok = False
        msgs.append("repairmode off get did not return stale")

    if not r["verify_issues"]:
        ok = False
        msgs.append("verify issues not parsed")
    elif r["verify_issues"][0]["total"] < 1:
        ok = False
        msgs.append("repairmode off verify total should be >= 1")

    if not r["status_snapshots"]:
        ok = False
        msgs.append("status output missing")
    else:
        st = r["status_snapshots"][-1]
        if st.get("repair_mode") != "off":
            ok = False
            msgs.append("status repair_mode is not off")
        if as_int(st, "inconsistency_total") < 1:
            ok = False
            msgs.append("status inconsistency_total should be >= 1")

    return ok, msgs


def check_hybrid_read_fault(r: dict) -> Tuple[bool, List[str]]:
    ok = True
    msgs: List[str] = []

    if r["exit_code"] != 0:
        ok = False
        msgs.append(f"process exit_code != 0 ({r['exit_code']})")

    if "base" not in r["cmd_results"][4]["output_text"]:
        ok = False
        msgs.append("repairmode read get did not return base")

    if not r["verify_issues"] or r["verify_issues"][0]["total"] != 0:
        ok = False
        msgs.append("repairmode read verify total should be 0")

    if not r["status_snapshots"]:
        ok = False
        msgs.append("status output missing")
    else:
        st = r["status_snapshots"][-1]
        if st.get("repair_mode") != "read":
            ok = False
            msgs.append("status repair_mode is not read")
        if as_int(st, "auto_repairs_read") < 1:
            ok = False
            msgs.append("auto_repairs_read should be >= 1")

    return ok, msgs


def check_hybrid_write_fault(r: dict) -> Tuple[bool, List[str]]:
    ok = True
    msgs: List[str] = []

    if r["exit_code"] != 0:
        ok = False
        msgs.append(f"process exit_code != 0 ({r['exit_code']})")

    if len(r["verify_issues"]) < 2:
        ok = False
        msgs.append("need two verify outputs")
    else:
        if r["verify_issues"][0]["total"] < 1:
            ok = False
            msgs.append("write mode pre-write verify total should be >= 1")
        if r["verify_issues"][1]["total"] != 0:
            ok = False
            msgs.append("write mode post-write verify total should be 0")

    if not r["status_snapshots"]:
        ok = False
        msgs.append("status output missing")
    else:
        st = r["status_snapshots"][-1]
        if st.get("repair_mode") != "write":
            ok = False
            msgs.append("status repair_mode is not write")
        if as_int(st, "auto_repairs_write") < 1:
            ok = False
            msgs.append("auto_repairs_write should be >= 1")

    return ok, msgs


def check_wal_basic(r: dict) -> Tuple[bool, List[str]]:
    ok = True
    msgs: List[str] = []

    if r["exit_code"] != 0:
        ok = False
        msgs.append(f"process exit_code != 0 ({r['exit_code']})")

    for i in [2, 3, 4]:
        if "v1" not in r["cmd_results"][i]["output_text"]:
            ok = False
            msgs.append("wal get/select/select --disk did not all contain v1")
            break

    if not r["status_snapshots"]:
        ok = False
        msgs.append("status output missing")
    elif r["status_snapshots"][-1].get("engine") != "wal_bptree":
        ok = False
        msgs.append(f"status engine is not wal_bptree: {r['status_snapshots'][-1].get('engine')}")

    return ok, msgs


def build_scenarios(prefix: str, quick: bool) -> List[Scenario]:
    items = [
        Scenario(
            name="01_hybrid_basic_get_vs_select",
            commands=[
                "engine hybrid",
                f"set {prefix}:s1 base",
                f"get {prefix}:s1",
                f"select {prefix}:s1",
                f"select --disk {prefix}:s1",
                "status",
            ],
            check=check_hybrid_basic,
        ),
        Scenario(
            name="02_hybrid_fault_cache_only_repairmode_off",
            commands=[
                "engine hybrid",
                "repairmode off",
                f"set {prefix}:s2 base",
                f"fault cache-only {prefix}:s2 stale",
                f"get {prefix}:s2",
                "verify",
                "status",
            ],
            check=check_hybrid_off_fault,
        ),
        Scenario(
            name="03_hybrid_fault_cache_only_repairmode_read",
            commands=[
                "engine hybrid",
                "repairmode read",
                f"set {prefix}:s3 base",
                f"fault cache-only {prefix}:s3 stale",
                f"get {prefix}:s3",
                "verify",
                "status",
            ],
            check=check_hybrid_read_fault,
        ),
        Scenario(
            name="04_hybrid_fault_disk_only_repairmode_write",
            commands=[
                "engine hybrid",
                "repairmode write",
                f"set {prefix}:s4 base",
                f"fault disk-only {prefix}:s4 disk_new",
                "verify",
                f"set {prefix}:s4_trigger x",
                "verify",
                "status",
            ],
            check=check_hybrid_write_fault,
        ),
        Scenario(
            name="05_wal_get_select_disk",
            commands=[
                "engine wal",
                f"set {prefix}:s5 v1",
                f"get {prefix}:s5",
                f"select {prefix}:s5",
                f"select --disk {prefix}:s5",
                "status",
            ],
            check=check_wal_basic,
        ),
    ]
    return items[:2] if quick else items


def build_once(root: Path, build_cmd: str) -> int:
    proc = subprocess.run(
        build_cmd,
        cwd=str(root),
        shell=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    if proc.returncode != 0:
        print(proc.stdout)
    return proc.returncode


def write_report(report_dir: Path, exe_path: Path, build_cmd: str, results: List[dict]) -> Path:
    md_path = report_dir / "report.md"
    js_path = report_dir / "report.json"

    lines: List[str] = []
    lines.append("# Command Behavior CI Report")
    lines.append("")
    lines.append(f"- build_cmd: `{build_cmd}`")
    lines.append(f"- exe_path: `{exe_path}`")
    lines.append(f"- generated_at: `{datetime.now().isoformat(timespec='seconds')}`")
    lines.append("")

    passed = sum(1 for r in results if r["passed"])
    lines.append(f"- scenarios: `{len(results)}`")
    lines.append(f"- passed: `{passed}`")
    lines.append(f"- failed: `{len(results) - passed}`")
    lines.append("")

    for r in results:
        status = "PASS" if r["passed"] else "FAIL"
        lines.append(f"## {r['name']} [{status}]")
        lines.append(f"- exit_code: `{r['exit_code']}`")
        lines.append(f"- log: `{r['raw_log']}`")
        if r["check_messages"]:
            lines.append("- check_messages:")
            for m in r["check_messages"]:
                lines.append(f"  - {m}")
        else:
            lines.append("- check_messages: (none)")
        if r["status_snapshots"]:
            lines.append(f"- last_status: `{r['status_snapshots'][-1]}`")
        lines.append("")

    md_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    js_path.write_text(json.dumps(results, ensure_ascii=False, indent=2), encoding="utf-8")
    return md_path


def main() -> int:
    parser = argparse.ArgumentParser(description="Run command behavior checks and output report")
    parser.add_argument("--quick", action="store_true", help="run first 2 scenarios")
    parser.add_argument("--reset-data", action="store_true", help="delete data/wal.* before run")
    parser.add_argument("--build-cmd", default="cargo build --quiet", help="build command")
    parser.add_argument("--exe", default="target/debug/arookieofcDB.exe", help="cli executable")
    parser.add_argument("--report-dir", default="compares/", help="custom report dir")
    parser.add_argument("--no-isolate", action="store_true", help="do not clean data before each scenario")
    args = parser.parse_args()

    root = Path(__file__).resolve().parent
    exe_path = (root / args.exe).resolve()

    if args.reset_data:
        clean_data(root)

    print(f"Building once: {args.build_cmd}")
    if build_once(root, args.build_cmd) != 0:
        print("build failed")
        return 2

    if not exe_path.exists():
        print(f"exe not found: {exe_path}")
        return 2

    stamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    report_dir = Path(args.report_dir) if args.report_dir else (root / f"compare_reports_{stamp}")
    report_dir.mkdir(parents=True, exist_ok=True)

    prefix = f"cmp{int(datetime.now().timestamp())}"
    scenarios = build_scenarios(prefix, args.quick)

    results = []
    for s in scenarios:
        print(f"\n=== {s.name} ===")
        print(" ; ".join(s.commands))
        results.append(run_scenario(root, exe_path, s, report_dir, isolate=(not args.no_isolate)))

    report = write_report(report_dir, exe_path, args.build_cmd, results)
    print("\nDone.")
    print(f"Report: {report}")

    return 1 if any(not r["passed"] for r in results) else 0


if __name__ == "__main__":
    sys.exit(main())
