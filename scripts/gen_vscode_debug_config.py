#!/usr/bin/env python

import json
import re
import subprocess


def cargo_get_target():
    cargo_output = subprocess.run(
        ["cargo", "config", "get", "-Z", "unstable-options", "--format", "json", "build.target"],
        capture_output=True
    ).stdout.decode()
    return json.loads(cargo_output)["build"]["target"].removesuffix(".json")


def cargo_get_package_name():
    return subprocess.run(["cargo", "pkgid"],
                          capture_output=True).stdout.decode().rsplit("/", 1)[-1].split("#")[0]


def cargo_get_bin_test_executable():
    package_name = cargo_get_package_name()
    cargo_output = next(filter(
        lambda line: "Executable unittests src/main.rs" in line,
        subprocess.run(["cargo", "test", "--bin", package_name, "--no-run"],
                       capture_output=True).stderr.decode().split("\n")
    ), None)
    if cargo_output:
        return cargo_output.split("(")[1].split(")")[0]


def cargo_get_lib_test_executable():
    cargo_output = next(filter(
        lambda line: "Executable unittests src/lib.rs" in line,
        subprocess.run(["cargo", "test", "--lib", "--no-run"],
                       capture_output=True).stderr.decode().split("\n")
    ), None)
    if cargo_output:
        return cargo_output.split("(")[1].split(")")[0]


def cargo_get_integration_tests():
    cargo_output = filter(
        lambda line: "Executable tests/" in line,
        subprocess.run(["cargo", "test", "--tests", "--no-run"],
                       capture_output=True).stderr.decode().split("\n")
    )
    integration_test_re = re.compile(r"\s*Executable tests/(.*?)(?:/main)?.rs \(.*\)")
    integration_tests = []
    for line in cargo_output:
        found = integration_test_re.fullmatch(line)
        if found:
            integration_tests.append(found.group(1))
    return integration_tests


def replace_or_append(sequence_of_mappings, mapping, key):
    for (i, current) in enumerate(sequence_of_mappings):
        if current.get(key) == mapping[key]:
            sequence_of_mappings[i] = mapping
            return
    sequence_of_mappings.append(mapping)


def replace_or_append_task(tasks, task):
    replace_or_append(tasks, task, "label")


def replace_or_append_configuration(configurations, configuration):
    replace_or_append(configurations, configuration, "name")


def gen_tasks_json(tasks, integration_tests):
    TASK_TEMPLATE = {
        "type": "shell",
        "hide": True,
        "dependsOn": [
            "symlink-test-executables"
        ],
        "dependsOrder": "sequence",
        "isBackground": True,
        "problemMatcher": {
            "pattern": {
                "regexp": "________"
            },
            "background": {
                "activeOnStart": True,
                "beginsPattern": "Building",
                "endsPattern": "Running"
            }
        }
    }

    # Task: Create predictable symlinks to test executables
    symlink_task = {
        "label": "symlink-test-executables",
        "type": "shell",
        "command": "scripts/symlink_tests.sh",
        "hide": True,
        "presentation": {
            "reveal": "silent",
            "panel": "shared",
            "showReuseMessage": False,
        }
    }
    replace_or_append_task(tasks, symlink_task)

    # Task: `cargo run` with arguments to wait for GDB attach
    gdb_run = TASK_TEMPLATE.copy()
    # No symlink for the default binary
    del gdb_run["dependsOn"]
    del gdb_run["dependsOrder"]
    gdb_run["command"] = "cargo run -- -s -S"
    gdb_run["label"] = "gdb-run"
    replace_or_append_task(tasks, gdb_run)

    # Task: `cargo test` binary crate (GDB)
    gdb_unittests_bin = TASK_TEMPLATE.copy()
    package_name = cargo_get_package_name()
    gdb_unittests_bin["command"] = f"cargo test --bin {package_name} -- -s -S"
    gdb_unittests_bin["label"] = "gdb-unittests-bin"
    replace_or_append_task(tasks, gdb_unittests_bin)

    # Task: `cargo test` library crate (GDB)
    gdb_unittests_lib = TASK_TEMPLATE.copy()
    gdb_unittests_lib["command"] = "cargo test --lib -- -s -S"
    gdb_unittests_lib["label"] = "gdb-unittests-lib"
    replace_or_append_task(tasks, gdb_unittests_lib)

    for test in integration_tests:
        # Task: `cargo test` integration test (GDB)
        gdb_integration_test = TASK_TEMPLATE.copy()
        gdb_integration_test["command"] = f"cargo test --test {test} -- -s -S"
        gdb_integration_test["label"] = f"gdb-test-{test}"
        replace_or_append_task(tasks, gdb_integration_test)


def gen_launch_json(configurations, integration_tests):
    CONFIGURATION_TEMPLATE = {
        "type": "gdb",
        "gdbpath": "rust-gdb",
        "request": "attach",
        "target": ":1234",
        "remote": True,
        "stopAtEntry": "_start",
        "internalConsoleOptions": "openOnSessionStart",
        "cwd": "${workspaceRoot}",
    }

    # Config: attach to OS binary (running in QEMU)
    gdb_run = CONFIGURATION_TEMPLATE.copy()
    target = cargo_get_target()
    package_name = cargo_get_package_name()
    gdb_run["executable"] = f"target/{target}/debug/{package_name}"
    gdb_run["preLaunchTask"] = "gdb-run"
    gdb_run["name"] = f"{package_name} (GDB+QEMU)"
    replace_or_append_configuration(configurations, gdb_run)

    # Config: attach to OS binary unittests
    gdb_unittests_bin = CONFIGURATION_TEMPLATE.copy()
    target = cargo_get_target()
    package_name = cargo_get_package_name()
    gdb_unittests_bin["executable"] = f"target/{target}/debug/{package_name}-unittests-bin"
    gdb_unittests_bin["preLaunchTask"] = "gdb-unittests-bin"
    gdb_unittests_bin["name"] = "unittests [bin] (GDB+QEMU)"
    replace_or_append_configuration(configurations, gdb_unittests_bin)

    # Config: attach to OS library unittests
    gdb_unittests_lib = CONFIGURATION_TEMPLATE.copy()
    target = cargo_get_target()
    package_name = cargo_get_package_name()
    gdb_unittests_lib["executable"] = f"target/{target}/debug/{package_name}-unittests-lib"
    gdb_unittests_lib["preLaunchTask"] = "gdb-unittests-lib"
    gdb_unittests_lib["name"] = "unittests [lib] (GDB+QEMU)"
    replace_or_append_configuration(configurations, gdb_unittests_lib)

    for test in integration_tests:
        # Config: attach to integration test `test`
        gdb_integration_test = CONFIGURATION_TEMPLATE.copy()
        gdb_integration_test["executable"] = f"target/{target}/debug/{package_name}-test-{test}"
        gdb_integration_test["preLaunchTask"] = f"gdb-test-{test}"
        gdb_integration_test["name"] = f"tests/{test} (GDB+QEMU)"
        replace_or_append_configuration(configurations, gdb_integration_test)


def main():
    integration_tests = cargo_get_integration_tests()

    with open(".vscode/tasks.json") as fp:
        json_root = json.load(fp)
    gen_tasks_json(json_root["tasks"], integration_tests)
    with open(".vscode/tasks.json", "w") as fp:
        json.dump(json_root, fp, indent="\t")

    with open(".vscode/launch.json") as fp:
        json_root = json.load(fp)
    gen_launch_json(json_root["configurations"], integration_tests)
    with open(".vscode/launch.json", "w") as fp:
        json.dump(json_root, fp, indent="\t")


if __name__ == "__main__":
    main()
