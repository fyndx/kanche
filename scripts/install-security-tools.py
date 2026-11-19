# SPDX-License-Identifier: MPL-2.0

"""Install reviewed Linux x86_64 release binaries, verified before extraction."""

import hashlib
import io
import os
import platform
import tarfile
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TOOLS = ROOT / ".tools"
RELEASES = [
    (
        "gitleaks",
        "https://github.com/gitleaks/gitleaks/releases/download/v8.30.1/gitleaks_8.30.1_linux_x64.tar.gz",
        "551f6fc83ea457d62a0d98237cbad105af8d557003051f41f3e7ca7b3f2470eb",
    ),
    (
        "cargo-deny",
        "https://github.com/EmbarkStudios/cargo-deny/releases/download/0.21.0/cargo-deny-0.21.0-x86_64-unknown-linux-musl.tar.gz",
        "a2170b854ff697e79e39c4c538981c9c1bdd803addbae6719a4cb66b4585a903",
    ),
]

if (platform.system(), platform.machine()) != ("Linux", "x86_64"):
    raise SystemExit("Security tool installer supports Linux x86_64; run make security in that environment (as CI does).")

TOOLS.mkdir(exist_ok=True)
for name, url, expected in RELEASES:
    archive_path = TOOLS / f"{name}-{expected}.tar.gz"
    if not archive_path.exists():
        with urllib.request.urlopen(url, timeout=60) as response:
            payload = response.read()
    else:
        payload = archive_path.read_bytes()
    if hashlib.sha256(payload).hexdigest() != expected:
        raise SystemExit(f"Checksum mismatch for {name}; remove its cached archive and investigate.")
    archive_path.write_bytes(payload)
    with tarfile.open(fileobj=io.BytesIO(payload), mode="r:gz") as archive:
        members = [member for member in archive.getmembers() if Path(member.name).name == name and member.isfile()]
        if len(members) != 1:
            raise SystemExit(f"Unexpected release archive layout for {name}")
        executable = archive.extractfile(members[0]).read()
    temporary = TOOLS / f"{name}.tmp"
    temporary.write_bytes(executable)
    temporary.chmod(0o755)
    os.replace(temporary, TOOLS / name)
    print(f"Installed checksum-verified {name}")
