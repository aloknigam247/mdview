import argparse
import base64
import json
import os
import re
import subprocess
import sys
import time
import tomllib
from urllib.parse import quote, urlencode


ALLOWED_FILES = {
    ".release-please-manifest.json",
    "CHANGELOG.md",
    "Cargo.lock",
    "Cargo.toml",
    "version.txt",
}
PENDING_LABEL = "autorelease: pending"
TAGGED_LABEL = "autorelease: tagged"
VERSION_BRANCH = "release-please--branches--main"
VERSION_PATTERN = r"(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)"


def api(repository, endpoint, method="GET", body=None):
    output = gh("api", f"repos/{repository}/{endpoint}", "--method", method, body=body)
    return json.loads(output) if output.strip() else None


def check_pull_request(repository, pull):
    match = re.fullmatch(rf"chore\(release\): release ({VERSION_PATTERN})", pull["title"])
    if (
        pull["base"]["ref"] != "main"
        or pull["head"]["ref"] != VERSION_BRANCH
        or (pull["head"]["repo"] or {}).get("full_name") != repository
        or not match
        or PENDING_LABEL not in {label["name"] for label in pull["labels"]}
    ):
        raise RuntimeError(f"PR #{pull['number']} is not an eligible version PR")
    if not 0 < pull["changed_files"] <= len(ALLOWED_FILES):
        raise RuntimeError(f"PR #{pull['number']} changes files outside version metadata")
    files = api(repository, f"pulls/{pull['number']}/files?per_page=100")
    if len(files) != pull["changed_files"] or any(
        entry["filename"] not in ALLOWED_FILES for entry in files
    ):
        raise RuntimeError(f"PR #{pull['number']} changes files outside version metadata")
    return match.group(1)


def check_version_files(repository, commit, version):
    cargo = tomllib.loads(get_file(repository, commit, "Cargo.toml"))
    changelog = get_file(repository, commit, "CHANGELOG.md")
    heading = re.search(rf"(?m)^#{{2,3}} \[?v?({VERSION_PATTERN})\b", changelog)
    lock = tomllib.loads(get_file(repository, commit, "Cargo.lock"))
    local_packages = [package for package in lock["package"] if "source" not in package]
    manifest = json.loads(get_file(repository, commit, ".release-please-manifest.json"))
    mirror = get_file(repository, commit, "version.txt").strip()
    if (
        cargo["workspace"]["package"]["version"] != version
        or not local_packages
        or any(package["version"] != version for package in local_packages)
        or manifest != {".": version}
        or mirror != version
        or not heading
        or heading.group(1) != version
    ):
        raise RuntimeError(f"Version metadata at {commit} does not consistently declare {version}")
    return cargo, lock


def finish_version(repository, pull):
    if not pull["merged"] or not pull["merge_commit_sha"]:
        raise RuntimeError(f"PR #{pull['number']} has not merged")
    commit = pull["merge_commit_sha"]
    version = check_pull_request(repository, pull)
    check_version_files(repository, commit, version)
    tag = f"v{version}"
    refs = api(repository, f"git/matching-refs/tags/{tag}")
    existing = [ref for ref in refs if ref["ref"] == f"refs/tags/{tag}"]
    if existing:
        target = existing[0]["object"]
        while target["type"] == "tag":
            target = api(repository, f"git/tags/{target['sha']}")["object"]
        if target["type"] != "commit" or target["sha"] != commit:
            raise RuntimeError(f"Tag {tag} does not point to {commit}; refusing to move it")
    else:
        api(repository, "git/refs", "POST", {"ref": f"refs/tags/{tag}", "sha": commit})

    labels = json.loads(gh("api", f"repos/{repository}/labels", "--paginate", "--slurp"))
    if not any(label["name"] == TAGGED_LABEL for page in labels for label in page):
        api(repository, "labels", "POST", {
            "color": "0e8a16",
            "description": "Version tag recorded; no GitHub Release published",
            "name": TAGGED_LABEL,
        })
    api(repository, f"issues/{pull['number']}/labels", "POST", {"labels": [TAGGED_LABEL]})
    # Clear pending only after the tag exists, so interrupted runs remain recoverable.
    api(repository, f"issues/{pull['number']}/labels/{quote(PENDING_LABEL, safe='')}", "DELETE")
    print(f"Recorded {tag} at {commit} for PR #{pull['number']}; no GitHub Release created")


def get_file(repository, commit, filename):
    result = api(repository, f"contents/{filename}?ref={commit}")
    if result.get("type") != "file" or result.get("encoding") != "base64":
        raise RuntimeError(f"Cannot read {filename} at {commit}")
    return base64.b64decode(result["content"].replace("\n", ""), validate=True).decode("utf-8")


def gh(*arguments, body=None):
    command = ["gh", *arguments]
    if body is not None:
        command += ["--input", "-"]
    result = subprocess.run(
        command,
        capture_output=True,
        check=False,
        encoding="utf-8",
        input=json.dumps(body) if body is not None else None,
        text=True,
    )
    if result.returncode:
        raise RuntimeError(f"{' '.join(command)} failed:\n{result.stderr.strip()}")
    return result.stdout


def main():
    parser = argparse.ArgumentParser(description="Merge version PRs and record Git tags without releases")
    parser.add_argument("mode", choices=["merge", "prepare"])
    arguments = parser.parse_args()
    repository = os.environ.get("GH_REPO", "")
    if not re.fullmatch(r"[\w.-]+/[\w.-]+", repository):
        raise RuntimeError("GH_REPO must identify the version repository")
    if not os.environ.get("GH_TOKEN", "").strip():
        raise RuntimeError("Set the repository secret RELEASE_PLEASE_TOKEN; there is no GITHUB_TOKEN fallback")

    if arguments.mode == "prepare":
        for number in pull_numbers(repository, "closed"):
            finish_version(repository, api(repository, f"pulls/{number}"))
    else:
        numbers = pull_numbers(repository, "open")
        if not numbers:
            print("No open version PR to merge")
        elif len(numbers) != 1:
            raise RuntimeError("More than one version PR is open; manual review is required")
        else:
            merge_version(repository, numbers[0])


def merge_version(repository, number):
    pull = api(repository, f"pulls/{number}")
    for _ in range(5):
        if pull["mergeable"] is not None or pull["state"] != "open":
            break
        time.sleep(2)
        pull = api(repository, f"pulls/{number}")
    if pull["merged"]:
        finish_version(repository, pull)
        return
    if pull["state"] != "open" or pull["mergeable"] is None:
        raise RuntimeError(f"PR #{number} is not ready for a merge decision; retry a later run")
    version = check_pull_request(repository, pull)
    cargo, lock = check_version_files(repository, pull["head"]["sha"], version)
    base_cargo = tomllib.loads(get_file(repository, pull["base"]["sha"], "Cargo.toml"))
    base_lock = tomllib.loads(get_file(repository, pull["base"]["sha"], "Cargo.lock"))
    base_cargo["workspace"]["package"]["version"] = version
    for package in base_lock["package"]:
        if "source" not in package:
            package["version"] = version
    if cargo != base_cargo or lock != base_lock:
        raise RuntimeError(f"PR #{number} changes Cargo metadata beyond workspace versions")
    if pull["auto_merge"]:
        print(f"PR #{number} is already waiting for auto-merge; no tag created before merging")
        return
    gh(
        "pr", "merge", str(number), "--repo", repository, "--auto", "--squash",
        "--match-head-commit", pull["head"]["sha"],
    )
    merged = api(repository, f"pulls/{number}")
    if merged["merged"]:
        finish_version(repository, merged)
    elif merged["state"] == "open" and merged["auto_merge"]:
        print(f"Auto-merge enabled for PR #{number}; its tag will be recorded after it merges")
    else:
        raise RuntimeError(f"PR #{number} was neither merged nor queued for auto-merge")


def pull_numbers(repository, state):
    selector = ".[] | .number"
    owner = repository.split("/")[0]
    query = urlencode({
        "base": "main",
        "head": f"{owner}:{VERSION_BRANCH}",
        "per_page": 100,
        "state": state,
    })
    if state == "closed":
        selector = (
            '.[] | select(.merged_at != null and '
            'any(.labels[]; .name == "autorelease: pending")) | .number'
        )
    output = gh("api", f"repos/{repository}/pulls?{query}", "--paginate", "--jq", selector)
    return sorted(int(number) for number in output.split())


if __name__ == "__main__":
    try:
        main()
    except (OSError, RuntimeError, ValueError) as error:
        print(f"Version automation failed: {error}", file=sys.stderr)
        sys.exit(1)
