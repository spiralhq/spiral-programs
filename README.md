# 🌀 Spiral Programs

<div align="center">

[![Solana](https://img.shields.io/badge/Solana-Devnet-blueviolet?style=for-the-badge&logo=solana&logoColor=white)](https://solana.com)
[![Anchor](https://img.shields.io/badge/Anchor-1.1.2-informational?style=for-the-badge)](https://book.anchor-lang.com/)
[![License](https://img.shields.io/github/license/spiralhq/spiral-programs?style=for-the-badge)](LICENSE)

**On-chain half of the Spiral protocol**

_Authorization and registration of audiovisual works, anchored to Solana accounts._

</div>

---

## ✨ Overview

**Spiral** is a protocol for preserving audiovisual works by combining
content-addressed storage (IPFS) with on-chain registration (Solana). A
work's identity, access policy, and declared replica set live in an account
on the network; no index or application service is trusted as the source of
truth.

This repository is the **Anchor/Solana workspace** — the two programs that
define Spiral's on-chain logic. The off-chain half (indexing, Kubo nodes,
CLI, and the evaluation harness) lives in
[`spiralhq/spiral-backbone`](https://github.com/spiralhq/spiral-backbone).

---

## 🧱 Protocol Architecture

```
Organization ──▶ assert_can_submit (CPI) ──▶ Film
     (members, roles)                  (registration, replicas, access policy)
```

`film` never reads a membership account on its own: it asks `organization`,
across programs, before creating a `Film` account or changing one. Each
program is independently deployable and enforces its own authorization —
there is no shared, implicit trust between them.

---

## 📦 Programs

### 🏛️ Organization

**Purpose**
Owns organizations, memberships, and role-based access control for the rest
of the protocol.

**Highlights**
- Deterministic organization & membership PDAs
- Roles (`Reader`, `Submitter`, `Admin`) and status (`Active`, `Revoked`)
- `assert_can_submit` — the authorization check other programs CPI into
- `initialize_organization`, `grant_membership`, `update_role`,
  `revoke_membership`, `transfer_authority`
- Error codes `6000`-`6010`

### 🎞️ Film

**Purpose**
Registers audiovisual works and their declared storage replicas.

**Highlights**
- `register_film` CPIs into `organization::assert_can_submit` before
  creating the `Film` account
- Up to eight declared replicas per work, identified by IPFS `PeerID`
- `declare_replica`, `deactivate_replica`, `update_access_policy` — gated to
  an active Submitter/Admin membership in the film's organization
- Duplicate CIDs refused **per organization**, not globally — the `Film`
  address is derived from the organization and the CID, by design
- Error codes `6005`-`6013`

Error ranges overlap between the two programs on purpose: a code is only
meaningful together with the program that returned it.

See [`docs/organization.md`](docs/organization.md) and
[`docs/film.md`](docs/film.md) for the full structure of each program —
every account, instruction, error code, and event.

---

## 🔐 Security Model

- PDA-derived authorities everywhere; no keypair owns protocol state
- Explicit CPI authorization — `film` cannot create or alter a work without
  `organization`'s sign-off
- Role enforcement at the instruction level, checked before any account is
  touched
- Deterministic account derivation closes the door on duplicate submissions

---

## 🧪 Testing

Both programs are covered by Anchor-based integration tests using
[`litesvm`](https://github.com/LiteSVM/litesvm) for fast, in-process program
testing — there is no local validator involved.

```
programs/
├── organization/tests/
│   ├── organization_instructions.rs
│   ├── organization_negative.rs
│   ├── event_completeness.rs
│   └── error_docs.rs
└── film/tests/
    ├── register_film.rs
    ├── replicas.rs
    ├── management_authz.rs
    ├── event_completeness.rs
    ├── h3_scenarios.rs
    └── error_docs.rs
```

`h3_scenarios.rs` exercises one of the four research hypotheses tested by
the off-chain evaluation harness in `spiral-backbone` (H1–H4, defined in
that repo's `docs/eval-harness.md`): **submission authority** — every
unauthorized registration is refused on-chain, each with its exact expected
error code.

---

## 🛠️ Development

### Requirements

| Tool        | Version                              |
|-------------|---------------------------------------|
| Rust        | 1.89.0 (pinned via `rust-toolchain.toml`) |
| Solana CLI  | 3.1.10                                |
| Anchor CLI  | 1.1.2 (pinned in `Anchor.toml`)       |

### Build

```bash
anchor build
```

### Test

```bash
anchor test
```

`Anchor.toml` sets `skip_local_validator = true` and `[scripts].test = "cargo
test"`, so `anchor test` just builds and then runs `cargo test`.

---

## 🚀 Devnet Deployment

[`docs/devnet-deploy-runbook.md`](docs/devnet-deploy-runbook.md) is the full
procedure. The essentials:

- **Keys.** The deployer wallet is the upgrade authority; back it up
  off-machine. The program keypairs in [`keys/`](keys/README.md) only fix
  the program addresses — back them up too, and never run
  `anchor keys sync` (it rewrites the program IDs every off-chain client
  uses).
- **Funding.** First deploy needs about 3.2 SOL in rent; an upgrade peaks
  around 4.8 SOL before IDL accounts and fees. Fund the deployer with at
  least 6 SOL.
- **IDL.** `anchor deploy` uploads the IDL by default and re-uploads it on
  every redeploy; do not run `anchor idl init` afterward. After
  `anchor upgrade` or a `solana program` upgrade, run `anchor idl upgrade`.
- **Build identity.** Build from a clean checkout of the tagged commit,
  record `sha256sum target/deploy/*.so`, and deploy exactly that build. The
  off-chain pre-flight compares the deployed program byte for byte against
  it.

---

## 📜 License

MIT License.

---

<div align="center">

**Spiral is building public infrastructure for the preservation of human audiovisual memory.**

Made with 🌀 by **spiralhq**

</div>
