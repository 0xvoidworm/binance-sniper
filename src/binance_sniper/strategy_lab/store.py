from __future__ import annotations

import json

from binance_sniper.research.store import ResearchStore, encoded


class LabStore(ResearchStore):
    def __init__(self,path):
        super().__init__(path)
        self.db.executescript('''
        CREATE TABLE IF NOT EXISTS lab_trials(id TEXT PRIMARY KEY, dataset TEXT NOT NULL, specification TEXT NOT NULL, status TEXT NOT NULL, report TEXT);
        CREATE TABLE IF NOT EXISTS lab_gates(trial TEXT, variant TEXT, symbol TEXT, day INTEGER, gate TEXT, count INTEGER, PRIMARY KEY(trial,variant,symbol,day,gate));
        CREATE TABLE IF NOT EXISTS lab_progress(trial TEXT, symbol TEXT, status TEXT, PRIMARY KEY(trial,symbol));
        ''')

    def trial(self,key,dataset,spec):
        with self.db:
            self.db.execute("INSERT OR IGNORE INTO lab_trials VALUES(?,?,?,'RUNNING',NULL)",(key,dataset,encoded(spec)))

    def finish_trial(self,key,report):
        with self.db:
            self.db.execute("UPDATE lab_trials SET status='COMPLETED',report=? WHERE id=?",(encoded(report),key))

    def histories(self,run):
        return [json.loads(r[0]) for r in self.db.execute('SELECT payload FROM outcomes WHERE run=? ORDER BY id',(run,))]
