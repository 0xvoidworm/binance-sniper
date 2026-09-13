# Security Model

Credentials are loaded from Windows CurrentUser DPAPI at `%LOCALAPPDATA%\CodexBinance\credentials.dpapi` into memory only. The project never creates a plaintext `.env`, never serializes credentials, and uses redacted repr/error messages.

Private requests are signed in memory and limited to an explicit GET allowlist. Mutation methods and known order endpoints are rejected before network. Redirects are disabled. The dashboard exposes only safe account status and balance summary.

Private hosts are pinned in code to `https://fapi.binance.com` and `https://api.binance.com`; arbitrary configured hosts, non-HTTPS schemes, userinfo, ports, and lookalike hostnames are rejected. Account mode uses dedicated read endpoints and separates regular and algo order counts.

`ipRestrict=false` is accepted as operator policy and shown as an informational warning. This service does not change API settings. Withdrawal, transfer, leverage, margin, position mode, order, and cancel endpoints are absent.
