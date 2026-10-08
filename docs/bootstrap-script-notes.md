# Bootstrap script implementation notes

`scripts/bootstrap-devnet.ts` is an **optional demo**. It creates a demo
organization (`spiralhq-devnet`) and three membership fixtures on Devnet. The
research campaign does **not** need it: the off-chain H3 harness builds its
own fixtures. See the "Optional: bootstrap demo fixtures" section of
[`devnet-deploy-runbook.md`](devnet-deploy-runbook.md) for running it.

It uses `@solana/kit` and the Codama-generated client under
`scripts/generated/` (run `npm run generate-clients` after any `idl/` change).

## WS URL override

`solanaRpc()` derives the WebSocket subscriptions URL from `rpcUrl` by
convention, which doesn't hold for every provider (e.g. Surfpool's
independently, randomly assigned ports). `SPIRAL_DEVNET_WS` overrides it
explicitly.

## Deterministic H3 fixture addresses

Each fixture member key is derived via `createKeyPairFromPrivateKeyBytes`
from a 32-byte seed computed from the fixture's **public name** (e.g.
`h3-revoked`). Re-running the script therefore yields the same addresses and
stays idempotent, and the derivation is byte-identical to the earlier
`Keypair.fromSeed()` output for the same seed (Ed25519: the seed alone
determines the public key), so already-bootstrapped fixtures stay valid.

Because the seeds are public, **anyone can recompute these private keys and
sign as the fixture members.** They are demo identities only: never fund them
or grant them a role in an organization that matters.

## `revoke_membership` needs `eventAuthority`/`program` passed explicitly

Anchor's `#[event_cpi]` self-CPI pattern isn't auto-defaulted by Codama's
`...Async` instruction builders, unlike other PDA-derivable accounts.

## The h3-revoked repair path

A prior interrupted run may have granted membership without finishing the
revoke. The script decodes the actual on-chain status and finishes the revoke
if it's still `Active`, rather than trusting the account's mere existence.
