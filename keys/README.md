# Program keypairs

This directory holds the private keypair files for the two Solana programs
declared in this workspace:

- `organization-keypair.json` -> `2JWb4r1LgykRNocsxVgfhysLtFtgjWvMRzceBDX79WCw`
  (matches `declare_id!` in `programs/organization/src/lib.rs` and
  `Anchor.toml` `[programs.localnet]` / `[programs.devnet]`)
- `film-keypair.json` -> `6MK8C7frTETvrmM5dhN6jkUyXdxcMAfTjKkhG1Y7Pcbu`
  (matches `declare_id!` in `programs/film/src/lib.rs` and
  `Anchor.toml` `[programs.localnet]` / `[programs.devnet]`)

## What these keys control, and what they don't

- **They fix the program addresses.** A program keypair signs only the first
  deploy, which creates the program at that address.
- **They are not the upgrade authority.** The upgrade authority is the
  **deployer wallet** that runs the deploy (`~/.config/solana/id.json`, the
  `[provider].wallet` in `Anchor.toml`). Every later upgrade, IDL upgrade or
  program close is signed by that wallet, not by these files.

Back up both:

- **The deployer wallet**, off-machine, before the first deploy. Without it the
  deployed programs can never be upgraded.
- **These program keypairs**, in secure storage outside this working copy and
  outside `target/` (a password manager or encrypted storage). Without them
  the programs cannot be deployed at these addresses on a new cluster, or
  again after a close.

## Why they live outside `target/`

`anchor deploy` signs the first deploy with `target/deploy/<p>-keypair.json`.
`target/` is gitignored and disappears with `cargo clean` or a fresh clone,
and when the file is missing `anchor build` / `anchor deploy` silently
generate a **new random keypair** there, with a different address. Before a
deploy, check:

```sh
solana-keygen pubkey target/deploy/organization-keypair.json
solana-keygen pubkey target/deploy/film-keypair.json
```

Each must equal the `declare_id!` above. If one differs, copy the real
keypair into place and repeat the check until both match:

```sh
cp keys/<p>-keypair.json target/deploy/<p>-keypair.json
```

Deploying with a mismatched keypair creates a second program at a random
address and rewrites `target/idl/<p>.json` with that address. On a fresh
clone this directory holds only this README: restore the keypairs from
backup first and check them with `solana-keygen pubkey keys/<p>-keypair.json`.

**Never run `anchor keys sync`.** It rewrites `declare_id!` and `Anchor.toml`
to match the keypair in `target/deploy/`, changing the program IDs that every
off-chain client uses. `anchor build` suggests it when it sees a mismatch;
restore the keypair instead.

See [`docs/devnet-deploy-runbook.md`](../docs/devnet-deploy-runbook.md) for
the full deploy procedure.

## These files are secrets

`keys/*.json` is gitignored on purpose. This README is the only committed
file here. Never remove `keys/*.json` from `.gitignore`, commit a keypair
file, or print its contents; `solana-keygen pubkey <file>` shows the public
address safely.
