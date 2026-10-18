from fastapi.testclient import TestClient
from binance_sniper.web import build_app


class State:
    clients = set()
    account = {'auth': 'AUTH_UNAVAILABLE'}
    def snapshot(self):
        return {'symbols': {}, 'events': [], 'feed_quality': {}}


def test_dashboard_smoke():
    client = TestClient(build_app(State()))
    response = client.get('/')
    assert response.status_code == 200
    assert 'RESEARCH DESK' in response.text
    assert 'tester-terminal.js' in response.text
    assert 'NO LIVE EXECUTION' in response.text
    response = client.get('/market')
    assert 'RANGE / ENGINE' in response.text
    assert 'range-terminal.js' in response.text
    assert 'NO FIXED TP' in response.text
    for asset in ('terminal.js', 'terminal-model.js', 'terminal.css'):
        assert client.get('/assets/' + asset).status_code == 200
    replay=client.get('/api/replay')
    assert replay.status_code==200
    payload=replay.json()
    assert payload['symbols']['BTCUSDT']['status']=='COMPLETED'
    assert payload['candidates'][0]['score']>0
