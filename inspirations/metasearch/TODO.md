# TODO

These are follow-up improvements, not blockers for the current source-complete handoff.

- Replace the lightweight fixed-window limiter with a richer per-client limiter if public traffic patterns require it.
- Add explicit per-engine configuration loading for instance-based adapters that currently default to empty base URLs or optional API keys.
- Add offline catalog generation for the engine registry so docs do not rely on hand-maintained engine counts.
- Audit high-churn scraper engines and move fragile selectors behind focused regression fixtures.
