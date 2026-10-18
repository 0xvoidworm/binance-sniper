"""Explicit local import only: python -m binance_sniper.tester --help."""
import argparse
import json
from pathlib import Path
import sqlite3

from .report import MAX_BYTES
from .store import DEFAULT_DB, EvidenceStore, read_artifact


def main():
    parser = argparse.ArgumentParser(description="Import an MT5 English HTML report; never launches MT5 or sends orders.")
    parser.add_argument("--db", type=Path, default=DEFAULT_DB)
    sub = parser.add_subparsers(dest="command", required=True)
    ingest = sub.add_parser("import")
    ingest.add_argument("--report", type=Path, required=True)
    ingest.add_argument("--context", type=Path, required=True)
    for artifact in ("binary", "set", "tester-config", "tester-log"):
        ingest.add_argument("--" + artifact, type=Path)
    sub.add_parser("status")
    compare = sub.add_parser("compare")
    compare.add_argument("left")
    compare.add_argument("right")
    args = parser.parse_args()
    store = EvidenceStore(args.db)
    try:
        if args.command == "import":
            context = json.loads(read_artifact(args.context, 16_384).decode("utf-8-sig"))
            artifacts = {key: read_artifact(path, 64 * 1024 * 1024) for key in ("binary", "set", "tester_config", "tester_log") if (path := getattr(args, key))}
            run = store.import_report(read_artifact(args.report, MAX_BYTES), context, artifacts)
            result = {"id": run["id"], "validation": run["report"]["validation"], "errors": run["report"]["errors"], "repeat_missing": run["repeat_missing"]}
        elif args.command == "compare":
            result = store.compare(args.left, args.right)
            if result is None:
                raise ValueError("Run not found")
        else:
            result = store.snapshot()
        print(json.dumps(result, indent=2, ensure_ascii=False, allow_nan=False))
        return 2 if args.command == "import" and result["errors"] else 0
    except (ValueError, UnicodeError, OSError, sqlite3.Error) as exc:
        # Never echo paths/file contents/credentials from parser or IO exceptions.
        print(json.dumps({"status": "IMPORT_OR_READ_FAILED", "error_type": type(exc).__name__}))
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
