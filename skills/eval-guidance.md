## Questions

- the spec17 dashboard says 'specs to compare 3' and 'later but built 2' - what does that mean for us and which ones are real problems?	
- quick SDD health check of this repo pls - what's tested, what's src-only, anything marked (later) that's already implemented
- find code under src/billing that isn't covered by any requirement, I want to know what behaviour we forgot to write a spec for
- i think ORD-12 and ORD-31 contradict each other on how discounts are rounded, both are cited in PriceCalculator.kt. can you confirm or rule it out
- before I merge feature/refund-limits: which requirement IDs does this branch touch, and do the specs still describe what the code now does?
- can you check which requirements in specs/ still don't have tests?
- we tag tests like [PAY-03] and cite (PAY-03) in the code
- generate unit tests for RefundService.calculateRefund, it has zero tests right now and handles partial refunds and fees
- convert our Jira epics export (epics.csv, columns key,summary,description) into a nicely structured markdown requirements doc
- my pytest suite has flaky tests in tests/integration, help me find which ones fail intermittently and why
- rewrite 'checkout should be fast and easy' into three measurable acceptance criteria for my requirements-writing workshop slides
- what's the difference between EARS requirement syntax and user stories, and which works better for a medical-device software team?
- explain what a requirements traceability matrix is in ISO 26262 and what auditors look for in one	I'm about to change the retry backoff in the sync client from linear to exponential. this repo is spec-driven, what do I update first and how?
- trace AUTH-07 for me (the session timeout one) - where is it implemented and which tests really cover it? dont trust the tags blindly
- specs/02-orders.md has ORD-01..ORD-40 and about half are check-marked ✅. are any of the ticked ones actually missing tests or not built?
- are there duplicate tests for INV-04? test_invoice_total and test_total_with_tax look like they check the same thing to me
- where's the function that parses the config file in this repo? somewhere under core/ i think, just need the location
- set up jest coverage thresholds at 80% lines for the web/ package and make CI fail when coverage drops below that
- write a PRD for a loyalty points feature: goals, non-goals, user stories, rollout plan. we're a small e-commerce startup	review PR #212 for bugs, especially the null handling in OrderMapper.toDto and the new pagination code

## Use when

1. Use when checking a repo's spec-driven development status with spec17: requirements without tests, untested or unspecified code, conflicting specs, duplicate tests, what a change touches, and before or after behaviour changes. Not for writing requirements or advice without a repo.

2. Use this skill whenever a user asks about the link between requirements, tests and code in their own repo. This covers repos whose specs (e.g. specs/*.md) define requirement IDs like PAY-03, with tests tagged [PAY-03] and code citing (PAY-03). It also covers any spec17 output or dashboard. Typical asks: an SDD or traceability health check; which requirements still have no tests; code with no requirement behind it; whether two requirement IDs contradict each other; which requirement IDs a branch or diff touches and whether the specs still match the code; items marked (later) that are already built; duplicate tests for one ID; keeping specs in sync before or after changing behaviour. Trigger even if spec17 is not named. Skip general test writing or flaky-test debugging that has no requirement IDs, converting tickets into specs, rewording acceptance criteria, and conceptual questions about requirements methods.	✗
