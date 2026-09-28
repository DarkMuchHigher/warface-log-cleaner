from pathlib import Path
import json
import subprocess
import yaml

workflow = yaml.load(Path('.github/workflows/windows.yml').read_text(encoding='utf-8'), Loader=yaml.BaseLoader)
assert workflow['permissions']['contents'] == 'read'
assert 'pull_request_target' not in workflow['on']
assert workflow['jobs']['release']['permissions']['contents'] == 'write'
assert workflow['jobs']['release']['needs'] == 'build'
checked = 0
for job in workflow['jobs'].values():
    for step in job['steps']:
        if 'uses' in step:
            assert len(step['uses'].split('@')[1]) == 40
        if 'run' not in step:
            continue
        script = step['run'].replace("'", "''")
        validation = "$tokens=$null; $errors=$null; [System.Management.Automation.Language.Parser]::ParseInput('" + script + "', [ref]$tokens, [ref]$errors) | Out-Null; if ($errors.Count) { $errors | ForEach-Object {$_.Message}; exit 1 }"
        subprocess.run(['powershell', '-NoProfile', '-Command', validation], check=True)
        checked += 1
print(json.dumps({'workflow': 'valid YAML', 'powershell_steps_parsed': checked}))
