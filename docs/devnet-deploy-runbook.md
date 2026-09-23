# Devnet deploy runbook

Manual steps — not run by any automated task. Requires a funded Devnet wallet.

1. Confirm `anchor build` is clean and up to date: `anchor build`.
2. Confirm the wallet at `~/.config/solana/id.json` (or whatever `Anchor.toml`'s
   `[provider].wallet` points at) has enough Devnet SOL. Airdrop if needed:
   `solana airdrop 2 --url devnet`.
3. Deploy both programs: `anchor deploy --provider.cluster devnet`.
4. Publish the IDL for each program:
   `anchor idl init --provider.cluster devnet --filepath target/idl/organization.json <organization_program_id>`
   `anchor idl init --provider.cluster devnet --filepath target/idl/film.json <film_program_id>`
5. Generate a dedicated bootstrap payer keypair — do **not** reuse a program's
   upgrade-authority keypair for this (see warning below):
   ```sh
   solana-keygen new --outfile keys/bootstrap-payer-keypair.json
   solana airdrop 2 <pubkey printed above> --url devnet
   ```
   Note the path: `keys/bootstrap-payer-keypair.json` at the repo root, not
   `scripts/keys/`. `scripts/bootstrap-devnet.ts` resolves `KEYS_DIR` to the
   repo-root `keys/` directory and reads `bootstrap-payer-keypair.json` from
   there. That path is also the one covered by `.gitignore`'s `keys/*.json`
   pattern, so the new keypair won't accidentally get committed.

   **Warning:** never point the bootstrap payer at `keys/organization-keypair.json`
   (or `keys/film-keypair.json`). Those are the programs' own upgrade-authority
   keypairs (see [`keys/README.md`](../keys/README.md)) — a distinct credential
   with a distinct purpose. Once a program is deployed, that address is owned
   by the BPF upgradeable loader and cannot sign as a transaction fee payer at
   all; before deploy it's just an unfunded system account. Either way it
   cannot do this job — generate a separate keypair as above.
6. Run the bootstrap script: `cd scripts && npm run bootstrap-devnet` (runs
   `tsx bootstrap-devnet.ts`). This script imports its `@solana/kit` client
   from `scripts/generated/organization/` — Codama-generated and committed,
   like `idl/`. If `idl/organization.json` or `idl/film.json` changed since
   the last commit to `scripts/generated/`, re-run `npm run generate-clients`
   and commit the result before running the bootstrap script, or it'll be
   operating against a stale client. The generated client's own files use
   directory-style relative imports Node's native ESM loader doesn't resolve
   (`ERR_UNSUPPORTED_DIR_IMPORT`) — `tsx` (bundler-style resolution, same as
   this project's other Kit-based tooling) is required here; plain `node
   bootstrap-devnet.ts` will not work.
7. Record the deployed Program IDs and the IDL transaction signatures somewhere
   durable, so a future IDL-sync tool knows which build produced the IDL it copies.
