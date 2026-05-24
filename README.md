# Anchor AMM Program

Constant-product AMM built with Anchor on Solana. Tests run fully in-process using LiteSVM.

---

## Project Structure

```
programs/amm-program/
├── src/
│   ├── lib.rs
│   ├── state.rs
│   ├── errors.rs
│   └── instructions/
│       ├── mod.rs
│       ├── initialize.rs
│       ├── deposit.rs      ← accounts are Box<> to avoid BPF stack overflow
│       ├── withdraw.rs     ← accounts are Box<> to avoid BPF stack overflow
│       └── swap.rs
tests/
├── amm-program.rs          ← 4 test cases
└── ix_handlers.rs          ← instruction builders
```

---

## Setup

**`Cargo.toml` dev-dependencies:**

```toml
[dev-dependencies]
litesvm        = "0.12.0"
litesvm-token  = "0.12.0"
sha2           = "0.11.0"
solana-keypair     = "3.1.0"
solana-signer      = "3.0.0"
solana-pubkey      = "4.1.0"
solana-hash        = "4.1.0"
solana-message     = "3.1.0"
solana-transaction = "3.0.2"
```

> Do **not** add `solana-sdk` directly — litesvm brings its own consistent version. Adding it separately causes duplicate-type conflicts.

---

## Build & Run

```bash
# 1. Build the program (.so is required by the tests)
anchor build

# 2. Run tests — no validator, no airdrop, runs in milliseconds
cargo test --test amm-program -- --nocapture

# 3. Run a single test
cargo test --test amm-program test_swap -- --nocapture
```

---

## Tests

| Test | What it checks |
|---|---|
| `test_initialize` | Config PDA, LP mint, and both vaults exist after init |
| `test_deposit` | Vaults receive tokens, payer receives LP tokens |
| `test_withdraw` | Vaults empty out, payer's tokens restored |
| `test_swap` | Payer receives Y after selling X, vault_x grows, k = x·y doesn't decrease |

---

## Screenshot

<img width="2156" height="1204" alt="image" src="https://github.com/user-attachments/assets/bea90fea-658a-42d8-bc16-3f6ad152f9c4" />
