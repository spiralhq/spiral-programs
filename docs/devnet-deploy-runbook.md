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
6. Run the bootstrap script: `cd scripts && node bootstrap-devnet.ts`, since
   Node's native TypeScript support handles this file without a separate
   transpile step. This requires Node 23.6+ (type stripping for `.ts` files
   is enabled by default from that version on; earlier Node 22.x versions can
   do it only behind `--experimental-strip-types`). Check with `node --version`
   first if you're not sure what's installed.
7. Record the deployed Program IDs and the IDL transaction signatures somewhere
   durable, so a future IDL-sync tool knows which build produced the IDL it copies.
