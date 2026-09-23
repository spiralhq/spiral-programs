# Bootstrap script implementation notes

`scripts/bootstrap-devnet.ts` uses `@solana/kit` and the Codama-generated client under
`scripts/generated/` (migrated from `@coral-xyz/anchor`/`@solana/web3.js`; run
`npm run generate-clients` after any `idl/` change). See the devnet deploy runbook for
running it.

- **WS URL override:** `solanaRpc()` derives the WebSocket subscriptions URL from `rpcUrl` by
  convention, which doesn't hold for every provider (e.g. Surfpool's independently, randomly
  assigned ports). `SPIRAL_DEVNET_WS` overrides it explicitly.
- **Deterministic H3 fixture addresses:** derived via `createKeyPairFromPrivateKeyBytes` from
  a 32-byte seed — verified byte-identical to the old `Keypair.fromSeed()` output for the same
  seed (Ed25519: the seed alone determines the public key), so already-bootstrapped devnet
  fixtures stay valid across the migration.
- **`revoke_membership` needs `eventAuthority`/`program` passed explicitly** — Anchor's
  `#[event_cpi]` self-CPI pattern isn't auto-defaulted by Codama's `...Async` instruction
  builders, unlike other PDA-derivable accounts.
- **The h3-revoked repair path:** a prior interrupted run may have granted membership without
  finishing the revoke. The script decodes the actual on-chain status and finishes the revoke
  if it's still `Active`, rather than trusting the account's mere existence.
