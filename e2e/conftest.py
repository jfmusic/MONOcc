"""shared harness for the sextant end-to-end suite.

each test drives the compiled `sextant tools headless` binary over a line protocol:
input commands go to stdin, and a `dump` command prints one json line of device
and controller state on stdout. the `session` fixture builds the binary once and
hands out `Session` objects that are torn down at the end of each test.
"""

import json
import os
import subprocess
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parent.parent
# screen-image assets are separated by device type; the indexed ones are palettegear's.
ASSETS = REPO_ROOT / "crates" / "sextant-core" / "resources" / "assets" / "palettegear"


def _binary() -> str:
    override = os.environ.get("SEXTANT_BIN")
    if override:
        return override
    return str(REPO_ROOT / "target" / "debug" / "sextant")


@pytest.fixture(scope="session", autouse=True)
def _build_binary():
    # build once per test session so `uv run pytest` works on its own.
    if not os.environ.get("SEXTANT_BIN"):
        subprocess.run(
            ["cargo", "build", "-p", "sextant-cli"],
            cwd=REPO_ROOT,
            check=True,
        )


@pytest.fixture(scope="session", autouse=True)
def _init_config_home(_build_binary, tmp_path_factory):
    # runtime reads profiles + screen assets from the configuration home, so seed
    # one for the whole suite and point every child process at it. tests that need
    # isolation pass their own XDG_CONFIG_HOME to override this.
    home = tmp_path_factory.mktemp("config")
    run_cli("init", env={"XDG_CONFIG_HOME": str(home)})
    os.environ["XDG_CONFIG_HOME"] = str(home)


def _merged_env(env: dict | None) -> dict | None:
    return None if env is None else {**os.environ, **env}


def run_cli(*args: str, env: dict | None = None) -> str:
    """run a one-shot `sextant` subcommand and return its stdout."""
    return subprocess.run(
        [_binary(), *args],
        capture_output=True,
        text=True,
        check=True,
        env=_merged_env(env),
    ).stdout


def run_cli_status(*args: str, env: dict | None = None) -> subprocess.CompletedProcess:
    """run a subcommand without raising; return the completed process (for the
    failure cases, e.g. asserting on a nonzero exit and stderr)."""
    return subprocess.run(
        [_binary(), *args],
        capture_output=True,
        text=True,
        timeout=30,
        env=_merged_env(env),
    )


class Session:
    """one running `sextant tools headless` process driven over its line protocol."""

    def __init__(self, *args: str, env: dict | None = None):
        self.proc = subprocess.Popen(
            [_binary(), "tools", "headless", *args],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            text=True,
            bufsize=1,
            env=_merged_env(env),
        )

    def send(self, command: str) -> None:
        assert self.proc.stdin is not None
        self.proc.stdin.write(command + "\n")
        self.proc.stdin.flush()

    def dump(self) -> dict:
        self.send("dump")
        assert self.proc.stdout is not None
        line = self.proc.stdout.readline()
        if not line:
            raise RuntimeError("headless process produced no dump (it likely exited)")
        return json.loads(line)

    # input + control helpers
    def sleep(self, seconds: float) -> None:
        self.send(f"sleep {seconds}")

    def wait(self) -> None:
        self.send("wait")

    def wait_layout(self) -> None:
        self.send("wait_layout")

    def pressed(self, index: int) -> None:
        self.send(f"pressed {index}")

    def released(self, index: int) -> None:
        self.send(f"released {index}")

    def slide_up(self, index: int, amount: int) -> None:
        self.send(f"slide_up {index} {amount}")

    def slide_down(self, index: int, amount: int) -> None:
        self.send(f"slide_down {index} {amount}")

    def rotation_left(self, index: int, amount: int) -> None:
        self.send(f"rotation_left {index} {amount}")

    def rotation_right(self, index: int, amount: int) -> None:
        self.send(f"rotation_right {index} {amount}")

    def screen_string(self, text: str) -> None:
        self.send(f"screen_string {text}")

    def layout_changed(self, name: str) -> None:
        self.send(f"layout_changed {name}")

    def configure_images(self) -> None:
        self.send("configure_images")

    def close(self) -> None:
        try:
            self.send("quit")
        except (BrokenPipeError, ValueError):
            pass
        try:
            self.proc.wait(timeout=15)
        except subprocess.TimeoutExpired:
            self.proc.kill()


@pytest.fixture
def session():
    started = []

    def make(*args: str, env: dict | None = None) -> Session:
        s = Session(*args, env=env)
        started.append(s)
        return s

    yield make

    for s in started:
        s.close()
