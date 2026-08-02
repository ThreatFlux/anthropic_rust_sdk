# Security policy

## Reporting a vulnerability

Do not open a public issue for a suspected vulnerability. Use
[GitHub's private vulnerability reporting form](https://github.com/ThreatFlux/anthropic_rust_sdk/security/advisories/new)
to contact the maintainers privately.

Include, when available:

- the affected crate version or Git commit;
- the affected module, method, or endpoint;
- impact and realistic attack prerequisites;
- minimal reproduction steps or a proof of concept;
- suggested remediation or relevant upstream guidance; and
- whether the issue is already public or under active exploitation.

Remove Anthropic API keys, admin keys, bearer tokens, customer data, and other
secrets from every report. If a credential was exposed, revoke or rotate it
before reporting the SDK issue.

The maintainers will use the private advisory to coordinate investigation,
credit, remediation, and disclosure. Response and release timing depend on
severity and maintainer availability; this community project does not promise a
fixed security-response SLA.

## Supported versions

Security fixes are normally developed on `main` and released through the active
crate line. Older releases might not receive backports. Check the
[latest release](https://github.com/ThreatFlux/anthropic_rust_sdk/releases/latest)
and reproduce against it before reporting when practical.

## In scope

Examples of SDK security issues include:

- credentials exposed through SDK-generated logs, errors, or request routing;
- authentication headers sent to an unintended destination;
- unsafe path handling in file upload or download helpers;
- parser behavior that enables denial of service or trust-boundary bypass;
- dependency vulnerabilities with a reachable impact in this crate; and
- documented security guarantees that the implementation does not provide.

Service-side behavior, account access, billing disputes, model output, and the
security of Anthropic-operated systems should be reported through Anthropic's
official channels unless the SDK itself causes the issue.

## Operational guidance

- Load credentials from a secret manager or protected environment variables.
- Never log `Config`: its `Debug` representation currently includes credential
  fields.
- Treat `ANTHROPIC_ADMIN_KEY` as a separate, high-privilege secret.
- Use a custom `ANTHROPIC_BASE_URL` only when the destination is trusted; the
  client sends credentials to that host.
- Redact prompts, responses, tool inputs/results, files, and headers from
  telemetry according to the application's data-handling requirements.
- Review automatic retries before using mutation endpoints, because an
  ambiguous failure can be replayed.
- Keep Rust, this crate, and its transitive dependencies current, and review
  `cargo audit` and `cargo deny` output as part of release maintenance.
