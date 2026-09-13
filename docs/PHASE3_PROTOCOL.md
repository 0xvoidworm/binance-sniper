# Phase 3 protocol — registered before evaluating new outcomes

Registered 2026-09-13. Baseline commit: f135d3d. This is bounded hypothesis testing, not a promise of profitable trading. No exchange mutations are permitted.

## Data separation

The known 2026-06-15 → 2026-09-13 dataset is EXPLORATORY only and cannot be called unseen again. New research uses the preceding 360 days, 2025-06-20 → 2026-06-15 UTC (end exclusive), six existing pairs, 220 warmup bars per H4/H1/M15/M5 feed. This is retrospective unseen-to-this-project evidence, not genuine forward/live validation; present-day universe selection remains a bias.

- Initial calibration: first 120 days.
- Three expanding-window walk-forward tests: days 120–180, 180–240, 240–300. Each candidate is selected using only outcomes strictly before its test boundary.
- Final sealed holdout: days 300–360. Select one candidate using the first 300 days; do not reselect after seeing the final 60-day result.
- Purge a full eight-M5-bar horizon before boundaries and embargo eight bars after boundaries for every variant. Never use a trade whose outcome crosses the requested training/test window.

## Eight fixed variants (no parameter grid or hidden extra trials)

1. P15_T1_4: M15 expansion, 35–65% retracement area, 1R target, four M5 bars.
2. P15_T15_4: same geometry, 1.5R target.
3. P60_T1_4: H1 expansion version, 1R/four bars.
4. RETEST_T1_4: M15 breakout of previous six bars, retest broken level, 1R/four bars.
5. SWEEP_T1_4: M5 sweep/reclaim of previous twelve-bar extreme aligned to H1, 1R/four bars.
6. SWEEP_T15_4: same sweep, 1.5R/four bars.
7. P15_T1_8: explicit horizon ablation, eight bars, not labelled a four-bar sniper.
8. RETEST_T1_8: eight-bar breakout/retest ablation.

Common causal rules: closed candles only; H1 EMA20/50 plus EMA20 slope establishes direction, opposite H4 EMA trend vetoes it (neutral H4 is not automatically blocked). Expansion range >=1.1 prior ATR14, directional body >=55% and relative volume >=1.2 prior mean20, close breaks prior six-bar extreme. Areas expire after twelve M5 bars; cancel on context reversal, invalidation, or gap. M5 trigger requires directional body >=40%, close beyond previous bar's directional extreme and area midpoint. Stops sit beyond observed pullback/sweep extreme with 0.15 ATR5 buffer and at least 0.8 ATR5 distance. Target is capped by known structure, with >=0.8R remaining room. Estimated base round-trip fee+slippage may consume at most 0.35 reference R. This gate is a testable cost feasibility assumption, not position sizing.

Sweep: excursion beyond the previous twelve-bar low/high by >=0.1 prior ATR5; close returns inside that range, directional body >=40%, close location >=60% toward trade direction. No same-symbol overlapping signals within a variant's maximum horizon; at most one signal per area. Every signal includes frozen area, context, costs, and reason features.

## Measurement and selection

Use the Phase 2 next-open model with base fee/slippage 5/2 bps per side and stress 7.5/5 bps. Correct any implementation error transparently, add tests, keep earlier reports rather than rewriting them. Funding at uncertain entry/exit boundaries is conservative. Compare matched next-open baseline on the new data. R is event risk-distance, never account ROI.

Rank candidates using training-only one-sided 95% lower confidence bound of stress mean R, then mean net R, then ID. Confidence uses deterministic bootstrap of calendar-aligned seven-day blocks with all symbols in the same block, preserving within-block cross-pair correlation. Require >=50 complete samples and >=3 symbols for ranking; if none qualify report a diagnostic choice only. Record every attempted variant and all statuses, not just a winner.

Observer-promotion gate: training and final holdout each >=100 completed events, positive base mean and stress mean, final holdout lower 95% base mean bound >0, final profit factor >=1.2, >=4 symbols with >=10 holdout samples, aggregate walk-forward base mean >0 and >=2/3 positive folds. Failure means NO_PROMOTION, not permission to relax requirements after the fact. Passing permits read-only shadow observation only, not live orders or a guarantee of edge.

Deliver: tested strategy modules, reproducible data/report hashes and selection decisions, visual area evidence, baseline comparison, retained live baseline, and a separately labelled research/shadow panel. Limitations and negative results must be reported plainly.
