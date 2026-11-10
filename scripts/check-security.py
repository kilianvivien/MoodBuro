"""Regression guard for the shell-only IPC boundary; runtime probes remain a QA gate."""
import json
from fnmatch import fnmatchcase
from pathlib import Path

root = Path(__file__).resolve().parents[1]
capabilities = list((root / 'src-tauri/capabilities').glob('*.json'))
assert len(capabilities) == 2
cap = json.loads((root / "src-tauri/capabilities/shell.json").read_text())
assert cap['webviews'] == ['shell']
assert not cap.get('windows') and not cap.get('remote')
assert cap['local'] is True
config = json.loads((root / 'src-tauri/tauri.conf.json').read_text())
assert config['app']['withGlobalTauri'] is False
assert config['app']['security']['capabilities'] == ['shell', 'popup-chrome']
popup = json.loads((root / 'src-tauri/capabilities/popup-chrome.json').read_text())
assert popup['webviews'] == ['chrome-popup-*']
assert popup['local'] is True and not popup.get('remote') and not popup.get('windows')
assert set(popup['permissions']) == {'core:event:allow-listen', 'core:event:allow-unlisten', 'allow-popup-context', 'allow-popup-nav'}
assert not any(permission.startswith('allow-popup-') for permission in cap['permissions'])
assert 'AppManifest::new().commands(' in (root / 'src-tauri/build.rs').read_text()
commands = (root / 'src-tauri/src/commands.rs').read_text()
assert commands.count('    shell(&webview)?;') + commands.count('popup_shell(&webview)?;') == commands.count('#[tauri::command]')
assert 'view.label() == format!("chrome-{}", view.window().label())' in commands
print('Local shell/popup-toolbar capability and caller guards passed; remote views have no grants.')

for label in ['site-0000', 'auth-0000', 'popup-0000-1111', 'document-0000-1111']:
    assert not any(fnmatchcase(label, pattern) for data in [cap, popup] for pattern in data['webviews'])
