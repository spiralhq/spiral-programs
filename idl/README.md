# IDL snapshots

This directory holds committed, point-in-time copies of the Anchor-generated
IDL JSON for both on-chain programs:

- `organization.json`
- `film.json`

They're copied here from `target/idl/*.json` (which is gitignored, since
`target/` is a build output directory that gets wiped by `cargo clean`, a
fresh clone, or CI). This `idl/` directory exists so consumers have
something versioned to read without needing a local Anchor build.

## These are snapshots, not a live sync

The files here reflect the state of `programs/organization` and
`programs/film` as of the commit that last updated them. They are **not**
automatically regenerated or kept in sync with the program source — if the
programs change, these files go stale until someone manually re-runs
`anchor build` and copies the new output here. A future sync mechanism
(e.g. a script that pulls the IDL from chain via `anchor idl fetch`, or
from a fresher build) may replace this snapshot approach. Don't build
anything that assumes these files auto-update.

## Client derivation contract: the `Film` PDA

Anchor's IDL format can express the `organization` program's PDA seeds
(they're static byte literals plus account keys), but it **cannot** express
the `film` program's PDA, because one of its seeds is a hash computed over a
*prefix* of a fixed-size array, not the whole field. A client that only
reads the IDL has no way to derive a `Film` account's address — this is
that missing piece, written down explicitly:

```
Film PDA = findProgramAddress(
  seeds: [
    "film",
    organization_pubkey (32 bytes),
    sha256(cid_bytes[..cid_len])[..32],
  ],
  programId: film program ID,
)
```

Where:
- `cid_bytes` is the on-chain `Film.cid` field: a fixed **64-byte** array.
- `cid_len` is the on-chain `Film.cid_len` field: the actual meaningful
  length of the CID within that 64-byte array — **not** 64. For the
  current Spiral Chunking Profile v1, `cid_len` is always `36`.
- The hash input is `cid_bytes[0..cid_len]` (the first `cid_len` bytes
  only) — hashing all 64 bytes (including the unused zero-padded tail)
  produces a different digest and the wrong address.
- The seed itself is the **first 32 bytes** of the sha256 digest output
  (sha256 already produces exactly 32 bytes, so this is the whole digest).

This matches `programs/film/src/instructions/register_film.rs`'s `seeds =
[...]` constraint exactly; if the program changes, that file is the source
of truth and this doc should be updated to match.

### CID byte layout (Spiral Chunking Profile v1)

For the current profile, a valid CID's first 4 bytes (of the 36 meaningful
bytes) follow this fixed layout, enforced by `register_film`'s validation:

| Byte(s) | Value             | Meaning                              |
|---------|-------------------|---------------------------------------|
| 0       | `0x01`            | CIDv1                                  |
| 1       | `0x70` or `0x55`  | codec: dag-pb or raw                   |
| 2       | `0x12`            | multihash code: sha2-256               |
| 3       | `0x20`            | multihash digest length: 32 bytes      |
| 4-35    | (32 bytes)        | the sha2-256 digest itself             |

Total meaningful length: 4 + 32 = 36 bytes, matching `cid_len == 36`.
