# OBCE usage example

This example is a port of a [rand-extension example](https://github.com/paritytech/ink/tree/master/integration-tests/rand-extension) from ink! repo.

As in the original example, this one provides you with:

* ink! smart contract, that calls the chain extension
* Substrate extension

## Example integration

### Substrate

In this section, we will initialize a `substrate-contracts-node` with our own chain extension,
which can later be used by the smart contracts that are deployed on this node.

1. Clone the [`substrate-contracts-node`](https://github.com/paritytech/substrate-contracts-node) repository.
2. Copy the `chain-extension` crate into your project and add it as a workspace member to `Cargo.toml` file:

```toml
# ...

members = [
    'chain-extension',
    'node',
    'runtime'
]

# ...
```

3. Add `rand-extension` to `runtime/Cargo.toml` file:

```toml
rand-extension = { path = "../chain-extension", default-features = false, features = ["substrate"] }
```

Also, add `rand-extension/substrate-std` to feature list that is activated when `std` feature is enabled:

```toml
[features]
# ...

std = [
    # ...
    "rand-extension/substrate-std",
]
```

4. Change `pallet_contracts::Config` to use the chain extension like so:

```rust
impl pallet_contracts::Config for Runtime {
    // ...

    type ChainExtension = (
        pallet_assets_chain_extension::substrate::AssetsExtension,

        // Your custom extension
        rand_extension::substrate::Extension,
    );

    // ...
}
```

5. Launch your node with the `cargo run` command.

### Ink

1. Make sure that you have [`cargo-contract`](https://github.com/paritytech/cargo-contract#installation) installed.
2. Create new ink! contract with `cargo contract new` command.
3. Replace lib.rs and Cargo.toml files with the example ones.
4. Modify path to your chain extension crate in the `Cargo.toml` file.
5. Build your contract with `cargo contract build` command.

You can utilize [Contracts UI](https://contracts-ui.substrate.io/) interface
to test your contract on the previously built node.
