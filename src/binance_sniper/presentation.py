"""Read-only explanation of scanner state; no order or signal generation."""
from __future__ import annotations


def decision_view(bias, efficiency, impulse, zone, m5, settings, h4_count):
    # H4's efficiency threshold is exported by its strategy module.
    from binance_sniper.analysis.regime import MIN_EFFICIENCY

    if h4_count < 40:
        explanation = f"H4 baru {h4_count}/40 candle tertutup."
    elif efficiency < MIN_EFFICIENCY:
        explanation = f"Efisiensi H4 {efficiency:.3f} < {MIN_EFFICIENCY:.2f}; arah belum cukup konsisten."
    elif bias == "NEUTRAL":
        explanation = "Swing high dan low H4 belum membentuk arah yang sama atau belum lengkap."
    else:
        explanation = f"Struktur H4 {bias}; efisiensi {efficiency:.3f}."
    gates = [
        {"tf": "4h", "label": "Arah pasar", "state": "PASS" if bias != "NEUTRAL" else "WAIT", "detail": explanation},
        {"tf": "1h", "label": "Impulse", "state": "PASS" if impulse else ("WAIT" if bias != "NEUTRAL" else "BLOCKED"), "detail": "Impulse terkonfirmasi." if impulse else "Menunggu ekspansi dari pivot searah H4."},
        {"tf": "15m", "label": "Area pullback", "state": "PASS" if zone else ("WAIT" if impulse else "BLOCKED"), "detail": f"Pullback {settings.pullback_min_depth:.0%}–{settings.pullback_max_depth:.0%}, maksimal {settings.pullback_max_bars} candle M15."},
        {"tf": "5m", "label": "Pemicu entry", "state": "BLOCKED", "detail": f"Sentuh zona, reclaim, break candle sebelumnya; body ≥ {settings.trigger_min_body_fraction:.0%}."},
    ]
    stage, headline, next_condition = "WAIT_H4", "Menunggu arah H4", explanation
    if bias != "NEUTRAL":
        stage, headline, next_condition = "WAIT_H1", "Menunggu impulse H1", gates[1]["detail"]
    if impulse:
        stage, headline, next_condition = "WAIT_M15", "Menunggu pullback M15", gates[2]["detail"]
    if zone:
        gates[2]["detail"] = f"Zona {zone.lower:g}–{zone.upper:g}; arah setup {zone.direction}."
        gates[3]["state"] = "WAIT"
        stage = zone.state.value
        headline = {
            "DETECTED": "Area terdeteksi", "ARMED": "Menunggu harga masuk area",
            "TOUCHED": "Area tersentuh · menunggu pemicu M5",
            "TRIGGERED": "Sinyal tercatat · observasi empat candle",
            "COMPLETED": "Observasi setup selesai", "INVALIDATED": "Setup gugur",
            "EXPIRED": "Masa berlaku area habis",
        }[stage]
        next_condition = gates[3]["detail"]
        if stage in {"TRIGGERED", "COMPLETED"}:
            gates[3]["state"] = "PASS"
            next_condition = "Pantau hasil setelah sinyal; level yang tampil adalah referensi observasi."
        if stage in {"INVALIDATED", "EXPIRED"}:
            gates[3]["state"] = "FAIL"
            next_condition = "Setup diarsipkan. Menunggu rangkaian setup berikutnya."
        if not impulse or zone.source_impulse != impulse["id"]:
            gates[1] = {"tf": "1h", "label": "Impulse asal", "state": "PASS", "detail": "Memakai impulse terdahulu; arah H4 di atas adalah kondisi sekarang."}
    return {
        "stage": stage, "headline": headline, "next_condition": next_condition,
        "gates": gates, "efficiency": efficiency, "minimum_efficiency": MIN_EFFICIENCY,
        "setup_id": zone.zone_id if zone else None,
        "evaluated_bar_at": m5[-1].close_time if m5 else None,
        "invalidation": zone.invalidation if zone else None,
        "expires_at": zone.expires_at if zone else None,
    }
