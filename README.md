# spiral-programs

On-chain half of the **Spiral** protocol: a decentralized audiovisual-preservation
architecture that combines IPFS content-addressed storage with Solana on-chain
registration. This repo is the Anchor/Solana workspace; the off-chain half lives
in [`spiralhq/spiral-backbone`](https://github.com/spiralhq/spiral-backbone).

## Programs

- **`organization`** — will own organizations, members, and roles. Currently an
  empty scaffold (`declare_id!` only, no accounts or instructions yet). Real
  logic is future work, not yet started.
- **`film`** — will own film/work registration. Also currently an empty
  scaffold, for the same reason.

Both programs are intentionally empty at this stage of the project — this
repository currently only bootstraps the workspace, toolchain, and CI. Do
not expect any account or instruction logic here yet.

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

### Gotcha: don't run bare `cargo test` before building

Each program's smoke test (`programs/*/tests/smoke.rs`) uses `include_bytes!`
to embed the compiled `.so` artifact from `target/deploy/`. That file only
exists after a build, so running `cargo test` directly, without an `anchor
build` first, fails to compile. Always use `anchor test` (which builds first),
or run `anchor build` before a bare `cargo test`.

## Program keypairs

The private keys backing the two Program IDs above are backed up outside of
`target/` in [`keys/`](keys/README.md) — see that file for why this matters
and what you must do with them before these programs are deployed to Devnet.
