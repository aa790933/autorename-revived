#!/usr/bin/env python3
"""Build orchestrator: triple-target distribution."""

import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
DIST = ROOT / "dist"
BUILD_TMP = ROOT / "build_tmp"
GUI_DIR = ROOT / "gui"
TAURI_BINARIES = GUI_DIR / "src-tauri" / "binaries"
TAURI_EXE = GUI_DIR / "src-tauri" / "target" / "release" / "autorename-revived-gui.exe"

TARGET_TRIPLE = "x86_64-pc-windows-msvc"


def _get_version() -> str:
    version_file = ROOT / "autorename_revived" / "_version.py"
    if version_file.is_file():
        with open(version_file, "r", encoding="utf-8") as f:
            for line in f:
                if line.startswith("VERSION"):
                    return line.split("=")[1].strip().strip('"').strip("'")
    return "3.0.4"


def _clean():
    for d in [DIST, BUILD_TMP]:
        if d.exists():
            shutil.rmtree(d)


def _pyinstaller(name: str, entry: str, extra_flags=None, onefile=True, console=False) -> Path:
    extra_flags = extra_flags or []
    cmd = [
        sys.executable, "-m", "PyInstaller",
        "--noconfirm",
        "--name", name,
        "--distpath", str(DIST / name),
        "--workpath", str(BUILD_TMP),
        "--collect-all", "autorename_revived",
        "--collect-all", "pypdfium2",
        "--collect-all", "pypdfium2_raw",
        "--collect-data", "dateparser",
        "--collect-submodules", "openai",
        "--collect-submodules", "anthropic",
        "--hidden-import", "pydantic",
        "--hidden-import", "google.generativeai",
        "--add-data", f"config.yaml.example{os.pathsep}.",
        "--add-data", f".env.example{os.pathsep}.",
    ]
    if onefile:
        cmd.append("--onefile")
    if not console:
        cmd.append("--noconsole")
    cmd.extend(extra_flags)
    cmd.append(str(entry))
    subprocess.check_call(cmd)
    return DIST / name


def build_cli_sidecar() -> Path:
    print("=== Build: CLI sidecar (--onefile --noconsole) ===")
    _pyinstaller("autorename-revived-cli", "cli.py", onefile=True, console=False)
    exe = DIST / "autorename-revived-cli" / "autorename-revived-cli.exe"
    if not exe.exists():
        raise FileNotFoundError(f"CLI EXE not found at {exe}")
    size_mb = exe.stat().st_size / (1024 * 1024)
    print(f"  CLI: {exe.name} ({size_mb:.1f} MB)")
    result = subprocess.run([str(exe), "--version"], capture_output=True, text=True)
    if result.returncode != 0:
        raise RuntimeError(f"CLI --version failed (exit {result.returncode})")
    print(f"  Version: {result.stdout.strip()}")
    return exe


def build_onedir_bundle() -> Path:
    print("=== Build: Setup bundle (--onedir --noconsole) ===")
    _pyinstaller("autorename-revived-cli", "cli.py", onefile=False, console=False)
    exe = DIST / "autorename-revived-cli" / "autorename-revived-cli.exe"
    if not exe.exists():
        raise FileNotFoundError(f"onedir EXE not found at {exe}")
    size_mb = exe.stat().st_size / (1024 * 1024)
    print(f"  onedir EXE: {exe.name} ({size_mb:.1f} MB)")
    result = subprocess.run([str(exe), "--version"], capture_output=True, text=True)
    if result.returncode != 0:
        raise RuntimeError(f"onedir --version failed (exit {result.returncode})")
    print(f"  Version: {result.stdout.strip()}")
    return exe


def copy_sidecar(cli_exe: Path):
    if not sys.platform.startswith("win"):
        print("  [skip] Sidecar copy only on Windows")
        return
    TAURI_BINARIES.mkdir(parents=True, exist_ok=True)
    sidecar_name = f"autorename-revived-cli-{TARGET_TRIPLE}.exe"
    dest = TAURI_BINARIES / sidecar_name
    shutil.copy2(cli_exe, dest)
    print(f"  Sidecar: {dest.name} ({dest.stat().st_size / (1024 * 1024):.1f} MB)")


def build_tauri():
    print("=== Build: Tauri GUI ===")
    env = os.environ.copy()
    subprocess.check_call(["pnpm", "install"], cwd=str(GUI_DIR), env=env)
    subprocess.check_call(["pnpm", "tauri", "build"], cwd=str(GUI_DIR), env=env)

    if not TAURI_EXE.exists():
        raise FileNotFoundError(f"Tauri EXE not found at {TAURI_EXE}")
    size_mb = TAURI_EXE.stat().st_size / (1024 * 1024)
    print(f"  Tauri: {TAURI_EXE.name} ({size_mb:.1f} MB)")


def package_artifacts(version: str):
    print("=== Packaging ===")

    cli_exe = DIST / "autorename-revived-cli" / "autorename-revived-cli.exe"
    cli_zip = DIST / f"AutoRename-v{version}-CLI.zip"
    if cli_exe.exists():
        shutil.make_archive(str(cli_zip.with_suffix("")), "zip", cli_exe.parent)
        print(f"  CLI ZIP: {cli_zip.name}")

    onedir_dir = DIST / "autorename-revived-cli"
    setup_zip = DIST / f"AutoRename-v{version}-Setup.zip"
    if onedir_dir.exists():
        shutil.make_archive(str(setup_zip.with_suffix("")), "zip", onedir_dir)
        print(f"  Setup ZIP: {setup_zip.name}")

    release_dir = DIST / "release"
    release_dir.mkdir(parents=True, exist_ok=True)

    tauri_exe = TAURI_EXE
    if tauri_exe.exists():
        dest = release_dir / f"AutoRename-v{version}-Portable.exe"
        shutil.copy2(tauri_exe, dest)
        print(f"  Portable: {dest.name} ({dest.stat().st_size / (1024 * 1024):.1f} MB)")

    msi_dir = GUI_DIR / "src-tauri" / "target" / "release" / "bundle" / "msi"
    msies = list(msi_dir.glob("*.msi"))
    for msi in msies:
        dest = release_dir / msi.name
        shutil.copy2(msi, dest)
        print(f"  MSI: {dest.name} ({dest.stat().st_size / (1024 * 1024):.1f} MB)")

    manifest_path = release_dir / "release_manifest.txt"
    artifacts = sorted(release_dir.glob("*"))
    with open(manifest_path, "w") as f:
        for a in artifacts:
            f.write(f"{a.name}\n")
    print(f"\n  Manifest: {manifest_path.name}")
    print(f"\n{'=' * 50}")
    for a in artifacts:
        print(f"  {a.name}  ({a.stat().st_size / (1024 * 1024):.1f} MB)")
    print(f"{'=' * 50}")


def main():
    import argparse
    parser = argparse.ArgumentParser(description="Build orchestrator")
    parser.add_argument("--cli-only", action="store_true", help="Build only the CLI sidecar")
    parser.add_argument("--nosign", action="store_true", help="Skip code signing (no-op)")
    args = parser.parse_args()

    version = _get_version()

    if args.cli_only:
        _clean()
        cli_exe = build_cli_sidecar()
        copy_sidecar(cli_exe)
        package_artifacts(version)
        return

    _clean()
    cli_exe = build_cli_sidecar()
    copy_sidecar(cli_exe)
    build_onedir_bundle()
    build_tauri()
    package_artifacts(version)

    print(f"\n{'=' * 50}")
    print(f"  All builds passed for v{version}")
    print(f"  Output: {DIST}")
    print(f"{'=' * 50}")


if __name__ == "__main__":
    main()
