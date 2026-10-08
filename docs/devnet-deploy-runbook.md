# Devnet deploy runbook

Manual procedure. Nothing in this repository runs it automatically. The
commands assume the pinned toolchain: `solana-cli 3.1.10` and `anchor-cli
1.1.2` (`Anchor.toml` `[toolchain]`), Rust 1.89.0 (`rust-toolchain.toml`).
Check with `solana --version` and `anchor --version` before you start.

| Program        | Program ID (`declare_id!`, `Anchor.toml` `[programs.devnet]`) |
|----------------|----------------------------------------------------------------|
| `organization` | `2JWb4r1LgykRNocsxVgfhysLtFtgjWvMRzceBDX79WCw`                 |
| `film`         | `6MK8C7frTETvrmM5dhN6jkUyXdxcMAfTjKkhG1Y7Pcbu`                 |

## Keys: who controls what

| Key | File | Controls |
|-----|------|----------|
| Deployer wallet | `~/.config/solana/id.json` (`Anchor.toml` `[provider].wallet`) | Pays for the deploy and becomes the **upgrade authority** of both programs, and the authority of their IDL metadata. |
| Program keypairs | `keys/organization-keypair.json`, `keys/film-keypair.json` | Fix the program **addresses**. They sign only the first deploy. They are not the upgrade authority. |

- Back up the deployer wallet off-machine before the first deploy. Losing it
  means the deployed programs and their IDL can never be upgraded or closed.
- Back up the program keypairs too (see [`keys/README.md`](../keys/README.md)).
  Losing one before the first deploy means that address can never be used.
- `anchor deploy` uses `[provider].wallet`; `solana program …` uses the
  keypair in the Solana CLI config (`solana config get`). Make sure both point
  at the same deployer wallet, or pass it explicitly
  (`--provider.wallet <WALLET>` for `anchor`, `--upgrade-authority
  <UPGRADE_AUTHORITY_SIGNER>` / `--keypair <KEYPAIR>` for `solana`).

**Never run `anchor keys sync`.** It rewrites `declare_id!` and `Anchor.toml`
to whatever keypair sits in `target/deploy/`, which changes the program IDs
that every off-chain client, the IDL and the generated clients rely on.
`anchor build` prints a "Program ID mismatch" warning that suggests it; fix
the keypair instead (step 2).

## 1. Build a recorded build

The deployed ELF must be exactly the local build: an off-chain pre-flight
compares the deployed program byte for byte with `target/deploy/<p>.so`, and
the local `target/idl/<p>.json` with the client IDL in `idl/`.

1. Check out the tagged commit you are deploying, in a clean tree:
   ```sh
   git checkout <tag>
   git status            # must report a clean working tree
   ```
2. Build and confirm the IDL matches the committed snapshot:
   ```sh
   anchor build
   bash scripts/check-idl.sh        # must print "IDL up to date"
   ```
3. Record the build identity somewhere durable outside `target/`:
   ```sh
   git rev-parse HEAD
   sha256sum target/deploy/*.so
   solana --version && anchor --version
   ```

Deploy exactly this build, and keep this `target/` directory untouched until
teardown. Any rebuild after the deploy (even of the same commit, after
`cargo clean` or a toolchain change) can produce a different `.so`, and the
pre-flight then fails until you redeploy that new build.

## 2. Check the program keypairs

`anchor deploy` signs the first deploy with `target/deploy/<p>-keypair.json`.
If that file is missing, `anchor build` / `anchor deploy` silently generate a
**new random keypair** there, with a different address. A deploy with that
keypair creates a second copy of the program at a random address (about
1.6 SOL wasted) and rewrites `target/idl/<p>.json` with the wrong address.

1. `keys/*.json` is gitignored, so a fresh clone has no keypairs. Restore
   `keys/organization-keypair.json` and `keys/film-keypair.json` from your
   backup first, and check them:
   ```sh
   solana-keygen pubkey keys/organization-keypair.json   # expect 2JWb4r1LgykRNocsxVgfhysLtFtgjWvMRzceBDX79WCw
   solana-keygen pubkey keys/film-keypair.json           # expect 6MK8C7frTETvrmM5dhN6jkUyXdxcMAfTjKkhG1Y7Pcbu
   ```
2. Check the keypairs Anchor will use:
   ```sh
   solana-keygen pubkey target/deploy/organization-keypair.json
   solana-keygen pubkey target/deploy/film-keypair.json
   ```
3. If one differs from `declare_id!` in `programs/<p>/src/lib.rs`, copy the
   real keypair into place and repeat check 2 until both match:
   ```sh
   cp keys/<p>-keypair.json target/deploy/<p>-keypair.json
   ```

Do not deploy until both `target/deploy` keypairs match `declare_id!`. Never
"fix" a mismatch by editing the IDs.

## 3. Fund the deployer

```sh
solana address                               # deployer public key (Solana CLI config)
solana-keygen pubkey ~/.config/solana/id.json  # same key, as Anchor sees it
solana balance --url devnet
solana airdrop 2 --url devnet                # repeat, or use a Devnet faucet
```

Rent-exempt deposits are `(128 + bytes) × 6960` lamports. For the current
binaries (`film.so` 225,752 bytes, `organization.so` 230,232 bytes):

- **About 3.2 SOL stays locked** as rent in the two program-data accounts
  (≈1.57 SOL for `film`, ≈1.60 SOL for `organization`).
- **First deploy: about 3.2 SOL.** Each program is first written to a buffer
  that holds its full rent (≈1.57-1.60 SOL); the deploy instruction moves
  those lamports into the program-data account. So the balance needed is the
  rent of the programs deployed so far plus one buffer, which ends at ≈3.2 SOL.
- **Upgrade: about 4.8 SOL.** An upgrade writes a full new buffer (up to
  ≈1.60 SOL) while both program-data accounts still exist; the buffer is
  refunded when the upgrade completes.
- **On top of that:** the IDL metadata accounts, transaction fees, and any
  buffer left behind by a failed upload (≈1.6 SOL each until closed).

Fund the deployer with **at least 6 SOL**: 5 SOL leaves only about 0.2 SOL
of headroom at the 4.8 SOL upgrade peak, before IDL accounts and fees.
Recompute the figures if the binaries grow.

## 4. Deploy

```sh
anchor deploy --provider.cluster devnet
```

**Use a dedicated RPC endpoint if you have one.** A deploy writes each
binary in hundreds of transactions, and the public Devnet endpoint
rate-limits them, which leaves failed buffers behind. In every command in
this runbook, replace `--provider.cluster devnet` and `--url devnet` with the
provider URL:

```sh
anchor deploy --provider.cluster <dedicated-url>   # same for idl init/upgrade and upgrade
solana program show <PROGRAM_ID> --url <dedicated-url>
solana config set --url <dedicated-url>            # optional: default for solana commands
```

Fall back to the public endpoint (`devnet`) only if no dedicated one is
available, and expect to resume a failed deploy as described below.

This deploys both programs with the deployer wallet as upgrade authority
(the output prints `Upgrade authority: <wallet>`). In Anchor 1.1.2 it then
**uploads each IDL to the Program Metadata program by default** (`--no-idl`:
"Don't upload IDL during deployment (IDL is uploaded by default)"). That step
runs `npx @solana-program/program-metadata`, so Node and network access are
needed. It also rewrites `target/idl/<p>.json` with the deployed address,
which leaves the file unchanged when step 2 passed.

- **Do not run `anchor idl init` after a normal deploy**: the IDL is already
  on-chain (`idl init` "Can only be run once"). After a later redeploy, use
  `anchor idl upgrade` (see Upgrades).
- To upload the IDL separately, deploy with `--no-idl` and then:
  ```sh
  anchor idl init --provider.cluster devnet --filepath target/idl/organization.json 2JWb4r1LgykRNocsxVgfhysLtFtgjWvMRzceBDX79WCw
  anchor idl init --provider.cluster devnet --filepath target/idl/film.json 6MK8C7frTETvrmM5dhN6jkUyXdxcMAfTjKkhG1Y7Pcbu
  ```
- If a deploy fails part way, Anchor keeps the buffer keypair in
  `target/deploy/<p>-upgrade-buffer.json`; re-running the same command
  resumes it. To give up instead, close the buffer (see Recovering a failed
  buffer).
- Always deploy with the `target/deploy` keypairs checked in step 2. Any
  later `anchor deploy`, including an upgrade, uses them too.

## 5. Verify and record

```sh
solana program show <PROGRAM_ID> --url devnet
```

`Authority` must be the deployer public key, and `Data Length` the size of
the local `.so`. Compare the deployed ELF and the IDL with the local build:

```sh
solana program dump <PROGRAM_ID> deployed-<p>.so --url devnet
sha256sum deployed-<p>.so target/deploy/<p>.so

anchor idl fetch <PROGRAM_ID> --provider.cluster devnet --out onchain-<p>.json
node -e 'const fs=require("fs"),u=require("util");const [a,b]=process.argv.slice(1).map(f=>JSON.parse(fs.readFileSync(f,"utf8")));process.exit(u.isDeepStrictEqual(a,b)?0:1)' target/idl/<p>.json onchain-<p>.json && echo "IDL matches"
```

The dump has the length of the program-data account. It matches the `.so`
byte for byte after a fresh deploy; after an upgrade to a smaller binary or an
extend, it carries trailing zero bytes.

Record, next to the build identity from step 1: the deployer public key, the
deploy and IDL transaction signatures, and the date.

## Upgrades

Start again from step 1 with the new tagged commit, and keep the new
`target/` from then on. The deployer wallet must sign every upgrade. The dedicated RPC endpoint
advice in step 4 applies here too.

- **`anchor deploy`** again (optionally `--program-name <p>`): it detects the
  existing program, upgrades it through a buffer, extends it if needed, and
  uploads the IDL again. It picks the program by
  `target/deploy/<p>-keypair.json`, so **repeat the step 2 check first**: with
  a regenerated keypair it deploys a new program at a random address instead
  of upgrading.
- **`anchor upgrade`**: upgrades one program but does **not** upload the IDL
  in Anchor 1.1.2, so follow it with `anchor idl upgrade`:
  ```sh
  anchor upgrade --provider.cluster devnet --program-id <PROGRAM_ID> target/deploy/<p>.so
  anchor idl upgrade --provider.cluster devnet --filepath target/idl/<p>.json <PROGRAM_ID>
  ```
- **Solana CLI**, directly or through an explicit buffer, then
  `anchor idl upgrade` as above:
  ```sh
  solana program deploy target/deploy/<p>.so --program-id <PROGRAM_ID> --url devnet
  # or
  solana program write-buffer target/deploy/<p>.so --url devnet   # prints <BUFFER>
  solana program upgrade <BUFFER> <PROGRAM_ID> --url devnet
  ```

Only `anchor deploy` re-uploads the IDL. After `anchor upgrade` or either
Solana CLI path, run `anchor idl upgrade`.

**Size growth.** `anchor deploy`, `anchor upgrade` and `solana program
deploy` (unless `--no-auto-extend`) extend the program-data account
automatically when the new binary is larger. The `write-buffer` +
`solana program upgrade` path does **not**: extend first, by the new `.so`
length minus the current `Data Length` shown by `solana program show
<PROGRAM_ID>`:

```sh
solana program show <PROGRAM_ID> --url devnet      # note Data Length
stat -c %s target/deploy/<p>.so                     # new length
solana program extend <PROGRAM_ID> <ADDITIONAL_BYTES> --url devnet
```

**Recovering a failed buffer.** A failed upload leaves a buffer holding the
rent for the whole binary.

```sh
solana program show --buffers --url devnet          # buffers owned by the deployer
solana program close <BUFFER> --url devnet          # returns its lamports
```

To resume instead of closing, you need the buffer's keypair:

- **Anchor** (`anchor deploy`, `anchor upgrade`): it is
  `target/deploy/<p>-upgrade-buffer.json`; re-run the same command.
- **Solana CLI** (`solana program deploy`, `write-buffer`): a failed upload
  prints a seed phrase for the buffer. Recover the keypair and pass it back:
  ```sh
  solana-keygen recover --outfile buffer-keypair.json      # enter the printed seed phrase
  solana program deploy target/deploy/<p>.so --program-id <PROGRAM_ID> --buffer buffer-keypair.json --url devnet
  ```

## Teardown

Keep the recorded `target/` until the programs are no longer used. Closing a
program (`solana program close <PROGRAM_ID> --bypass-warning --url devnet`)
returns its rent but makes the address unusable forever, so never close a
program whose ID clients still use.

## Things to know about the deployed programs

- **Duplicate CIDs are refused per organization, not globally.** The `Film`
  address is derived from `["film", organization, sha256(cid)]`, so the same
  CID registered twice in one organization fails, while another organization
  can register it. This is by design.
- `docs/organization.md` and `docs/film.md` describe every instruction,
  error code and event.

## Optional: bootstrap demo fixtures

`scripts/bootstrap-devnet.ts` creates a demo organization (`spiralhq-devnet`)
and three membership fixtures (`h3-no-membership`, `h3-revoked`,
`h3-reader-role`). **The research campaign does not need it**: the off-chain
H3 harness builds its own fixtures. It is kept as an optional demo.

The fixture member keys are derived from their **public names** (see
[`bootstrap-script-notes.md`](bootstrap-script-notes.md)), so anyone can
recompute them and sign as those members. Never fund them or give them a role
in an organization that matters.

To run it:

1. Generate a dedicated payer (it becomes the demo organization's authority):
   ```sh
   solana-keygen new --outfile keys/bootstrap-payer-keypair.json
   solana airdrop 1 <payer pubkey> --url devnet
   ```
   The path is the repo-root `keys/`, which `.gitignore` covers
   (`keys/*.json`). Do not use a program keypair as payer: it is not a
   funded wallet and must never leave `keys/`.
2. If `idl/` changed since `scripts/generated/` was last regenerated, run
   `npm run --prefix scripts generate-clients` first.
3. `cd scripts && npm run bootstrap-devnet` (runs `tsx bootstrap-devnet.ts`;
   plain `node` cannot resolve the generated client's directory imports).
   `SPIRAL_DEVNET_RPC` and `SPIRAL_DEVNET_WS` override the endpoints.
