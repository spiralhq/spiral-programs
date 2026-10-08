# spiral-programs

On-chain half of the **Spiral** protocol: a decentralized audiovisual-preservation
architecture that combines IPFS content-addressed storage with Solana on-chain
registration. This repo is the Anchor/Solana workspace; the off-chain half lives
in [`spiralhq/spiral-backbone`](https://github.com/spiralhq/spiral-backbone).

## Programs

Both programs are fully implemented: accounts, instructions, CPI-based
authorization, events, and error codes. See `programs/organization/src/`
and `programs/film/src/` for the source.

- **`organization`** — owns organizations, memberships, and role-based
  access control. State: `Organization`, `Membership` (with `Role` —
  Reader/Submitter/Admin — and `MembershipStatus` — Active/Revoked).
  Instructions: `initialize_organization`, `grant_membership`,
  `update_role`, `revoke_membership`, `transfer_authority`, and
  `assert_can_submit` (the authorization check other programs CPI into).
  Error codes `6000`-`6010`.
- **`film`** — owns audiovisual-work registration. `register_film` CPIs
  into `organization::assert_can_submit` to authorize the submitter before
  creating the `Film` account. Also provides `declare_replica`,
  `deactivate_replica`, and `update_access_policy`, which require an active
  membership in the film's organization (the film's submitter as Submitter
  or Admin, or any org Admin). Error codes `6005`-`6013`.

See [`docs/organization.md`](docs/organization.md) and
[`docs/film.md`](docs/film.md) for the full structure of each program:
every account, instruction, error code, and event.

## Build & test

Requires [`rustup`](https://rustup.rs/) (the toolchain is pinned via
`rust-toolchain.toml`) plus the Solana CLI and Anchor CLI (`anchor-cli 1.1.2`,
pinned in `Anchor.toml`'s `[toolchain]`).

```sh
anchor build
anchor test
```

`Anchor.toml` sets `skip_local_validator = true` and `[scripts].test = "cargo
test"`, so `anchor test` just builds and then runs `cargo test` — there is no
local validator involved. Both programs' tests use
[`litesvm`](https://github.com/LiteSVM/litesvm) for fast in-process program
testing instead.

The test suites live under each program's `tests/` directory:
`organization_instructions.rs`, `organization_negative.rs` and
`event_completeness.rs` for `organization`; `register_film.rs`,
`replicas.rs`, `management_authz.rs`, `event_completeness.rs`, and
`h3_scenarios.rs` (the H3 negative-scenario tests, with exact expected error
codes) for `film`.

H3 is one of the four research hypotheses the off-chain evaluation harness in
`spiral-backbone` tests (H1–H4, defined in that repo's `docs/eval-harness.md`):
**submission authority** — the programs refuse every unauthorized registration
on-chain, each with its expected error code.

## Devnet deployment

[`docs/devnet-deploy-runbook.md`](docs/devnet-deploy-runbook.md) is the full
procedure. The essentials:

- **Keys.** The deployer wallet (`~/.config/solana/id.json`) is the upgrade
  authority; back it up off-machine. The program keypairs in
  [`keys/`](keys/README.md) only fix the program addresses; back them up too.
  Before deploying, `solana-keygen pubkey target/deploy/<p>-keypair.json` must
  equal `declare_id!`; if not, restore `keys/` from backup, copy
  `keys/<p>-keypair.json` into `target/deploy/` and check again. **Never run
  `anchor keys sync`**: it rewrites the program IDs every off-chain client
  uses.
- **Funding.** About 3.2 SOL stays locked as rent. The first deploy needs
  about 3.2 SOL and an upgrade peaks around 4.8 SOL, before IDL accounts and
  fees; fund the deployer with at least 6 SOL.
- **IDL.** `anchor deploy` uploads the IDL to the Program Metadata program by
  default, and re-uploads it on every redeploy. Do not run `anchor idl init`
  after it. To upload separately, deploy with `--no-idl` and then run
  `anchor idl init`. After `anchor upgrade` or a `solana program` upgrade,
  run `anchor idl upgrade`.
- **Build identity.** Build from a clean checkout of the tagged commit, record
  the commit and `sha256sum target/deploy/*.so`, deploy exactly that build and
  keep that `target/` until teardown. The off-chain pre-flight compares the
  deployed program byte for byte with it, so a rebuild after the deploy makes
  that check fail until you redeploy.
- **Duplicate CIDs** are refused per organization, not globally: the `Film`
  address is derived from the organization and the CID. This is by design.
- `scripts/bootstrap-devnet.ts` is an optional demo; the research campaign
  does not need it.
