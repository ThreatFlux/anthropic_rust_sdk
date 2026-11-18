# Maintained Anthropic timestamps

The next breaking release replaces chrono with Jiff 1.0.1 across public file,
batch, admin, model, message, skill and managed-agent timestamp fields and usage
API parameters. Import `threatflux_anthropic_sdk::Timestamp` and `Date`.
Use `Timestamp::now()`, RFC3339 parsing or fallible `Timestamp::from_second`;
calendar-only usage reports use `Date::new(year, month, day)`.

JSON instants remain RFC3339 strings. Offsets and nanoseconds are preserved as
instants and formatted canonically in UTC. Fractional trailing zeros may differ
from chrono. Existing optional omission/null behavior and unknown metadata are
preserved. Jiff accepts lexical leap second 60 and normalizes it to 59 while
retaining fractional digits. Invalid dates and missing offsets fail. Calendar
dates remain `YYYY-MM-DD`. Outbound admin report queries retain explicit
`+00:00` UTC offsets, and the existing HTTP query regressions remain intact.
Jiff's `as_second()` truncates negative fractional instants toward zero, while
chrono's `timestamp()` floors them. Consumers relying on the latter rounding
must implement it explicitly; RFC3339 instants retain their subsecond precision.

Rate-limit reset headers use fallible Unix-second conversion. An unrepresentable
reset is ignored rather than replaced by a fabricated current time. Valid
future reset delays retain the one-minute cap and explicit numeric Retry-After
retains priority, including values greater than one minute.

No chrono advisory exception is added. GitHub qualifies public-model/date/header
regressions, API mock transports, all Cargo features, MSRV, security and every
configured release channel. Countermeasure consumers must adopt the published
breaking SDK version and its new public types after publication is verified.
