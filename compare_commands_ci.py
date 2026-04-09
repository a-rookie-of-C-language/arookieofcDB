#!/usr/bin/env python3
import argparse
import json
import re
import subprocess
import sys
import time
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
class ScenarioStage:
    commands: List[str]
    sleep_before: float = 0.0

@dataclass
class Scenario:
    name: str
    stages: List[ScenarioStage]
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

    all_raw = []
    all_commands = []
    in_text_accum = []
    exit_codes = []

    for stage in scenario.stages:
        if stage.sleep_before > 0:
            time.sleep(stage.sleep_before)

        stage_cmds = stage.commands + ["exit"]
        input_text = "\n".join(stage_cmds) + "\n"
        in_text_accum.append(input_text)

        proc = run_exe(root, exe_path, input_text)
        all_raw.append(proc.stdout or "")
        exit_codes.append(proc.returncode)
        all_commands.extend(stage.commands)

    raw = "\n".join(all_raw)
    out_path.write_text(raw, encoding="utf-8")
    in_path.write_text("\n---restart---\n".join(in_text_accum), encoding="utf-8")

    lines = normalize_lines(raw)
    chunks, prelude = chunk_by_elapsed(lines)

    cmd_results = []
    for i, cmd in enumerate(all_commands):
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
        "exit_code": exit_codes[-1] if exit_codes else 0,
        "exit_codes": exit_codes,
        "commands": all_commands,
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

# ================= Check Functions =================

def check_hybrid_basic(r: dict) -> Tuple[bool, List[str]]:
    ok, msgs = True, []
    if r["exit_code"] != 0: ok = False; msgs.append(f"exit_code != 0 ({r['exit_code']})")
    base_hits = sum(1 for i in [2, 3, 4] if "base" in r["cmd_results"][i]["output_text"])
    if base_hits < 3: ok = False; msgs.append("get/select did not all contain base")
    return ok, msgs

def check_hybrid_off_fault(r: dict) -> Tuple[bool, List[str]]:
    ok, msgs = True, []
    if "stale" not in r["cmd_results"][4]["output_text"]:
        ok = False; msgs.append("repairmode off get did not return stale")
    if not r["verify_issues"] or r["verify_issues"][0]["total"] < 1:
        ok = False; msgs.append("verify total should be >= 1")
    return ok, msgs

def check_hybrid_read_fault(r: dict) -> Tuple[bool, List[str]]:
    ok, msgs = True, []
    if "base" not in r["cmd_results"][4]["output_text"]:
        ok = False; msgs.append("repairmode read get did not return base")
    if not r["verify_issues"] or r["verify_issues"][0]["total"] != 0:
        ok = False; msgs.append("verify total should be 0")
    return ok, msgs

def check_hybrid_write_fault(r: dict) -> Tuple[bool, List[str]]:
    ok, msgs = True, []
    if len(r["verify_issues"]) < 2:
        return False, ["need two verify outputs"]
    if r["verify_issues"][0]["total"] < 1:
        ok = False; msgs.append("pre-write verify total should be >= 1")
    if r["verify_issues"][1]["total"] != 0:
        ok = False; msgs.append("post-write verify total should be 0")
    return ok, msgs

def check_wal_basic(r: dict) -> Tuple[bool, List[str]]:
    ok, msgs = True, []
    for i in [2, 3, 4]:
        if "v1" not in r["cmd_results"][i]["output_text"]:
            ok = False; msgs.append("wal get/select did not contain v1"); break
    return ok, msgs

def check_cachemax(r: dict) -> Tuple[bool, List[str]]:
    ok, msgs = True, []
    st = r["status_snapshots"]
    if not st or len(st) < 2: return False, ["missing status"]
    if as_int(st[1], "cache_evictions") < 1: ok = False; msgs.append("cache_evictions should be >= 1")
    if as_int(st[1], "cache_current_keys") != 1: ok = False; msgs.append("cache_current_keys should be 1")
    return ok, msgs

def check_sql_alias(r: dict) -> Tuple[bool, List[str]]:
    ok, msgs = True, []
    if "alice" not in r["cmd_results"][2]["output_text"]: ok = False; msgs.append("select fail 1")
    if "bob" not in r["cmd_results"][4]["output_text"]: ok = False; msgs.append("select fail 2")
    if "(nil)" not in r["cmd_results"][6]["output_text"]: ok = False; msgs.append("delete fail")
    return ok, msgs

def check_numeric(r: dict) -> Tuple[bool, List[str]]:
    ok, msgs = True, []
    if "111" not in r["cmd_results"][5]["output_text"]: ok = False; msgs.append("incr/incrby fail")
    if "4" not in r["cmd_results"][6]["output_text"]: ok = False; msgs.append("incrbyfloat fail")
    return ok, msgs

def check_cachepolicy(r: dict) -> Tuple[bool, List[str]]:
    ok, msgs = True, []
    st = r["status_snapshots"]
    if as_int(st[0], "cache_current_keys") != 0: ok = False; msgs.append("cache should be 0")
    if as_int(st[1], "cache_current_keys") != 1: ok = False; msgs.append("cache should be 1")
    return ok, msgs

def check_health_status(r: dict) -> Tuple[bool, List[str]]:
    ok, msgs = True, []
    st = r["status_snapshots"]
    if as_int(st[0], "inconsistency_total") < 1: ok = False; msgs.append("inconsistency misses")
    if as_int(st[1], "inconsistency_total") != 0: ok = False; msgs.append("repair failed")
    if as_int(st[1], "last_repair_total") < 1: ok = False; msgs.append("last_repair_total missing")
    return ok, msgs

def check_ttl_restart(r: dict) -> Tuple[bool, List[str]]:
    ok, msgs = True, []
    # Index 2 of commands is the 'get' after restart
    if "alive" not in r["cmd_results"][3]["output_text"]: ok = False; msgs.append("get failed after restart")
    if "Seconds" not in r["cmd_results"][4]["output_text"]: ok = False; msgs.append("ttl failed after restart")
    return ok, msgs

def check_ttl_down_expire(r: dict) -> Tuple[bool, List[str]]:
    ok, msgs = True, []
    if "(nil)" not in r["cmd_results"][3]["output_text"]: ok = False; msgs.append("should be nil but found value")
    if "NotFound" not in r["cmd_results"][4]["output_text"]: ok = False; msgs.append("should be NotFound")
    return ok, msgs

def check_boot_ttl_prune(r: dict) -> Tuple[bool, List[str]]:
    ok, msgs = True, []
    st = r["status_snapshots"]
    if len(st) < 2: return False, ["missing status"]
    if as_int(st[1], "ttl_loaded_on_startup") < 1: ok = False; msgs.append("loaded count missing")
    if as_int(st[1], "ttl_pruned_on_startup") < 1: ok = False; msgs.append("pruned count missing")
    return ok, msgs

# ================= Scenarios =================

def build_scenarios(prefix: str, quick: bool) -> List[Scenario]:
    items = [
        Scenario(
            name="01_hybrid_basic_get_vs_select",
            stages=[ScenarioStage(commands=["engine hybrid", f"set {prefix}:s1 base", f"get {prefix}:s1", f"select {prefix}:s1", f"select --disk {prefix}:s1", "status"])],
            check=check_hybrid_basic,
        ),
        Scenario(
            name="02_hybrid_fault_cache_only_repairmode_off",
            stages=[ScenarioStage(commands=["engine hybrid", "repairmode off", f"set {prefix}:s2 base", f"fault cache-only {prefix}:s2 stale", f"get {prefix}:s2", "verify", "status"])],
            check=check_hybrid_off_fault,
        ),
        Scenario(
            name="03_hybrid_fault_cache_only_repairmode_read",
            stages=[ScenarioStage(commands=["engine hybrid", "repairmode read", f"set {prefix}:s3 base", f"fault cache-only {prefix}:s3 stale", f"get {prefix}:s3", "verify", "status"])],
            check=check_hybrid_read_fault,
        ),
        Scenario(
            name="04_hybrid_fault_disk_only_repairmode_write",
            stages=[ScenarioStage(commands=["engine hybrid", "repairmode write", f"set {prefix}:s4 base", f"fault disk-only {prefix}:s4 disk_new", "verify", f"set {prefix}:s4_trigger x", "verify", "status"])],
            check=check_hybrid_write_fault,
        ),
        Scenario(
            name="05_wal_get_select_disk",
            stages=[ScenarioStage(commands=["engine wal", f"set {prefix}:s5 v1", f"get {prefix}:s5", f"select {prefix}:s5", f"select --disk {prefix}:s5", "status"])],
            check=check_wal_basic,
        ),
        # --- NEW SCENARIOS BELOW ---
        Scenario(
            name="06_hybrid_cachemax",
            stages=[ScenarioStage(commands=["engine hybrid", "cachemax 1", f"set {prefix}:c1 v1", f"set {prefix}:c2 v2", "status", "cachemax 0"])],
            check=check_cachemax,
        ),
        Scenario(
            name="07_hybrid_sql_add_update_del",
            stages=[ScenarioStage(commands=["engine hybrid", f"add {prefix}:user alice", f"select {prefix}:user", f"update {prefix}:user bob", f"select {prefix}:user", f"delete {prefix}:user", f"select {prefix}:user"])],
            check=check_sql_alias,
        ),
        Scenario(
            name="08_hybrid_numeric_incr",
            stages=[ScenarioStage(commands=["engine hybrid", f"set {prefix}:count 100", f"incr {prefix}:count", f"incrby {prefix}:count 10", f"set {prefix}:ratio 3.14", f"incrbyfloat {prefix}:ratio 0.86", f"get {prefix}:count", f"get {prefix}:ratio"])],
            check=check_numeric,
        ),
        Scenario(
            name="09_hybrid_cachepolicy",
            stages=[ScenarioStage(commands=["engine hybrid", "cachepolicy none", f"set {prefix}:pn 1", f"get {prefix}:pn", "status", "cachepolicy lru", f"get {prefix}:pn", "status"])],
            check=check_cachepolicy,
        ),
        Scenario(
            name="10_hybrid_health_status",
            stages=[ScenarioStage(commands=["engine hybrid", f"set {prefix}:vrh disk", f"fault cache-only {prefix}:vrh stale", "verify", "status", "repair --to disk", "verify", "status"])],
            check=check_health_status,
        ),
        Scenario(
            name="11_wal_ttl_survive_restart",
            stages=[
                ScenarioStage(commands=["engine wal", f"set {prefix}:ttl_alive alive", f"expire {prefix}:ttl_alive 30", "status"]),
                ScenarioStage(commands=["engine wal", f"get {prefix}:ttl_alive", f"ttl {prefix}:ttl_alive"], sleep_before=0.5)
            ],
            check=check_ttl_restart,
        ),
        Scenario(
            name="12_wal_ttl_down_expire",
            stages=[
                ScenarioStage(commands=["engine wal", f"set {prefix}:ttl_down die", f"expire {prefix}:ttl_down 1", "status"]),
                ScenarioStage(commands=["engine wal", f"get {prefix}:ttl_down", f"ttl {prefix}:ttl_down"], sleep_before=1.5)
            ],
            check=check_ttl_down_expire,
        ),
        Scenario(
            name="13_boot_ttl_prune_stats",
            stages=[
                ScenarioStage(commands=["engine wal", f"set {prefix}:bt:alive ok", f"expire {prefix}:bt:alive 30", f"set {prefix}:bt:dead x", f"expire {prefix}:bt:dead 1", "status"]),
                ScenarioStage(commands=["engine wal", "status"], sleep_before=1.5)
            ],
            check=check_boot_ttl_prune,
        )
    ]
    return items[:2] if quick else items

# ================= Reporting =================

def build_once(root: Path, build_cmd: str) -> int:
    proc = subprocess.run(
        build_cmd, cwd=str(root), shell=True, stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT, text=True, encoding="utf-8", errors="replace"
    )
    if proc.returncode != 0:
        print(proc.stdout)
    return proc.returncode

def write_report(report_dir: Path, exe_path: Path, build_cmd: str, results: List[dict]) -> Path:
    md_path = report_dir / "report.md"
    js_path = report_dir / "report.json"
    lines = ["# Command Behavior CI Report", "", f"- build_cmd: `{build_cmd}`", f"- exe_path: `{exe_path}`", f"- generated_at: `{datetime.now().isoformat(timespec='seconds')}`", ""]
    passed = sum(1 for r in results if r["passed"])
    lines.extend([f"- scenarios: `{len(results)}`", f"- passed: `{passed}`", f"- failed: `{len(results) - passed}`", ""])

    for r in results:
        status = "PASS" if r["passed"] else "FAIL"
        lines.extend([f"## {r['name']} [{status}]", f"- exit_codes: `{r['exit_codes']}`", f"- log: `{r['raw_log']}`"])
        if r["check_messages"]:
            lines.append("- check_messages:")
            for m in r["check_messages"]: lines.append(f"  - {m}")
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
        print(" ; ".join(cmd for stage in s.stages for cmd in stage.commands))
        results.append(run_scenario(root, exe_path, s, report_dir, isolate=(not args.no_isolate)))

    report = write_report(report_dir, exe_path, args.build_cmd, results)
    print("\nDone.")
    print(f"Report: {report}")

    return 1 if any(not r["passed"] for r in results) else 0

if __name__ == "__main__":
    sys.exit(main())
