"""Package the native build, preserving Unix permissions and macOS app metadata."""
import argparse
import json
import os
from pathlib import Path
import plistlib
import re
import shutil
import subprocess
import tarfile
import tempfile
import tomllib
import zipfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", required=True)
    parser.add_argument("--platform", required=True,
                        choices=["windows-x64", "linux-x64", "macos-arm64", "macos-x64"])
    parser.add_argument("--version", required=True)
    args = parser.parse_args()
    if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9.-]*", args.version):
        parser.error("Invalid version")
    windows = args.platform.startswith("windows")
    source = Path("target") / args.target / "release" / ("wmc.exe" if windows else "wmc")
    if not source.is_file():
        parser.error(f"Missing executable: {source}")
    dist = Path("dist")
    dist.mkdir(exist_ok=True)
    name = f"wmc-{args.version}-{args.platform}"
    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary) / name
        root.mkdir()
        shutil.copy2("README.md", root / "README.md")
        (root / "BUILD-INFO.json").write_text(json.dumps({
            "version": args.version, "target": args.target,
            "commit": os.environ.get("GITHUB_SHA", "local"),
        }, indent=2) + "\n", encoding="utf-8")
        if args.platform.startswith("macos"):
            app = root / "WMC.app"
            contents = app / "Contents"
            executable = contents / "MacOS" / "wmc"
            executable.parent.mkdir(parents=True)
            shutil.copy2(source, executable)
            executable.chmod(0o755)
            version = args.version.removeprefix("v").split("-", 1)[0]
            if not re.fullmatch(r"\d+\.\d+\.\d+", version):
                version = tomllib.loads(Path("Cargo.toml").read_text(encoding="utf-8"))["workspace"]["package"]["version"]
            with (contents / "Info.plist").open("wb") as file:
                plistlib.dump({
                    "CFBundleName": "WMC", "CFBundleDisplayName": "Windows Media Center",
                    "CFBundleIdentifier": "io.github.wanghaozhengming.wmc",
                    "CFBundleExecutable": "wmc", "CFBundlePackageType": "APPL",
                    "CFBundleShortVersionString": version,
                    "CFBundleVersion": os.environ.get("GITHUB_RUN_NUMBER", "1"),
                    "LSMinimumSystemVersion": "13.0", "NSHighResolutionCapable": True,
                }, file)
            subprocess.run(["codesign", "--force", "--sign", "-", str(app)], check=True)
            subprocess.run(["codesign", "--verify", "--deep", "--strict", str(app)], check=True)
        else:
            executable = root / ("wmc.exe" if windows else "wmc")
            shutil.copy2(source, executable)
            executable.chmod(0o755)
        if windows:
            shutil.copy2(source, dist / f"{name}.exe")
            with zipfile.ZipFile(dist / f"{name}.zip", "w", zipfile.ZIP_DEFLATED) as archive:
                for file in sorted(root.rglob("*")):
                    if file.is_file():
                        archive.write(file, file.relative_to(root))
        else:
            def permissions(info):
                if info.isdir() or (info.isfile() and Path(info.name).name == "wmc"):
                    info.mode = 0o755
                elif info.isfile():
                    info.mode = 0o644
                return info

            with tarfile.open(dist / f"{name}.tar.gz", "w:gz") as archive:
                archive.add(root, arcname=name, filter=permissions)
    for file in sorted(dist.iterdir()):
        print(file.name, file.stat().st_size)


if __name__ == "__main__":
    main()
