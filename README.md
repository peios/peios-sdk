# Peios SDK

The Peios SDK provides C libraries for Peios security, registry access,
events, and registry storage backends. The repository contains three
components:

| Component | Role | Public library |
|---|---|---|
| `libpeios` | Application-facing Peios services | `libpeios.so.0` |
| `librsi` | Registry storage backends | `librsi.so.0` |
| `peios-cabi` | Internal allocation, errors, and system-call helpers | None |

The components share one release version. The public libraries retain
separate packages and ABI compatibility boundaries. The kernel remains a
separate project; `vendor/kernel/ORIGIN.json` records the exact source used
for the SDK's headers and shared semantic crates.

Read the [SDK guides](docs/learn/1--peios.shelf/peios.product/300--developing-for-peios.antho/100--start-here.shelf/100--sdk-basics.topic/100--what-is-the-peios-sdk.md)
for application development. See [building and testing](docs/building.md)
for repository work, [implementation ownership](docs/design.md), and
[ABI maintenance](docs/abi.md) for interface changes.

Historical design notes and superseded documentation are preserved in
[the documentation archive](docs/archive/documentation/README.md).
