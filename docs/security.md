# Export safety and threat model

`cpctx` is a local context packer with a focused credential detector. It is not
a complete secret scanner. Review exported context before sharing it.

| Material | Expected behavior | Boundary |
| --- | --- | --- |
| PKCS#8, RSA, EC, DSA, OpenSSH and encrypted private-key headers | Reject, including headers inside quoted source literals | Does not decode encoded or split headers |
| Recognized GitHub, GitLab, Stripe, Slack and OpenAI token prefixes | Reject once the configured suffix length is reached | Prefix/length heuristics do not validate a token with its provider |
| AWS access-key IDs | Reject supported prefixes with the expected 16-character suffix | Does not detect every AWS credential format |
| Named password/API-key/token/client-secret assignments | Reject long literal values, including quoted JSON and typed Rust/Python assignments | Supported field names and simple literal syntax only |
| Public-key headers, short examples, environment references and typed assignments of identifiers | Accept regression fixtures | Other source syntax may still cause false positives |
| Unknown providers, encoded secrets, split strings, database URLs, short passwords | No guarantee | Must be reviewed or scanned independently |

Private-key and service-token checks apply to every selected UTF-8 source file.
Assignment checks recognize `password`, `passwd`, `api_key`, `access_token`,
`auth_token`, `client_secret`, `secret_key`, `aws_secret_access_key`,
`openai_api_key`, and `github_token`. They are heuristic and case-insensitive.
There is no flag to bypass credential checks. Ignored files are not scanned;
tree-only mode exports paths and does not scan source contents.

The detector returns a category, never the credential value. Collection and
rendered-budget validation complete before source output or clipboard calls.
Backend and stdout I/O failures can still happen after validation; these are
reported as errors rather than falsely claiming successful delivery.

## Regression and performance evidence

`src/snapshot/secrets.rs` contains a minimum-length boundary table for every
service prefix, embedded key-header cases, typed/JSON assignment cases, and safe
source/environment-reference fixtures. Integration tests also check zero stdout
and no clipboard invocation on rejected credentials. All fixtures are synthetic;
complete private-key headers and service tokens are assembled at test runtime.

On the managed Linux environment, 2026-10-22, a release CLI export of a 53,000-byte
safe Rust file took a median **3.204 ms** before and **3.040 ms** after the detector
changes (30 measured fresh-process runs per binary after five warm-up runs).
This includes process startup, traversal, scanning and rendering. The small
difference is noise, not a speedup claim; no ordinary-path regression was observed
in this smoke measurement. Recheck representative projects before broadening
patterns substantially. The baseline is commit `e7be27b`; this does not measure
network or clipboard latency.
