import json
import sys
from pathlib import Path

RUST_OUTCOMES = Path("mutants.out/outcomes.json")
STRYKER_REPORT = Path("apps/scribe-tray/reports/mutation/mutation.json")


def rust_score():
    if not RUST_OUTCOMES.exists():
        return None
    outcomes = json.loads(RUST_OUTCOMES.read_text())
    caught, missed = outcomes.get("caught", 0), outcomes.get("missed", 0)
    return percentage(caught, caught + missed)


def stryker_score():
    if not STRYKER_REPORT.exists():
        return None
    files = json.loads(STRYKER_REPORT.read_text())["files"].values()
    statuses = [m["status"] for f in files for m in f["mutants"]]
    killed = sum(s in ("Killed", "Timeout") for s in statuses)
    survived = sum(s == "Survived" for s in statuses)
    return percentage(killed, killed + survived)


def percentage(caught, total):
    return None if total == 0 else round(100 * caught / total, 1)


def describe(score):
    return "no mutants in this diff" if score is None else f"{score}%"


def main():
    scores = {"Rust (cargo-mutants)": rust_score(), "TypeScript (Stryker)": stryker_score()}
    print("## Mutation testing\n")
    for name, score in scores.items():
        print(f"- {name}: {describe(score)}")
    threshold = float(sys.argv[2]) if sys.argv[1:2] == ["--enforce"] else None
    below = [s for s in scores.values() if s is not None and threshold and s < threshold]
    sys.exit(1 if below else 0)


main()
