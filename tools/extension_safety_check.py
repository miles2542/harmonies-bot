from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parents[1]
EXTENSION_ROOT = PROJECT_ROOT / "extension"
EXTENSION_SRC_ROOT = EXTENSION_ROOT / "src"
MANIFEST_PATH = EXTENSION_ROOT / "manifest.json"

ALLOWED_PERMISSIONS = {
    "https://boardgamearena.com/*",
    "https://*.boardgamearena.com/*",
    "http://127.0.0.1:17848/*",
    "ws://127.0.0.1:17848/*",
}
ALLOWED_CONTENT_MATCHES = {
    "https://boardgamearena.com/*",
    "https://*.boardgamearena.com/*",
}
ALLOWED_ENDPOINTS = {
    "http://127.0.0.1:17848/advise",
    "ws://127.0.0.1:17848/ws",
}


@dataclass(frozen=True)
class PatternRule:
    code: str
    pattern: re.Pattern[str]
    message: str


FORBIDDEN_JS_PATTERNS = [
    PatternRule("bga-ajaxcall", re.compile(r"\bajaxcall\s*\("), "BGA ajax action call"),
    PatternRule("bga-do-action", re.compile(r"\bdoAction\s*\("), "BGA action helper call"),
    PatternRule(
        "bga-perform-action",
        re.compile(r"\bbgaPerformAction\s*\("),
        "BGA action helper call",
    ),
    PatternRule(
        "synthetic-click",
        re.compile(r"\.click\s*\("),
        "synthetic DOM click",
    ),
    PatternRule(
        "synthetic-event",
        re.compile(r"\bdispatchEvent\s*\("),
        "synthetic DOM event dispatch",
    ),
    PatternRule(
        "remote-fetch",
        re.compile(r"\bfetch\s*\(\s*[`'\"]https?://(?!127\.0\.0\.1(?::17848)?/)"),
        "non-local fetch URL",
    ),
    PatternRule(
        "remote-websocket",
        re.compile(r"\bnew\s+WebSocket\s*\(\s*[`'\"]wss?://(?!127\.0\.0\.1(?::17848)?/)"),
        "non-local WebSocket URL",
    ),
]


@dataclass(frozen=True)
class Finding:
    path: str
    line_number: int | None
    code: str
    message: str
    detail: str


def rel_path(path: Path) -> str:
    return path.resolve().relative_to(PROJECT_ROOT).as_posix()


def scan_file(path: Path) -> list[Finding]:
    findings: list[Finding] = []
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        for rule in FORBIDDEN_JS_PATTERNS:
            if rule.pattern.search(line):
                findings.append(
                    Finding(
                        path=rel_path(path),
                        line_number=line_number,
                        code=rule.code,
                        message=rule.message,
                        detail=line.strip(),
                    )
                )
    return findings


def scan_js_tree(root: Path) -> list[Finding]:
    findings: list[Finding] = []
    for path in sorted(root.rglob("*.js")):
        findings.extend(scan_file(path))
    return findings


def check_manifest(path: Path) -> list[Finding]:
    manifest = json.loads(path.read_text(encoding="utf-8"))
    findings: list[Finding] = []

    permissions = set(manifest.get("permissions", []))
    extra_permissions = sorted(permissions - ALLOWED_PERMISSIONS)
    if extra_permissions:
        findings.append(
            Finding(
                path=rel_path(path),
                line_number=None,
                code="manifest-permission",
                message="unexpected manifest permission",
                detail=", ".join(extra_permissions),
            )
        )

    for index, content_script in enumerate(manifest.get("content_scripts", []), start=1):
        matches = set(content_script.get("matches", []))
        extra_matches = sorted(matches - ALLOWED_CONTENT_MATCHES)
        if extra_matches:
            findings.append(
                Finding(
                    path=rel_path(path),
                    line_number=None,
                    code="manifest-match",
                    message=f"unexpected content script match in entry {index}",
                    detail=", ".join(extra_matches),
                )
            )
    return findings


def check_local_endpoint_constants(path: Path) -> list[Finding]:
    source = path.read_text(encoding="utf-8")
    urls = set(re.findall(r"['\"](https?://[^'\"]+|wss?://[^'\"]+)['\"]", source))
    unexpected = sorted(urls - ALLOWED_ENDPOINTS)
    if not unexpected:
        return []
    return [
        Finding(
            path=rel_path(path),
            line_number=None,
            code="endpoint-url",
            message="unexpected advisor endpoint URL",
            detail=", ".join(unexpected),
        )
    ]


def scan_extension(src_root: Path, manifest_path: Path) -> list[Finding]:
    findings = scan_js_tree(src_root)
    findings.extend(check_manifest(manifest_path))
    findings.extend(check_local_endpoint_constants(src_root / "advisorClient.js"))
    return findings


def main() -> None:
    parser = argparse.ArgumentParser(description="Check extension stays read-only against BGA.")
    parser.add_argument("--src-root", type=Path, default=EXTENSION_SRC_ROOT)
    parser.add_argument("--manifest", type=Path, default=MANIFEST_PATH)
    parser.add_argument("--json", action="store_true", help="Emit machine-readable JSON.")
    args = parser.parse_args()

    findings = scan_extension(args.src_root, args.manifest)
    payload = {
        "status": "failed" if findings else "ok",
        "scanned": {
            "srcRoot": rel_path(args.src_root),
            "manifest": rel_path(args.manifest),
        },
        "findingCount": len(findings),
        "findings": [finding.__dict__ for finding in findings],
    }
    if args.json:
        print(json.dumps(payload, indent=2, sort_keys=True))
    elif not findings:
        print("extension safety check ok")
        return

    for finding in findings:
        location = finding.path
        if finding.line_number is not None:
            location = f"{location}:{finding.line_number}"
        print(f"{location}: {finding.code}: {finding.message}: {finding.detail}", file=sys.stderr)
    if findings:
        sys.exit(1)


if __name__ == "__main__":
    main()
