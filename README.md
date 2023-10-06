# Elliptic V0

Find the documentation regarding the design of the protocol [here](https://elliptic-1.gitbook.io/elliptic-docs/). The design has been inspired by the [liquity protocol](https://www.liquity.org/) on Ethereum.

## Build Elliptic
The code is depending on the [Internet Computer](https://github.com/dfinity/ic) repo. If you want to compile it yourself the code please add the ic repo next to this repo.

```shell
git clone git@github.com:dfinity/ic.git
cd ic
git checkout 423f17976070f91353a817ee234b68284094b6e0
```

```shell
dfx build protocol
```

By compiling the code you can verify the Module Hash
```shell
sha256sum ./.dfx/local/canisters/protocol/protocol.wasm
```

Current deployed Module Hash: **a7ebfd7096ee5ef70072643773f7f7f6f0a44257ac941b6762f3440264c5f6d4**

Note: To execute the following commands, locate your terminal in the current folder.

## Vault

Note: The amounts described below have 8 decimals.

Example: 1 ckBTC would be 100_000_000

### Open a vault
```
 ┌────┐     ┌────────┐                        ┌────────────┐
 │User│     │Elliptic│                        │ckBTC Ledger│
 └─┬──┘     └───┬────┘                        └─────┬──────┘
   │            │                                   │       
   │open_vault()│                                   │       
   │───────────>│                                   │       
   │            │                                   │       
   │            │icrc2_transfer_from(user, elliptic)│       
   │            │──────────────────────────────────>│       
   │            │                                   │       
   │            │            block_index            │       
   │            │<──────────────────────────────────│       
   │            │                                   │       
   │  vault_id  │                                   │       
   │<───────────│                                   │       
 ┌─┴──┐     ┌───┴────┐                        ┌─────┴──────┐
 │User│     │Elliptic│                        │ckBTC Ledger│
 └────┘     └────────┘                        └────────────┘
```

```shell
dfx canister --network ic call protocol open_vault '(CKBTC_AMOUNT)'
```

### Borrow from a vault
```
 ┌────┐                            ┌────────┐                  ┌──────────┐
 │User│                            │Elliptic│                  │TAL Ledger│
 └─┬──┘                            └───┬────┘                  └────┬─────┘
   │                                   │                            │      
   │borrow_from_vault(vault_id, amount)│                            │      
   │──────────────────────────────────>│                            │      
   │                                   │                            │      
   │                                   │icrc1_transfer(user, amount)│      
   │                                   │───────────────────────────>│      
   │                                   │                            │      
   │                                   │        block_index         │      
   │                                   │<───────────────────────────│      
   │                                   │                            │      
   │            block_index            │                            │      
   │<──────────────────────────────────│                            │      
 ┌─┴──┐                            ┌───┴────┐                  ┌────┴─────┐
 │User│                            │Elliptic│                  │TAL Ledger│
 └────┘                            └────────┘                  └──────────┘

```
```shell
dfx canister --network ic call protocol borrow_from_vault '(record{vault_id=VAULT_ID; amount=BORROWED_AMOUNT})'
```

### Repay to a vault
```
 ┌────┐                         ┌────────┐                       ┌──────────┐
 │User│                         │Elliptic│                       │TAL Ledger│
 └─┬──┘                         └───┬────┘                       └────┬─────┘
   │                                │                                 │      
   │repay_to_vault(vault_id, amount)│                                 │      
   │───────────────────────────────>│                                 │      
   │                                │                                 │      
   │                                │icrc2_transfer_from(user, amount)│      
   │                                │────────────────────────────────>│      
   │                                │                                 │      
   │                                │           block_index           │      
   │                                │<────────────────────────────────│      
   │                                │                                 │      
   │          block_index           │                                 │      
   │<───────────────────────────────│                                 │      
 ┌─┴──┐                         ┌───┴────┐                       ┌────┴─────┐
 │User│                         │Elliptic│                       │TAL Ledger│
 └────┘                         └────────┘                       └──────────┘
```

```shell
dfx canister --network ic call protocol repay_to_vault '(record{vault_id=VAULT_ID; amount=BORROWED_AMOUNT})'
```

### Add margin to a vault 
```
 ┌────┐                              ┌────────┐                      ┌────────────┐
 │User│                              │Elliptic│                      │ckBTC Ledger│
 └─┬──┘                              └───┬────┘                      └─────┬──────┘
   │                                     │                                 │       
   │add_margin_to_vault(vault_id, amount)│                                 │       
   │────────────────────────────────────>│                                 │       
   │                                     │                                 │       
   │                                     │icrc2_transfer_from(user, amount)│       
   │                                     │────────────────────────────────>│       
   │                                     │                                 │       
   │                                     │           block_index           │       
   │                                     │<────────────────────────────────│       
   │                                     │                                 │       
   │             block_index             │                                 │       
   │<────────────────────────────────────│                                 │       
 ┌─┴──┐                              ┌───┴────┐                      ┌─────┴──────┐
 │User│                              │Elliptic│                      │ckBTC Ledger│
 └────┘                              └────────┘                      └────────────┘
```
```shell
dfx canister --network ic call protocol add_margin_to_vault '(record{vault_id=VAULT_ID; amount=BORROWED_AMOUNT})'
```
## Close a vault
```
 ┌────┐              ┌────────┐                                     ┌──────────┐┌────────────┐
 │User│              │Elliptic│                                     │TAL Ledger││ckBTC ledger│
 └─┬──┘              └───┬────┘                                     └────┬─────┘└─────┬──────┘
   │                     │                                               │            │       
   │close_vault(vault_id)│                                               │            │       
   │────────────────────>│                                               │            │       
   │                     │                                               │            │       
   │                     │icrc2_transfer_from(user, borrowed_amount_left)│            │       
   │                     │──────────────────────────────────────────────>│            │       
   │                     │                                               │            │       
   │                     │                  block_index                  │            │       
   │                     │<──────────────────────────────────────────────│            │       
   │                     │                                               │            │       
   │                     │                icrc1_transfer(user, margin)   │            │       
   │                     │───────────────────────────────────────────────────────────>│       
   │                     │                                               │            │       
   │                     │                        block_index            │            │       
   │                     │<───────────────────────────────────────────────────────────│       
   │                     │                                               │            │       
   │     block_index     │                                               │            │       
   │<────────────────────│                                               │            │       
 ┌─┴──┐              ┌───┴────┐                                     ┌────┴─────┐┌─────┴──────┐
 │User│              │Elliptic│                                     │TAL Ledger││ckBTC ledger│
 └────┘              └────────┘                                     └──────────┘└────────────┘
```

```shell
dfx canister --network ic call protocol close_vault '(VAULT_ID)'
```
## Liquidity

Users can provide liquidity to the liquidity pool in the form of TAL. The liquidity pool is used to liquidate the vault whose collateral ratio falls below 110%, hence buying ckBTC at a discount.

### Provide liquidity
```
 ┌────┐                  ┌────────┐                       ┌──────────┐
 │User│                  │Elliptic│                       │TAL Ledger│
 └─┬──┘                  └───┬────┘                       └────┬─────┘
   │                         │                                 │      
   │provide_liquidity(amount)│                                 │      
   │────────────────────────>│                                 │      
   │                         │                                 │      
   │                         │icrc2_transfer_from(user, amount)│      
   │                         │────────────────────────────────>│      
   │                         │                                 │      
   │                         │           block_index           │      
   │                         │<────────────────────────────────│      
   │                         │                                 │      
   │       block_index       │                                 │      
   │<────────────────────────│                                 │      
 ┌─┴──┐                  ┌───┴────┐                       ┌────┴─────┐
 │User│                  │Elliptic│                       │TAL Ledger│
 └────┘                  └────────┘                       └──────────┘
```

```shell
dfx canister --network ic call protocol provide_liquidity '(TAL_AMOUNT)'
```
### Withdraw liquidity
```
 ┌────┐                   ┌────────┐                  ┌──────────┐
 │User│                   │Elliptic│                  │TAL Ledger│
 └─┬──┘                   └───┬────┘                  └────┬─────┘
   │                          │                            │      
   │withdraw_liquidity(amount)│                            │      
   │─────────────────────────>│                            │      
   │                          │                            │      
   │                          │icrc1_transfer(user, amount)│      
   │                          │───────────────────────────>│      
   │                          │                            │      
   │                          │        block_index         │      
   │                          │<───────────────────────────│      
   │                          │                            │      
   │       block_index        │                            │      
   │<─────────────────────────│                            │      
 ┌─┴──┐                   ┌───┴────┐                  ┌────┴─────┐
 │User│                   │Elliptic│                  │TAL Ledger│
 └────┘                   └────────┘                  └──────────┘
```
```shell
dfx canister --network ic call protocol withdraw_liquidity '(TAL_AMOUNT)'
```

### Claim liquidity returns
```
 ┌────┐                  ┌────────┐                 ┌────────────┐
 │User│                  │Elliptic│                 │ckBTC Ledger│
 └─┬──┘                  └───┬────┘                 └─────┬──────┘
   │                         │                            │       
   │claim_liquidity_returns()│                            │       
   │────────────────────────>│                            │       
   │                         │                            │       
   │                         │icrc1_transfer(user, amount)│       
   │                         │───────────────────────────>│       
   │                         │                            │       
   │                         │        block_index         │       
   │                         │<───────────────────────────│       
   │                         │                            │       
   │       block_index       │                            │       
   │<────────────────────────│                            │       
 ┌─┴──┐                  ┌───┴────┐                 ┌─────┴──────┐
 │User│                  │Elliptic│                 │ckBTC Ledger│
 └────┘                  └────────┘                 └────────────┘
```
```shell
dfx canister --network ic call protocol claim_liquidity_returns '()'
```

## Query State Data
Note: You need to pass REDEMPTION_AMOUNT to the following endpoint because redemption fees are depending on the amount of TAL you are trying to redeem.
```shell
dfx canister --network ic call protocol get_fees '(REDEMPTION_AMOUNT)'
```

```shell
dfx canister --network ic call protocol get_liquidity_status '(principal "PRINCIPAL")'
```

```shell
dfx canister --network ic call protocol get_protocol_status '()'
```
```shell
dfx canister --network ic call protocol get_vaults '()'
```
```shell
dfx canister --network ic call protocol get_vault_history '(VAULT_ID)'
```
```shell
dfx canister --network ic call protocol get_events '(record { start : START; length : LENGTH })'
```