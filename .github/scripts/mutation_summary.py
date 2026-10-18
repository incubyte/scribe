import argparse
import json
import sys
from dataclasses import dataclass, field
from pathlib import Path

RUST = "Rust (cargo-mutants)"
TYPESCRIPT = "TypeScript (Stryker)"
STRYKER_DETECTED = {"Killed", "Timeout"}
STRYKER_UNDETECTED = {"Survived", "NoCoverage"}
MAX_LISTED_SURVIVORS = 50


@dataclass
class ToolResult:
    name: str
    score: float | None = None
    survivors: list[str] = field(default_factory=list)
    note: str = ""
    problem: str = ""


def parse_args():
    parser = argparse.ArgumentParser(description="Summarise mutation testing for a pull request.")
    parser.add_argument("--title", default="Mutation testing")
    parser.add_argument("--rust-diff", type=Path, help="diff given to cargo-mutants --in-diff")
    parser.add_argument("--rust-out", type=Path, default=Path("mutants.out"))
    parser.add_argument("--rust-log", type=Path, help="cargo-mutants console output")
    parser.add_argument("--stryker-report", type=Path, help="Stryker's JSON report")
    parser.add_argument("--base-scores", type=Path, help="scores recorded on the base branch")
    parser.add_argument("--write-scores", type=Path, help="where to record this run's scores")
    parser.add_argument("--enforce", type=float, help="fail below this score or on a missing report")
    return parser.parse_args()


def rust_result(diff, out_dir, log):
    result = ToolResult(RUST)
    outcomes = out_dir / "outcomes.json"
    if not outcomes.exists():
        explain_missing_rust_report(result, diff, log)
        return result
    data = json.loads(outcomes.read_text())
    detected = data.get("caught", 0) + data.get("timeout", 0)
    result.score = percentage(detected, detected + data.get("missed", 0))
    result.survivors = read_lines(out_dir / "missed.txt")
    return result


def explain_missing_rust_report(result, diff, log):
    if not touches_rust(diff):
        result.note = "no Rust changes in this diff"
    elif found_no_mutants(log):
        result.note = "no mutants in the changed Rust code"
    else:
        result.problem = "cargo-mutants wrote no report for a diff that changes Rust code"


def found_no_mutants(log):
    return log is not None and log.exists() and "No mutants" in log.read_text(errors="replace")


def touches_rust(diff):
    header = "+++ b/"
    lines = diff.read_text(errors="replace").splitlines()
    return any(line.startswith(header) and line.endswith(".rs") for line in lines)


def read_lines(path):
    if not path.exists():
        return []
    return [line for line in path.read_text().splitlines() if line.strip()]


def stryker_result(report):
    result = ToolResult(TYPESCRIPT)
    if not report.exists():
        result.problem = "Stryker wrote no report"
        return result
    mutants = [
        (path, mutant)
        for path, file in json.loads(report.read_text())["files"].items()
        for mutant in file["mutants"]
    ]
    detected = sum(mutant["status"] in STRYKER_DETECTED for _, mutant in mutants)
    undetected = [(path, mutant) for path, mutant in mutants if mutant["status"] in STRYKER_UNDETECTED]
    result.score = percentage(detected, detected + len(undetected))
    result.survivors = [describe_stryker_mutant(path, mutant) for path, mutant in undetected]
    return result


def describe_stryker_mutant(path, mutant):
    line = mutant["location"]["start"]["line"]
    return f"{path}:{line} {mutant['mutatorName']} ({mutant['status']})"


def percentage(caught, total):
    return None if total == 0 else round(100 * caught / total, 1)


def collect_results(args):
    results = []
    if args.rust_diff is not None:
        results.append(rust_result(args.rust_diff, args.rust_out, args.rust_log))
    if args.stryker_report is not None:
        results.append(stryker_result(args.stryker_report))
    return results


def load_base_scores(path):
    if path is None or not path.exists():
        return {}
    return json.loads(path.read_text())


def describe_score(result):
    if result.problem:
        return f"failed: {result.problem}"
    if result.score is None:
        return result.note or "no mutants"
    return f"{result.score}%"


def describe_delta(result, base_scores):
    base = base_scores.get(result.name)
    if result.score is None or base is None:
        return "unavailable"
    return f"{result.score - base:+.1f} pts vs base ({base}%)"


def survivors_section(result):
    if not result.survivors:
        return []
    listed = result.survivors[:MAX_LISTED_SURVIVORS]
    hidden = len(result.survivors) - len(listed)
    lines = ["", f"<details><summary>{result.name}: {len(result.survivors)} surviving mutants</summary>", ""]
    lines += [f"- `{survivor}`" for survivor in listed]
    if hidden > 0:
        lines.append(f"- … and {hidden} more")
    return lines + ["", "</details>"]


def render(title, results, base_scores):
    lines = [f"## {title}", "", "| Tool | Score | Change vs base |", "|---|---|---|"]
    lines += [f"| {r.name} | {describe_score(r)} | {describe_delta(r, base_scores)} |" for r in results]
    for result in results:
        lines += survivors_section(result)
    return "\n".join(lines)


def write_scores(path, results):
    if path is not None:
        scores = {r.name: r.score for r in results if r.score is not None}
        path.write_text(json.dumps(scores, indent=2))


def failures(results, threshold):
    problems = [f"{r.name}: {r.problem}" for r in results if r.problem]
    below = [f"{r.name}: {r.score}% < {threshold}%" for r in results if r.score is not None and r.score < threshold]
    return problems + below


def main():
    args = parse_args()
    results = collect_results(args)
    print(render(args.title, results, load_base_scores(args.base_scores)))
    write_scores(args.write_scores, results)
    if args.enforce is None:
        return 0
    found = failures(results, args.enforce)
    for failure in found:
        print(failure, file=sys.stderr)
    return 1 if found else 0


if __name__ == "__main__":
    sys.exit(main())
