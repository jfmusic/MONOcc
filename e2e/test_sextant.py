"""end-to-end behavioral contract, ported from the upstream test_ambit.py.

these drive the controller against the in-memory fake device through the
headless binary and assert on the dumped device state. the timing-sensitive
cases use generous sleeps; the exact dropped-message counts from upstream are
not asserted because they depend on precise scheduling.
"""

import os

from conftest import ASSETS, REPO_ROOT, run_cli, run_cli_status

FIXTURES = REPO_ROOT / "e2e" / "fixtures"


def test_devices_lists_the_simulator():
    # the in-memory fake is always selectable, even with no hardware attached.
    out = run_cli("devices")
    assert "simulator" in out
    assert "in-memory fake" in out


def test_profiles_lists_available_sets():
    # the `profiles` command surfaces the sets in the configuration home (seeded
    # by `init`) that are selectable with --profiles.
    out = run_cli("profiles")
    assert "multifunction-buttons" in out
    assert "test-behaviors" in out


def test_default_layout_geometry(session):
    s = session("--fake-layout", "expertkit")
    s.sleep(0.5)
    s.wait()
    d = s.dump()

    assert d["connected"] == 8
    assert d["screen_string"] == "SEXTANT"
    # the default profile shows the clean black slot-23 icon.
    assert d["screen_display"] == 23
    assert d["screen_display_values"] == [23]

    assert d["orientations"] == {
        "1": 0, "2": 0, "3": 270, "4": 270,
        "5": 90, "6": 180, "7": 90, "8": 180,
    }
    assert d["slots"]["1"] == [0, 0]
    assert d["slots"]["2"] == [0, -1]
    assert d["slots"]["3"] == [1, -1]
    assert d["slots"]["5"] == [-1, -1]
    assert d["flips"]["4"] is False
    assert d["flips"]["7"] is True

    for index in range(1, 9):
        assert d["leds"][str(index)] == [255, 255, 255]
    assert d["failed_writes"] == 0
    assert d["dropped_leds"] == 0


def test_default_input_updates_values(session):
    s = session("--fake-layout", "expertkit", "--echo")
    # well past the 1.5s connect settle window, so slider echoes are live.
    s.sleep(2.5)

    s.rotation_right(5, 1)
    s.rotation_right(5, 2)
    s.rotation_right(5, 2)
    s.rotation_left(5, 1)
    s.rotation_left(5, 1)
    s.pressed(5)
    s.released(5)
    s.slide_up(7, 4)
    s.slide_up(7, 1)  # cumulative 4 + 1 = 5
    s.sleep(2.0)
    s.wait()
    d = s.dump()

    assert d["values"]["5"] == [0, 0, 0, 3, 0, 0, 0, 0]
    assert d["values"]["7"] == [5, 0, 0, 0, 0, 0, 0, 0]
    # echo mode shows the last input value on the screen.
    assert d["screen_string"] == "Slider: 5"


def test_multifunction_buttons(session):
    s = session("--fake-layout", "expertkit", "--profiles", "multifunction-buttons")
    s.sleep(0.6)
    assert s.dump()["screen_string"] == "META"

    # dial 2 left three times -> setColorGreen accumulates 255 down to 231.
    s.rotation_left(2, 8)
    s.rotation_left(2, 8)
    s.rotation_left(2, 16)
    s.sleep(1.0)
    s.wait()
    d = s.dump()
    assert d["screen_string"] == "Green: 231"
    for index in range(1, 9):
        assert d["leds"][str(index)] == [255, 231, 255]

    # button 6 -> previous profile (meta wraps to home).
    s.pressed(6)
    s.released(6)
    s.sleep(1.0)
    assert s.dump()["screen_string"] == "HOME"

    # button 8 -> next profile (home wraps to meta).
    s.pressed(8)
    s.released(8)
    s.sleep(1.0)
    assert s.dump()["screen_string"] == "META"


def test_cycle_mapping_selects_color_control(session):
    s = session("--fake-layout", "expertkit", "--profiles", "multifunction-dial")
    s.sleep(0.6)
    assert s.dump()["screen_string"] == "META"

    # button 8 cycles which color the target component controls.
    s.pressed(8)
    s.released(8)
    s.sleep(0.7)
    assert s.dump()["screen_string"] == "Green"
    s.pressed(8)
    s.released(8)
    s.sleep(0.7)
    assert s.dump()["screen_string"] == "Blue"


def test_behaviors(session):
    s = session("--fake-layout", "prokit", "--profiles", "test-behaviors")
    s.sleep(0.6)
    d = s.dump()
    assert d["connected"] == 15
    assert d["screen_string"] == "TEST"

    # dial 7: ACCUMULATE (rotation steps by the threshold, press adds the value).
    s.rotation_left(7, 8)
    s.rotation_left(7, 8)
    s.sleep(0.7)
    assert s.dump()["screen_string"] == "A: -2"
    s.pressed(7)
    s.released(7)
    s.sleep(0.7)
    assert s.dump()["screen_string"] == "A: 998"
    s.rotation_right(7, 8)
    s.rotation_right(7, 8)
    s.sleep(0.7)
    assert s.dump()["screen_string"] == "A: 1000"

    # slider 9: TRIGGER reports the raw value, saturating at 255.
    for amount in (4, 16, 128, 64, 64):
        s.slide_up(9, amount)
    s.sleep(1.0)
    assert s.dump()["screen_string"] == "T: 255"

    # slider 11: CYCLE maps the slider range onto six items.
    s.slide_up(11, 255)
    s.sleep(0.5)
    assert s.dump()["screen_string"] == "C: SIX"
    for expect in ("FIVE", "FOUR", "THREE", "TWO"):
        s.slide_down(11, 40)
        s.sleep(0.5)
        assert s.dump()["screen_string"] == f"C: {expect}"
    s.slide_down(11, 55)
    s.sleep(0.5)
    assert s.dump()["screen_string"] == "C: ONE"

    # button 4: TRIGGER on press and release.
    s.pressed(4)
    s.sleep(0.5)
    assert s.dump()["screen_string"] == "T: 1"
    s.released(4)
    s.sleep(0.5)
    assert s.dump()["screen_string"] == "T: 0"

    # button 8: CYCLE advances one item per press and loops.
    for expect in ("TWO", "THREE", "FOUR", "FIVE", "SIX", "ONE"):
        s.pressed(8)
        s.released(8)
        s.sleep(0.6)
        assert s.dump()["screen_string"] == f"C: {expect}"

    # after the screen-reset timeout the base returns to the profile title.
    s.sleep(4.0)
    assert s.dump()["screen_string"] == "TEST"
    assert s.dump()["failed_writes"] == 0


def test_configure_images_match_assets(session):
    s = session("--fake-layout", "base-only")
    s.sleep(0.5)
    s.wait()
    s.configure_images()
    s.sleep(0.5)
    s.wait()
    d = s.dump()

    # slot 23 (the black base screen) is always uploaded; slot 0 (the brand icon)
    # is too when the gitignored reference asset is present, so the count varies.
    expected = (ASSETS / "23.raw").read_bytes().hex()
    assert d["screen_images"]["23"] == expected


def test_layout_change_updates_connected(session):
    s = session("--fake-layout", "expertkit")
    s.sleep(0.5)
    assert s.dump()["connected"] == 8

    s.layout_changed("prokit")
    s.wait_layout()
    s.sleep(0.5)
    assert s.dump()["connected"] == 15


def test_slider_range_broken_firmware(session):
    s = session("--fake-layout", "expertkit", "--fake-firmware", "1.3.1")
    s.sleep(0.6)
    s.slide_up(7, 254)
    s.sleep(0.5)
    assert s.dump()["values"]["7"] == [255, 0, 0, 0, 0, 0, 0, 0]


def test_slider_range_fixed_firmware(session):
    s = session("--fake-layout", "expertkit", "--fake-firmware", "1.4.6136")
    s.sleep(0.6)
    s.slide_up(7, 254)
    s.sleep(0.5)
    assert s.dump()["values"]["7"] == [254, 0, 0, 0, 0, 0, 0, 0]
    s.slide_up(7, 1)
    s.sleep(0.5)
    assert s.dump()["values"]["7"] == [255, 0, 0, 0, 0, 0, 0, 0]


def test_keepalive_pings_when_idle(session):
    s = session("--fake-layout", "expertkit")
    s.sleep(0.6)
    s.wait()
    assert s.dump()["message_counts"]["check"] == 1

    # with no user input, the keepalive sends a second check within the timeout.
    s.sleep(6.0)
    assert s.dump()["message_counts"]["check"] == 2


def test_toggle_preview_toggles_the_base_icon(session):
    s = session("--config", str(FIXTURES / "behaviors.plp"), "--fake-layout", "prokit")
    s.wait_layout()
    s.wait()
    # the Keyboard tab loads its icon (5); togglePreview flips it to blank and back.
    assert s.dump()["screen_display"] == 5
    s.pressed(4)  # button at slot (1,0): pressed -> togglePreview
    s.released(4)
    s.sleep(0.3)
    assert s.dump()["screen_display"] == 23
    s.pressed(4)
    s.released(4)
    s.sleep(0.3)
    assert s.dump()["screen_display"] == 5


def test_long_press_fires_after_the_hold_threshold(session):
    s = session("--config", str(FIXTURES / "behaviors.plp"), "--fake-layout", "prokit")
    s.wait_layout()
    s.wait()
    # a quick tap is not a long press -> the icon is unchanged.
    s.pressed(5)  # button at slot (1,1): long_pressed -> togglePreview
    s.released(5)
    s.sleep(0.3)
    assert s.dump()["screen_display"] == 5
    # holding past the threshold registers a long press on release.
    s.pressed(5)
    s.sleep(0.6)
    s.released(5)
    s.sleep(0.3)
    assert s.dump()["screen_display"] == 23


def test_double_press_within_the_window(session):
    s = session("--config", str(FIXTURES / "behaviors.plp"), "--fake-layout", "prokit")
    s.wait_layout()
    s.wait()
    # two quick presses -> double_pressed -> togglePreview.
    s.pressed(8)  # button at slot (-1,0): double_pressed -> togglePreview
    s.released(8)
    s.pressed(8)
    s.released(8)
    s.sleep(0.3)
    assert s.dump()["screen_display"] == 23


def test_media_map_programs_consumer_hid(session):
    s = session("--config", str(FIXTURES / "behaviors.plp"), "--fake-layout", "prokit")
    s.wait_layout()
    s.wait()
    # hid / media output is device-side, programmed by `map hid` (not on connect).
    s.send("map_hid")
    s.wait()
    # PK_MEDIA_PLAYPAUSE (0xe8 = 232) is programmed onto the device hidmap.
    keys = [seg["k"] for message in s.dump()["hidmap"] for entry in message for seg in entry["s"]]
    assert 232 in keys


def test_init_mirrors_resources_and_is_idempotent(tmp_path):
    # init writes the embedded tree under $XDG_CONFIG_HOME/sextant.
    env = {"XDG_CONFIG_HOME": str(tmp_path)}
    home = tmp_path / "sextant"
    out = run_cli("init", env=env)
    assert "written" in out
    assert (home / "profiles" / "showcase" / "01-hack.plp").is_file()
    hello = home / "scripts" / "hello.sh"
    assert hello.is_file() and os.access(hello, os.X_OK)
    # a second run keeps every existing file (nothing overwritten).
    assert "0 written" in run_cli("init", env=env)


def test_device_command_requires_init(tmp_path):
    # runtime reads from the home, so a device command refuses on an empty
    # (un-initialized) home and points the user at `sextant init`.
    env = {"XDG_CONFIG_HOME": str(tmp_path)}
    result = run_cli_status("connect", "--device", "simulator", env=env)
    assert result.returncode != 0
    assert "sextant init" in result.stderr


def test_execute_command_resolves_config_script(session, tmp_path):
    # ${SEXTANT_CONFIG} in an executeCommand argv resolves to the config home, so
    # the bundled scripts run regardless of the working directory.
    env = {"XDG_CONFIG_HOME": str(tmp_path)}
    run_cli("init", env=env)  # materialize <home>/scripts/hello.sh
    s = session("--config", str(FIXTURES / "exec.plp"), "--fake-layout", "prokit", env=env)
    s.wait_layout()
    s.wait()
    s.pressed(4)  # button at slot (1,0) -> executeCommand hello.sh e2e
    s.released(4)
    s.sleep(0.5)
    # the script's stdout (`hi e2e`) is echoed onto the base screen.
    assert s.dump()["screen_string"] == "hi e2e"
