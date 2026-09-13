import ast
from pathlib import Path


def test_source_has_no_duplicate_definitions_or_statement_after_return():
    problems=[]
    for path in Path('src').rglob('*.py'):
        tree=ast.parse(path.read_text(encoding='utf-8'),filename=str(path))
        for node in ast.walk(tree):
            body=getattr(node,'body',None)
            if not isinstance(body,list): continue
            names={}
            for statement in body:
                if isinstance(statement,(ast.FunctionDef,ast.AsyncFunctionDef,ast.ClassDef)):
                    if statement.name in names: problems.append(f'{path}:{statement.lineno}: duplicate {statement.name}')
                    names[statement.name]=statement.lineno
            for previous,current in zip(body,body[1:]):
                if isinstance(previous,(ast.Return,ast.Raise)):
                    problems.append(f'{path}:{current.lineno}: unreachable statement after {type(previous).__name__}')
    assert problems==[]


def test_dashboard_has_no_fixed_trigger_coordinate_or_external_innerhtml():
    html=Path('web/index.html').read_text(encoding='utf-8') + Path('web/assets/terminal.js').read_text(encoding='utf-8')
    assert "c.width*.72,185" not in html
    assert '.innerHTML' not in html
    assert 'time->x' not in html  # mappings are executable functions, not placeholder prose
    assert "const x=t=>" in html and "const y=v=>" in html


def test_process_scripts_use_exact_project_marker_and_no_wildcard_kill():
    run=Path('scripts/run.ps1').read_text(encoding='utf-8')
    stop=Path('scripts/stop.ps1').read_text(encoding='utf-8')
    assert '--app-dir $root' in run
    assert '.Contains($root)' in run and '.Contains($root)' in stop
    assert "CommandLine -match 'binance_sniper.app:app'" not in stop
    assert 'Where-Object { $_.CommandLine' not in stop
    assert '$pid =' not in stop.lower()
