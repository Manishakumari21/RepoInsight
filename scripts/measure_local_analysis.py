from __future__ import annotations

import argparse
import json
import time
import urllib.request


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", required=True, help="Local Git repository path.")
    parser.add_argument("--output", required=True, help="Output JSON file.")
    parser.add_argument(
        "--base-url",
        default="http://127.0.0.1:3000",
        help="Running backend base URL.",
    )
    parser.add_argument("--timeout", type=int, default=3600)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    body = json.dumps({"path": args.repo}).encode("utf-8")
    request = urllib.request.Request(
        args.base_url.rstrip("/") + "/api/local/timings",
        data=body,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    start = time.perf_counter()
    try:
        with urllib.request.urlopen(request, timeout=args.timeout) as response:
            status = response.status
            payload = json.loads(response.read().decode("utf-8"))
    except Exception as exc:
        print(f"request failed: {exc}")
        return 1
    wall_seconds = time.perf_counter() - start

    record = {
        "repository": args.repo,
        "http_status": status,
        "wall_seconds": round(wall_seconds, 3),
        "response": payload,
    }
    with open(args.output, "w", encoding="utf-8") as handle:
        json.dump(record, handle, indent=2)
        handle.write("\n")
    print(f"status={status} wall={wall_seconds:.1f}s -> {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
