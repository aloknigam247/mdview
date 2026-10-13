import argparse
import pathlib
import re
import sys
import tomllib


def replace_version(content, version):
    updated, count = re.subn(
        r'(?m)^(version[ \t]*=[ \t]*")[^"\r\n]+(")',
        lambda match: f"{match[1]}{version}{match[2]}",
        content,
    )
    if count != 1:
        raise ValueError(f"Expected one version field, found {count}")
    return updated


def set_version(version):
    local_names = []
    member_names = []
    lock_path = pathlib.Path("Cargo.lock")
    lock_content = lock_path.read_bytes().decode("utf-8")
    lock = tomllib.loads(lock_content)
    root_path = pathlib.Path("Cargo.toml")
    root_content = root_path.read_bytes().decode("utf-8")
    root = tomllib.loads(root_content)
    current = root["workspace"]["package"]["version"]
    if tuple(map(int, version.split("."))) < tuple(map(int, current.split("."))):
        raise ValueError(f"Refusing to replace workspace version {current} with older version {version}")

    for pattern in root["workspace"]["members"]:
        for directory in pathlib.Path.cwd().glob(pattern):
            member = tomllib.loads((directory / "Cargo.toml").read_text(encoding="utf-8"))
            if member["package"]["version"] != {"workspace": True}:
                raise ValueError(f"{directory} must inherit the workspace version")
            member_names.append(member["package"]["name"])

    sections = re.split(r"(?m)(?=^\[\[package\]\][ \t]*\r?$)", lock_content)
    for index, section in enumerate(sections):
        if not section.startswith("[[package]]"):
            continue
        package = tomllib.loads(section)["package"][0]
        if "source" not in package:
            local_names.append(package["name"])
            sections[index] = replace_version(section, version)
    if not member_names or sorted(local_names) != sorted(member_names):
        raise ValueError("Source-less lockfile packages must match the workspace members exactly")
    updated_lock = "".join(sections)
    updated_root, count = re.subn(
        r"(?ms)^\[workspace\.package\][ \t]*\r?\n.*?(?=^\[|\Z)",
        lambda match: replace_version(match[0], version),
        root_content,
    )
    if count != 1:
        raise ValueError(f"Expected one workspace.package section, found {count}")

    root["workspace"]["package"]["version"] = version
    for package in lock["package"]:
        if "source" not in package:
            package["version"] = version
    if tomllib.loads(updated_root) != root or tomllib.loads(updated_lock) != lock:
        raise ValueError("Version update would modify unrelated Cargo metadata")

    root_path.write_bytes(updated_root.encode("utf-8"))
    lock_path.write_bytes(updated_lock.encode("utf-8"))
    print(f"Set workspace and {len(member_names)} package versions to {version}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Write the version selected by semantic-release")
    parser.add_argument("version")
    arguments = parser.parse_args()
    if not re.fullmatch(r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)", arguments.version):
        parser.error("expected a stable MAJOR.MINOR.PATCH version")
    try:
        set_version(arguments.version)
    except (KeyError, OSError, ValueError) as error:
        print(f"Version update failed: {error}", file=sys.stderr)
        sys.exit(1)
