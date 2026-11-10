"""Independently validate the Rust fixture: pip install vobject==0.9.9."""
from datetime import datetime, timezone
from pathlib import Path
from zoneinfo import ZoneInfo
import vobject

fixture = Path(__file__).resolve().parents[1] / "src-tauri/tests/fixtures/agenda.ics"
raw = fixture.read_bytes()
assert b"\n" not in raw.replace(b"\r\n", b"")
assert all(len(line) <= 75 for line in raw.split(b"\r\n"))
calendar = vobject.readOne(raw.decode("utf-8"))
assert calendar.validate()
assert len(calendar.vevent_list) == 1
event = calendar.vevent
assert "valarm" not in event.contents
assert event.uid.value == "73697465-7-42@moodburo.local"
assert event.summary.value == "Échéance, cours; test\\notes\nBEGIN:VALARM"
assert event.description.value == "Université\nFrançais"
assert event.url.value == "https://example.org/campus/calendar/view.php?view=day&time=1792888200"
start = event.dtstart.value
end = event.dtend.value
assert start == datetime(2026, 10, 25, 0, 30, tzinfo=timezone.utc)
assert (end - start).total_seconds() == 7200
# Fall-back crosses from summer to winter time without shifting the instants.
paris = ZoneInfo("Europe/Paris")
assert start.astimezone(paris).utcoffset().total_seconds() == 7200
assert end.astimezone(paris).utcoffset().total_seconds() == 3600
assert vobject.readOne(calendar.serialize()).vevent.summary.value == event.summary.value
assert vobject.readOne(calendar.serialize()).vevent.dtstart.value == start
print("ICS parser validation passed: UTF-8, escaping, fixed URL, UID, UTC and DST.")
