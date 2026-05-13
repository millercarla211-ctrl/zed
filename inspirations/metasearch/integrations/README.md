# Integrations

This folder is reserved for external host adapters and deployment glue that embed the `metasearch` workspace.

Current recommendation:

- integrate against the Rust crates directly when you are inside a Rust host
- use the HTTP API only when you need process isolation or non-Rust consumers

The stable integration references for this workspace are:

- [F:\flow\metasearch\README.md](/F:/flow/metasearch/README.md)
- [F:\flow\metasearch\INTEGRATION_GUIDE.md](/F:/flow/metasearch/INTEGRATION_GUIDE.md)
- [F:\flow\metasearch\docs\PRODUCTION_READY.md](/F:/flow/metasearch/docs/PRODUCTION_READY.md)

This folder intentionally does not vendor other metasearch projects. Keep it focused on adapters, wrappers, and deployment-specific code for this workspace.
