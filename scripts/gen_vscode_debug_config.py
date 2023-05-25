#!/usr/bin/env python

import functools
import json
import re
import subprocess


@functools.cache
def cargo_get_target():
    cargo_output = subprocess.run(
        ["cargo", "config", "get", "-Z", "unstable-options", "--format", "json", "build.target"],
        capture_output=True
    ).stdout.decode()
    return json.loads(cargo_output)["build"]["target"].removesuffix(".json")


@functools.cache
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


TASK_LABEL_PREFIX = "debug-"
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


def gen_cargo_task(label, cargo_cmd):
    task = TASK_TEMPLATE.copy()
    task["label"] = f"{TASK_LABEL_PREFIX}{label}"
    task["command"] = f"cargo {cargo_cmd} -- -s -S"
    return task


def gen_tasks_json(tasks, integration_tests):
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

    # Task: `cargo run` with arguments to wait for debugger attach
    debug_run = gen_cargo_task("run", "run")
    # No symlink for the default binary
    del debug_run["dependsOn"]
    del debug_run["dependsOrder"]
    replace_or_append_task(tasks, debug_run)

    # Task: `cargo test` binary crate
    package_name = cargo_get_package_name()
    debug_unittests_bin = gen_cargo_task("unittests-bin", f"test --bin {package_name}")
    replace_or_append_task(tasks, debug_unittests_bin)

    # Task: `cargo test` library crate
    debug_unittests_lib = gen_cargo_task("unittests-lib", "test --lib")
    replace_or_append_task(tasks, debug_unittests_lib)

    for test in integration_tests:
        # Task: `cargo test` integration test
        debug_integration_test = gen_cargo_task(f"test-{test}", f"test --test {test}")
        replace_or_append_task(tasks, debug_integration_test)


CONFIGURATION_NAME_SUFFIX_GDB = " (GDB+QEMU)"
CONFIGURATION_TEMPLATE_GDB = {
    "type": "gdb",
    "gdbpath": "rust-gdb",
    "request": "attach",
    "target": ":1234",
    "remote": True,
    "stopAtEntry": "_start",
    "internalConsoleOptions": "openOnSessionStart",
    "cwd": "${workspaceRoot}",
}


def gen_gdb_config(name, executable, pre_launch_task):
    debug_target_dir = f"target/{cargo_get_target()}/debug"
    config = CONFIGURATION_TEMPLATE_GDB.copy()
    config["executable"] = f"{debug_target_dir}/{executable}"
    config["preLaunchTask"] = f"{TASK_LABEL_PREFIX}{pre_launch_task}"
    config["name"] = f"{name}{CONFIGURATION_NAME_SUFFIX_GDB}"
    return config


CONFIGURATION_NAME_SUFFIX_LLDB = " (LLDB+QEMU)"
CONFIGURATION_TEMPLATE_LLDB = {
    "type": "lldb",
    "request": "custom",
    # Need to continue on attach (see QEMU man page, option '-S')
    "processCreateCommands": ["gdb-remote localhost:1234", "c"],
}


def gen_lldb_config(name, executable, pre_launch_task):
    debug_target_dir = f"target/{cargo_get_target()}/debug"
    config = CONFIGURATION_TEMPLATE_LLDB.copy()
    config["targetCreateCommands"] = [
        f"target create ${{workspaceRoot}}/{debug_target_dir}/{executable}",
        # Set a breakpoint at the program entry
        "b _start"
    ]
    config["preLaunchTask"] = f"{TASK_LABEL_PREFIX}{pre_launch_task}"
    config["name"] = f"{name}{CONFIGURATION_NAME_SUFFIX_LLDB}"
    return config


def gen_launch_json(configurations, integration_tests, *, variants=["gdb", "lldb"]):
    package_name = cargo_get_package_name()

    if "gdb" in variants:
        # Config: attach to OS binary (running in QEMU)
        gdb_run = gen_gdb_config(package_name, package_name, "run")
        replace_or_append_configuration(configurations, gdb_run)

        # Config: attach to OS binary unittests
        gdb_unittests_bin = gen_gdb_config(
            "unittests [bin]",
            f"{package_name}-unittests-bin",
            "unittests-bin")
        replace_or_append_configuration(configurations, gdb_unittests_bin)

        # Config: attach to OS library unittests
        gdb_unittests_lib = gen_gdb_config(
            "unittests [lib]",
            f"{package_name}-unittests-lib",
            "unittests-lib")
        replace_or_append_configuration(configurations, gdb_unittests_lib)

        for test in integration_tests:
            # Config: attach to integration test `test`
            gdb_integration_test = gen_gdb_config(
                f"tests/{test}", f"{package_name}-test-{test}", f"test-{test}")
            replace_or_append_configuration(configurations, gdb_integration_test)

    if "lldb" in variants:
        # Config: attach to OS binary (running in QEMU)
        lldb_run = gen_lldb_config(package_name, package_name, "run")
        replace_or_append_configuration(configurations, lldb_run)

        # Config: attach to OS binary unittests
        lldb_unittests_bin = gen_lldb_config(
            "unittests [bin]",
            f"{package_name}-unittests-bin",
            "unittests-bin")
        replace_or_append_configuration(configurations, lldb_unittests_bin)

        # Config: attach to OS library unittests
        lldb_unittests_lib = gen_lldb_config(
            "unittests [lib]",
            f"{package_name}-unittests-lib",
            "unittests-lib")
        replace_or_append_configuration(configurations, lldb_unittests_lib)

        for test in integration_tests:
            # Config: attach to integration test `test`
            lldb_integration_test = gen_lldb_config(
                f"tests/{test}", f"{package_name}-test-{test}", f"test-{test}")
            replace_or_append_configuration(configurations, lldb_integration_test)


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
