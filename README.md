# Foundry EVM Mini

A copy of https://github.com/foundry-rs/foundry

Foundry introduces new changes to their repo everyday, and this can cause projects using foundry-evm break every once in a while.

This foundry-evm-mini project is an attempt to take a snapshot of the library so that it never breaks again.

Currently supports:

- SharedBackend (evm/executor/fork)
- BlockchainDB (evm/executor/fork)
- BlockchainDBMeta (evm/executor/fork)

---

* How to use?

To use this repo, you can configure your Cargo.toml as below:

```
[dependencies]
ethers = { version = "2.0", features = ["ethers-solc"] }

# EVM
bytes = "1"
hashbrown = { version = "0.13", features = ["serde"] }
revm = { version = "3", default-features = false, features = [
  "std",
  "serde",
  "memory_limit",
  "optional_eip3607",
  "optional_block_gas_limit",
  "optional_no_base_fee",
] }

foundry-evm-mini = { git = "https://github.com/solidquant/foundry-evm-mini.git" }

[patch.crates-io]
revm = { git = "https://github.com/bluealloy/revm/", rev = "80c909d6f242886cb26e6103a01d1a4bf9468426" }
```

Make sure **revm** version is patched to the commit at: 80c909d6f242886cb26e6103a01d1a4bf9468426 (v3.4)