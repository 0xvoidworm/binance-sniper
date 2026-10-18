from concurrent.futures import ThreadPoolExecutor
import sqlite3
from types import SimpleNamespace

from fastapi.testclient import TestClient
import pytest

from binance_sniper.tester.report import DEAL_HEADER, canonical, parse_report
from binance_sniper.tester.store import CANDIDATE, EvidenceStore
from binance_sniper.web import build_app


def row(values):
    return '<tr>' + ''.join(f'<td><b>{v}</b></td>' for v in values) + '</tr>'


def fixture(expert='SniperFX', profit=30, comment='ordinary', input_value='private-value'):
    settings = [('Expert:', expert), ('Symbol:', 'EURUSD'), ('Period:', 'M5 (2026.01.01 - 2026.02.01)'),
                ('Inputs:', 'SomeInput=1'), ('', f'ApiSecret={input_value}'), ('Currency:', 'USD'),
                ('Initial Deposit:', '1 000.00'), ('Leverage:', '1:100'), ('Total Net Profit:', str(profit-4)),
                ('Gross Profit:', str(profit-4)), ('Gross Loss:', '0'), ('Profit Factor:', 'inf'),
                ('Total Trades:', '1'), ('Total Deals:', '2'), ('History Quality:', '100%')]
    deals = [
        ['2026.01.01 00:00:00','1','','balance','','','','','0.00','0.00','1000.00','1000.00',comment],
        ['2026.01.02 09:00:00','2','EURUSD','buy','in','0.1','1.12345','2','-1.00','0.00','0.00','999.00',comment],
        ['2026.01.02 10:00:00','3','EURUSD','sell','out','0.1','1.12645','3','-1.00','-2.00',str(profit),str(1000+profit-4),comment],
    ]
    return ('<html><body><table>'+row(['Strategy Tester Report'])+row(['Test-Broker (Build 6184)'])+
            ''.join(row(s) for s in settings)+row(['Orders'])+row(['Deals'])+row(DEAL_HEADER)+
            ''.join(row(d) for d in deals)+row(['','','','','','','','', '-2','-2',str(profit),str(1000+profit-4),''])+
            '</table></body></html>').encode()


def context(candidate=False, **changes):
    value = {'engine_id': CANDIDATE['id'] if candidate else 'legacy-sniperfx',
             'expected_expert': CANDIDATE['name'] if candidate else 'SniperFX',
             'purpose': 'COMPATIBILITY' if candidate else 'LEGACY_PARSER_CHECK', 'run_label':'fixture',
             'version':'fixture-v1','broker_timezone':'UTC+2/3','tester_model':'REAL_TICKS',
             'execution_delay':'fixed-100ms','cost_model':'fixture-fees','dataset_id':'fixture-dataset-hash',
             'randomizer':'enabled'}
    return value | changes


ARTIFACTS = {'binary':b'fixture-only-binary','set':b'fixture-only-set','tester_config':b'fixture-config'}


def test_parser_reconciles_nonzero_commission_swap_and_deposit():
    result = parse_report(fixture())
    assert result['validation'] == 'REPORT_RECONCILED'
    assert result['metrics']['net_profit'] == result['metrics']['ledger_net'] == 26
    assert result['metrics']['final_balance'] == 1026
    assert result['metrics']['commission'] == -2 and result['metrics']['swap'] == -2
    assert result['metrics']['total_deals'] == 2 and len(result['deals']) == 3
    assert result['metrics']['profit_factor'] is None
    assert result['deals'][1]['time'] == '2026-01-02T09:00:00'
    assert result['time_basis'] == 'BROKER_LOCAL_UNCONVERTED'
    assert not result['availability']['equity_curve']
    assert not result['availability']['entry_zones']


@pytest.mark.parametrize('encoding', ['utf-8-sig','utf-16','utf-16-be'])
def test_encoding_variants(encoding):
    raw = fixture().decode().encode(encoding)
    if encoding == 'utf-16-be': raw = b'\xfe\xff' + raw
    assert parse_report(raw)['metrics']['net_profit'] == 26


@pytest.mark.parametrize('old,new', [
    (b'</html>', b''), (b'<b>Total Deals:</b>', b'<b>Gesamt:</b>'),
    (b'<b>26</b>', b'<b>NaN</b>'), (b'<b>26</b>', b'<b>26,00</b>'),
    (b'<b>26</b>', b'<b>Infinity</b>'), (b'<b>1 000.00</b>', b'<b>0</b>'),
    (b'<b>0.1</b>', b'<b>-0.1</b>'), (b'<b>1.12345</b>', b'<b>0</b>'),
    (b'<b>2026.01.02 09:00:00</b>', b'<b>not-a-time</b>'),
    (b'<b>buy</b>', b'<b>fee</b>'),
])
def test_malformed_inputs_rejected(old,new):
    with pytest.raises(ValueError): parse_report(fixture().replace(old,new))


@pytest.mark.parametrize('old,new,error', [
    (b'<b>999.00</b>',b'<b>998.00</b>','BALANCE_LEDGER_MISMATCH'),
    (b'<b>26</b>',b'<b>27</b>','NET_PROFIT_MISMATCH'),
    (b'Total Deals:</b></td><td><b>2',b'Total Deals:</b></td><td><b>3','DEAL_COUNT_MISMATCH'),
    (b'<b>2026.01.02 10:00:00</b>',b'<b>2026.01.02 08:00:00</b>','NON_CHRONOLOGICAL_DEALS'),
    (b'<b>2026.01.02 10:00:00</b>',b'<b>2026.03.02 10:00:00</b>','DEAL_OUTSIDE_REPORTED_PERIOD'),
])
def test_inconsistent_reports_are_flagged_not_plotted(old,new,error):
    report = parse_report(fixture().replace(old,new))
    assert error in report['errors']
    assert report['validation'] == 'RECONCILIATION_FAILED'
    assert not report['availability']['balance']


def test_duplicate_deal_and_summary_detected():
    text = fixture().replace(b'<b>3</b>',b'<b>2</b>')
    assert 'DUPLICATE_DEAL_ID' in parse_report(text)['errors']
    with pytest.raises(ValueError):
        parse_report(fixture().replace(b'</table>',row(['Total Deals:','2']).encode()+b'</table>'))


def test_raw_inputs_and_comments_not_exposed_or_executed():
    text = fixture(comment='<script>window.BAD=1</script>personal-account-name',input_value='never-expose-secret')
    report = parse_report(text)
    serialized = canonical(report)
    assert 'never-expose-secret' not in serialized and 'personal-account-name' not in serialized
    assert 'window.BAD' not in serialized and report['inputs_sha256']
    changed = parse_report(fixture(input_value='different-secret'))
    assert report['inputs_sha256'] != changed['inputs_sha256']


def test_read_only_empty_store_does_not_create_files(tmp_path):
    store = EvidenceStore(tmp_path/'missing'/'tester.db')
    result = store.snapshot()
    assert result['candidate_runs']==0 and result['runs']==[]
    assert result['status']=='AWAITING_CANDIDATE_REPORT'
    assert not store.path.parent.exists()


def test_atomic_restart_idempotence_and_immutable_storage(tmp_path):
    store=EvidenceStore(tmp_path/'tester.db')
    run=store.import_report(fixture(),context())
    assert store.import_report(fixture(),context()) == run
    assert EvidenceStore(store.path).get(run['id']) == run
    assert len(store.snapshot()['runs']) == 1
    assert store.snapshot()['candidate_runs']==0 and store.snapshot()['legacy_runs']==1
    with sqlite3.connect(store.path) as con:
        for query in ("DELETE FROM runs", "UPDATE runs SET payload='{}'"):
            with pytest.raises(sqlite3.IntegrityError,match='immutable'): con.execute(query)
    before=store.snapshot()
    with pytest.raises(ValueError): store.import_report(b'partial',context())
    assert store.snapshot()==before


def test_concurrent_duplicate_import_is_one_run(tmp_path):
    store=EvidenceStore(tmp_path/'tester.db')
    store.import_report(fixture(),context())
    with ThreadPoolExecutor(max_workers=4) as pool:
        runs=list(pool.map(lambda _:store.import_report(fixture(),context()),range(8)))
    assert len({r['id'] for r in runs})==1
    assert len(store.snapshot()['runs'])==1


def test_identity_rejection_prevents_legacy_candidate_mix(tmp_path):
    store=EvidenceStore(tmp_path/'tester.db')
    with pytest.raises(ValueError,match='expert'): store.import_report(fixture(),context(candidate=True))
    with pytest.raises(ValueError): store.import_report(fixture(),context(engine_id='mystery'))
    with pytest.raises(ValueError): store.import_report(fixture(),context(api_key='secret'))
    with pytest.raises(ValueError): store.import_report(fixture(),context(run_label='line\nbreak'))
    assert not store.path.exists()


def test_compare_blocks_missing_context_duplicate_or_changed_setup(tmp_path):
    store=EvidenceStore(tmp_path/'tester.db')
    a=store.import_report(fixture(),context())
    b=store.import_report(fixture(profit=32),context())
    result=store.compare(a['id'],b['id'])
    assert result['status']=='NOT_COMPARABLE' and 'INCOMPLETE_REPEAT_PROVENANCE' in result['reasons']
    assert result['delta_right_minus_left']['net_profit']==2
    assert 'SAME_IMPORT_NOT_INDEPENDENT_RUNS' in store.compare(a['id'],a['id'])['reasons']
    c=store.import_report(fixture(profit=32),context(tester_model='OHLC_M1'),ARTIFACTS)
    assert 'DIFFERENT_TEST_CONTEXT_OR_ARTIFACTS' in store.compare(b['id'],c['id'])['reasons']


def test_compare_reports_same_and_different_outputs_without_claiming_edge(tmp_path):
    store=EvidenceStore(tmp_path/'tester.db')
    a=store.import_report(fixture(),context(),ARTIFACTS|{'tester_log':b'run1'})
    b=store.import_report(fixture(),context(),ARTIFACTS|{'tester_log':b'run2'})
    c=store.import_report(fixture(profit=32),context(),ARTIFACTS|{'tester_log':b'run3'})
    assert store.compare(a['id'],b['id'])['status']=='MATCH_REPORTED_OUTPUT'
    assert store.compare(a['id'],c['id'])['status']=='DIFFERENT_REPORTED_OUTPUT'
    assert 'future profitability' in store.compare(a['id'],c['id'])['limitation']
    d=store.import_report(fixture(),context(run_label='renamed'),ARTIFACTS|{'tester_log':b'run1'})
    assert 'IDENTICAL_REPORT_WITHOUT_DISTINCT_RUN_LOGS' in store.compare(a['id'],d['id'])['reasons']


def test_failed_reconciliation_blocks_comparison(tmp_path):
    store=EvidenceStore(tmp_path/'tester.db')
    a=store.import_report(fixture(),context(),ARTIFACTS)
    b=store.import_report(fixture().replace(b'<b>999.00</b>',b'<b>900.00</b>'),context(),ARTIFACTS)
    assert 'REPORT_RECONCILIATION_FAILED' in store.compare(a['id'],b['id'])['reasons']


def test_web_read_only_routes_and_source_separation(tmp_path):
    store=EvidenceStore(tmp_path/'tester.db')
    state=SimpleNamespace(tester_store=store,clients=set())
    client=TestClient(build_app(state))
    assert client.get('/').status_code==200 and 'RESEARCH DESK' in client.get('/').text
    assert 'RANGE / ENGINE' in client.get('/market').text
    assert client.get('/api/tester').json()['candidate_runs']==0
    run=store.import_report(fixture(),context())
    result=client.get('/api/tester/runs/'+run['id'])
    assert result.status_code==200 and 'private-value' not in result.text
    assert client.get('/api/tester').json()['candidate_runs']==0
    assert client.post('/api/tester',json={'order':'buy'}).status_code==405
    assert client.get('/api/tester/runs/..%5Ccredentials.dpapi').status_code==404
    assert client.get('/api/tester/compare?left=../x&right=a').status_code==422
    assert client.get('/api/tester/raw/'+run['id']).status_code==404
    for route in client.app.routes:
        if route.path.startswith('/api/tester'): assert route.methods=={'GET'}
    for asset in ('tester-terminal.js','tester-model.js','tester-terminal.css'):
        assert client.get('/assets/'+asset).status_code==200


def test_corrupt_database_fails_closed_without_leaking_details(tmp_path):
    store=EvidenceStore(tmp_path/'tester.db')
    store.import_report(fixture(),context())
    with sqlite3.connect(store.path) as con:
        con.execute('DROP TRIGGER immutable_update')
        con.execute("UPDATE runs SET payload='secret-account-data'")
    client=TestClient(build_app(SimpleNamespace(tester_store=store)))
    result=client.get('/api/tester')
    assert result.status_code==503 and 'secret-account-data' not in result.text


def test_candidate_report_has_own_count_not_vendor_claim(tmp_path):
    store=EvidenceStore(tmp_path/'tester.db')
    store.import_report(fixture(expert=CANDIDATE['name']),context(candidate=True))
    snapshot=store.snapshot()
    assert snapshot['candidate_runs']==1 and snapshot['legacy_runs']==0
    assert snapshot['candidate']['profitability']=='NOT_ESTABLISHED'
