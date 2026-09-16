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
  Error codes `6000`-`6004`.
- **`film`** — owns audiovisual-work registration. `register_film` CPIs
  into `organization::assert_can_submit` to authorize the submitter before
  creating the `Film` account. Also provides `declare_replica`,
  `deactivate_replica`, and `update_access_policy`. Error codes
  `6005`-`6008`.

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
`organization_instructions.rs` and `event_completeness.rs` for
`organization`; `register_film.rs`, `event_completeness.rs`, and
`h3_scenarios.rs` (the H3 negative-scenario tests, with exact expected error
codes) for `film`.

## Program keypairs

The private keys backing the two Program IDs above are backed up outside of
`target/` in [`keys/`](keys/README.md) — see that file for why this matters
and what you must do with them before these programs are deployed to Devnet.
