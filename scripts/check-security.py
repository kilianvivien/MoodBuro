"""Regression guard for the shell-only IPC boundary; runtime probes remain a QA gate."""
import json
from pathlib import Path

root = Path(__file__).resolve().parents[1]
capabilities = list((root / 'src-tauri/capabilities').glob('*.json'))
assert len(capabilities) == 1
cap = json.loads(capabilities[0].read_text())
assert cap['webviews'] == ['shell']
assert not cap.get('windows') and not cap.get('remote')
assert cap['local'] is True
config = json.loads((root / 'src-tauri/tauri.conf.json').read_text())
assert config['app']['withGlobalTauri'] is False
assert config['app']['security']['capabilities'] == ['shell']
assert 'AppManifest::new().commands(' in (root / 'src-tauri/build.rs').read_text()
commands = (root / 'src-tauri/src/commands.rs').read_text()
assert commands.count('shell(&webview)?;') == commands.count('#[tauri::command]')
print('Shell-only capability and command caller guards passed.')
