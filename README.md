**🔐 FlexFi Smart Contracts** — `flexfi-web3`

This repository contains the Solana smart contracts powering the FlexFi protocol. Written in Rust using the Anchor framework, these programs handle:

- BNPL plan creation

- Collateral staking

- Score-related logic

- Token flows & reward hooks

⚠️ Open sourced for hackathon transparency — but key logic is protected by NDA and contributor agreements.

**🚀 Local Dev Setup**

Prerequisites

- Rust (via rustup)

- Solana CLI

- Anchor CLI

Clone the repo
```
git clone https://github.com/flexfi-protocol/flexfi-web3.git
cd flexfi-web3
```
Install deps & build

`anchor build`

Run local tests

`anchor test`

**📁 Structure**
```
/programs/flexfi
 ┣ 📁 instructions   # Anchor handlers
 ┣ 📁 state          # Account structures
 ┣ 📁 utils          # Token helpers
 ┣ 📜 lib.rs         # Program entry
/tests                # Mocha/TS tests
```
**🛡 License (MIT)**

See `LICENSE` — Open source license for audit and transparency.
⚠️ Strategic logic (scoring, reward flow, custom triggers) may be hidden or simplified.

**🙌 Contributing**

See `CONTRIBUTING.md` before submitting PRs.
