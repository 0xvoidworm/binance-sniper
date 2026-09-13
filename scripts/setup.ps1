$ErrorActionPreference='Stop'
$root = Split-Path $PSScriptRoot -Parent
$python = 'C:\Users\yuri\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe'
& $python -m venv (Join-Path $root '.venv')
$venvPython = Join-Path $root '.venv\Scripts\python.exe'
& $venvPython -m pip install --upgrade pip
Push-Location $root
try { & $venvPython -m pip install -e '.[test]' } finally { Pop-Location }
& $venvPython -m pip freeze | Out-File (Join-Path $root 'lockfile.txt') -Encoding utf8
