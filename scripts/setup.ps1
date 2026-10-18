$ErrorActionPreference='Stop'
$root = Split-Path $PSScriptRoot -Parent
$python = '%USERPROFILE%\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe'
& $python -m venv (Join-Path $root '.venv')
$venvPython = Join-Path $root '.venv\Scripts\python.exe'
Push-Location $root
try { & $venvPython -m pip install -r (Join-Path $root 'lockfile.txt'); & $venvPython -m pip install -e . --no-deps --no-build-isolation } finally { Pop-Location }
