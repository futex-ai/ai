# Provider-Reported Model Cost Protocol

## Status And Purpose

Planned as part of the
[OpenRouter implementation plan](../../plans/openrouter-model-provider.md).
The current `ModelUsage` and pricing wrapper do not yet implement these
additions. Preserve provider-reported account charges independently of
configured estimates, without inventing token prices or treating absence as
free usage.

## Shared Contract

`ai-interface` adds `ModelUsage::provider_cost`, an optional
`ProviderReportedCost` with:

- `provider: ProviderKind`, identifying the API reporting the charge;
- `account_charge_microusd: Option<u64>`, the successful request's reported
  debit when supplied;
- `upstream_inference_cost_microusd: Option<u64>`, separate informational
  upstream cost when the reporting API supplies it.

The envelope and both amount fields default to `None` during deserialization
and are omitted when absent. Adapters emit an envelope when either amount is
available, including zero, and `None` when both are absent. An upstream-only
report retains `account_charge_microusd: None`; an empty envelope received from
a consumer also supplies no account charge. Existing provider constructors
and fixtures set the envelope to `None`; their token accounting and configured
prices remain unchanged.

The existing `estimated_cost_microusd` and `cost_lines` describe configured
estimates. A provider aggregate charge is never assigned to a fabricated input
or output token line. It is also never summed with the upstream informational
cost. The reported charge can legitimately be zero, including when the API
charges upstream inference separately from the account being metered.

Add pure `ModelUsage::effective_cost()` returning `ModelCostSummary` with an
optional amount and `ModelCostSource::{ProviderReported, ConfiguredEstimate,
Unknown}`. Selection is deterministic:

1. A present `account_charge_microusd` wins, including a reported zero.
   An envelope containing only upstream cost, or neither amount, proceeds to
   the configured-estimate rules without promoting upstream cost to a debit.
2. Otherwise, a nonempty set of cost lines covering every nonzero token bucket
   exactly once, with matching quantities and known amounts on every line,
   yields the configured estimate using their sum. Explicitly free lines
   qualify. Integer addition saturates at `u64::MAX`. Missing or duplicate
   bucket coverage is unknown.
3. Otherwise, return no amount and `Unknown`. An empty or partially priced
   line set, or a legacy scalar zero alone, cannot establish free usage.

Consumers must not sum aggregate and line-item costs or select an amount by
testing whether it is nonzero. An estimate may be retained for comparison,
but the effective cost accessor is the supported total-selection boundary.

## Pricing Wrapper

`ai-models-core::price_usage` and both `UsagePricingModel` entrypoints preserve
`provider_cost` exactly. They may calculate configured estimates in the
existing scalar/lines fields even when a provider charge exists. Applying one
or several pricing wrappers must not replace, erase, double count, or
reinterpret an authoritative charge. Wrappers forward completion events
unchanged and never make network calls to obtain prices or billing data.

Change `price_usage`'s existing `.sum::<u64>()` scalar aggregation to a fold
using `saturating_add`, matching `effective_cost()` at `u64::MAX`. This is a
planned behavior change: the current sum can panic with overflow checks or
wrap without them. Individual line pricing already saturates and remains
unchanged. Partial estimates still sum known lines into the legacy scalar,
but remain `Unknown` through the accessor when bucket coverage is incomplete.

The wrapper's existing line measurement states keep their current meaning:
token quantity and configured price quality. The new summary source records
whether the selected aggregate came from a provider report or configuration.
This change does not relabel existing token lines as actual invoice charges.

## OpenRouter Mapping

OpenRouter returns `usage.cost` for the account debit and a separate
`usage.cost_details.upstream_inference_cost`. Usage is supplied in the final
stream chunk; current documentation says legacy usage opt-in fields are no
longer required. See [usage accounting](https://openrouter.ai/docs/cookbook/administration/usage-accounting).
Its credit system is denominated in USD; see
[OpenRouter billing support](https://openrouter.ai/support).

The adapter converts both values independently from USD to micro-USD. Parse
JSON decimal numbers, including scientific notation, without intermediate
binary floating-point arithmetic. Round to the nearest micro-USD, with exact
half values rounded upward. Negative, nonnumeric, and out-of-range amounts
are typed invalid-provider-data failures, never zero or saturated valid
charges. Missing/null account cost sets `account_charge_microusd` to `None`;
missing/null upstream cost independently sets its amount to `None`. Preserve
an upstream-only amount in `provider_cost: Some(...)`, and omit the envelope
only when neither amount is supplied. Validate every supplied amount even
when its counterpart is absent. Missing costs do not invalidate otherwise
complete token usage or establish an account charge.

Token usage remains independent: separate cached input from ordinary prompt
tokens and reasoning from visible completion tokens using saturating
subtraction. Cache-write tokens remain part of ordinary input quantity in the
existing DTO, and must not be added to total prompt tokens again. No fictitious
cache-write unit price is inferred. The provider-reported total is retained
when present; otherwise reconstruct it from non-overlapping token buckets.

The total measures the reported successful request. It does not claim to
include credit-purchase fees, account-wide discounts absent from the response,
separate upstream invoices, or failed/interrupted attempts whose charges are
unavailable. No `/generation` lookup or account reconciliation is introduced.

## Verification And Adoption

Tests must cover:

- old/new serde payloads, each combination of present/absent amounts, empty
  envelopes, authoritative zero, and unknown cost;
- upstream-only reports surviving serde, pricing, and runtime copies while
  effective cost falls back to a complete estimate or remains unknown;
- nonzero upstream cost with a zero account debit, without double counting;
- integer/fractional/exponent decimals, half-micro rounding, overflow,
  malformed amounts, and negative values;
- overlapping token counters and cache writes without double counting;
- complete, partial, free, and absent configured pricing;
- multiple individually valid cost lines whose sum reaches or exceeds
  `u64::MAX`, including zero and incomplete coverage, proving scalar/accessor
  saturation and unchanged unknown-cost selection in debug and release builds;
- repeated pricing wrappers around a provider report, for both buffered and
  event-observing calls, with identical effective cost and unchanged events;
- retention through the tool runtime's terminal checkpoint and logger copies.

New examples and OpenRouter live tests read `effective_cost()` and require
provider-reported cost for the live probe, including valid zero. Existing
providers continue using estimates. Firna's future dependency update must use
the accessor when metering/displaying totals; its storage or billing migration
is outside this workspace change and must not be represented as delivered.
