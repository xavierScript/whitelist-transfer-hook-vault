# Whitelist Transfer Hook

This example demonstrates how to implement a transfer hook using the SPL Token 2022 Transfer Hook interface to enforce whitelist restrictions on token transfers.

In this example, only whitelisted addresses will be able to transfer tokens that have this transfer hook enabled, providing fine-grained access control over token movements.

The latest version of the program also includes an end-to-end vault mint flow. The admin can create a Token-2022 mint with the TransferHook and MetadataPointer extensions already configured, attach token metadata directly to the mint, initialize a vault token account for that mint, and then deposit or withdraw tokens through the whitelist-checked transfer hook.

## Current Flow

1. Initialize the global config PDA.
2. Add approved users to the whitelist with per-user PDA accounts.
3. Create a vault mint with transfer hook and metadata support.
4. Initialize the vault token account for that mint.
5. Initialize the extra account meta list used by the transfer hook.
6. Deposit and withdraw tokens only for whitelisted users.

---

## Architecture

This program uses **per-user PDA accounts** to manage whitelist status. Instead of storing all whitelisted addresses in a single `Vec<Pubkey>`, each whitelisted user gets their own small PDA account. The **existence** of the PDA is the whitelist check — O(1) lookup instead of O(n) vector scanning.

The program has 2 state accounts:

### Config Account

A global configuration PDA that stores the admin authority. Seeds: `[b"config"]`.

```rust
#[account]
pub struct Config {
    pub admin: Pubkey,
    pub bump: u8,
}
```

- **admin**: The public key of the admin who can add/remove users from the whitelist.
- **bump**: The bump seed used to derive the Config PDA.

### WhitelistEntry Account

A per-user PDA that proves a user is whitelisted. Seeds: `[b"whitelist", user_pubkey.as_ref()]`.

```rust
#[account]
pub struct WhitelistEntry {
    pub bump: u8,
}
```

- **bump**: The bump seed used to derive this WhitelistEntry PDA.

The account is only 9 bytes (8 discriminator + 1 bump). Its mere existence means the user is whitelisted. When a user is removed, the account is closed and rent is refunded to the admin.

---

### The admin initializes the Config account. For that, we create the following context:

```rust
#[derive(Accounts)]
pub struct InitializeConfig<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        init,
        payer = admin,
        space = Config::SIZE,
        seeds = [Config::SEED],
        bump
    )]
    pub config: Account<'info, Config>,
    pub system_program: Program<'info, System>,
}
```

Let's have a closer look at the accounts that we are passing in this context:

- **admin**: Will be the person creating the config account. He will be a signer of the transaction, and we mark his account as mutable as we will be deducting lamports from this account.

- **config**: Will be the state account that we will initialize. The admin will be paying for the initialization. We derive the Config PDA from the byte representation of the word "config".

- **system_program**: Program responsible for the initialization of any new account.

### We then implement some functionality for our InitializeConfig context:

```rust
impl<'info> InitializeConfig<'info> {
    pub fn initialize_config(&mut self, bumps: InitializeConfigBumps) -> Result<()> {
        self.config.set_inner(Config {
            admin: self.admin.key(),
            bump: bumps.config,
        });
        msg!("Config initialized. Admin: {}", self.admin.key());
        Ok(())
    }
}
```

In here, we set the initial data of our Config account with the admin's public key and store the bump.

---

### The admin can add users to the whitelist by creating individual WhitelistEntry PDAs:

```rust
#[derive(Accounts)]
#[instruction(user: Pubkey)]
pub struct AddToWhitelist<'info> {
    #[account(
        mut,
        constraint = admin.key() == config.admin @ ErrorCode::ConstraintAddress
    )]
    pub admin: Signer<'info>,
    #[account(
        seeds = [Config::SEED],
        bump = config.bump,
    )]
    pub config: Account<'info, Config>,
    #[account(
        init,
        payer = admin,
        space = WhitelistEntry::SIZE,
        seeds = [WhitelistEntry::SEED, user.as_ref()],
        bump
    )]
    pub whitelist_entry: Account<'info, WhitelistEntry>,
    pub system_program: Program<'info, System>,
}
```

In this context, we are passing all the accounts needed to add a user to the whitelist:

- **admin**: The address of the platform admin. He must match the admin stored in the Config account. He will be a signer and payer for the new WhitelistEntry account.

- **config**: The global config account used to verify admin authority. We derive it from the "config" seed.

- **whitelist_entry**: The per-user PDA that will be created. We derive it from `[b"whitelist", user_pubkey]`. Anchor's `init` constraint handles account creation, rent payment, and space allocation automatically.

- **system_program**: Program responsible for account creation.

### We then implement some functionality for our AddToWhitelist context:

```rust
impl<'info> AddToWhitelist<'info> {
    pub fn add_to_whitelist(&mut self, _user: Pubkey, bumps: AddToWhitelistBumps) -> Result<()> {
        self.whitelist_entry.set_inner(WhitelistEntry {
            bump: bumps.whitelist_entry,
        });
        msg!("User added to whitelist");
        Ok(())
    }
}
```

Adding a user is simply creating their WhitelistEntry PDA and storing the bump. No reallocation is needed — Anchor's `init` handles everything.

---

### The admin can remove users from the whitelist by closing their WhitelistEntry PDAs:

```rust
#[derive(Accounts)]
#[instruction(user: Pubkey)]
pub struct RemoveFromWhitelist<'info> {
    #[account(
        mut,
        constraint = admin.key() == config.admin @ ErrorCode::ConstraintAddress
    )]
    pub admin: Signer<'info>,
    #[account(
        seeds = [Config::SEED],
        bump = config.bump,
    )]
    pub config: Account<'info, Config>,
    #[account(
        mut,
        seeds = [WhitelistEntry::SEED, user.as_ref()],
        bump = whitelist_entry.bump,
        close = admin,
    )]
    pub whitelist_entry: Account<'info, WhitelistEntry>,
    pub system_program: Program<'info, System>,
}
```

Removing a user is achieved by Anchor's `close = admin` constraint, which zeroes out the account data and refunds all rent lamports back to the admin. No manual reallocation or lamport manipulation is required.

---

### The system will need to initialize extra account metadata for the transfer hook:

```rust
#[derive(Accounts)]
pub struct InitializeExtraAccountMetaList<'info> {
    #[account(mut)]
    payer: Signer<'info>,

    /// CHECK: ExtraAccountMetaList Account, must use these seeds
    #[account(
        init,
        seeds = [b"extra-account-metas", mint.key().as_ref()],
        bump,
        space = ExtraAccountMetaList::size_of(
            InitializeExtraAccountMetaList::extra_account_metas()?.len()
        ).unwrap(),
        payer = payer
    )]
    pub extra_account_meta_list: AccountInfo<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    pub system_program: Program<'info, System>,
}
```

In this context, we are passing all the accounts needed to set up the transfer hook metadata:

- **payer**: The address paying for the initialization. He will be a signer of the transaction, and we mark his account as mutable as we will be deducting lamports from this account.

- **extra_account_meta_list**: The account that will store the extra metadata required for the transfer hook. This account is derived from the byte representation of "extra-account-metas" and the mint's public key.

- **mint**: The token mint that will have the transfer hook enabled.

- **system_program**: Program responsible for the initialization of any new account.

### We then implement some functionality for our InitializeExtraAccountMetaList context:

```rust
impl<'info> InitializeExtraAccountMetaList<'info> {
    pub fn extra_account_metas() -> Result<Vec<ExtraAccountMeta>> {
        // Dynamic PDA resolution: seeds = [b"whitelist", owner_pubkey]
        // In the transfer hook interface, account indices are:
        //   0 = source_token
        //   1 = mint
        //   2 = destination_token
        //   3 = owner (source token authority)
        // We use index 3 to derive the per-user whitelist entry PDA.
        Ok(vec![ExtraAccountMeta::new_with_seeds(
            &[
                Seed::Literal {
                    bytes: b"whitelist".to_vec(),
                },
                Seed::AccountKey { index: 3 }, // owner = source token authority
            ],
            false, // is_signer
            false, // is_writable
        )?])
    }
}
```

This is a key architectural change from the original design. Instead of hardcoding a single static whitelist PDA via `new_with_pubkey`, we use **dynamic PDA seed resolution** via `new_with_seeds`. The seeds include:

1. A literal `"whitelist"` prefix
2. The public key at account index 3 (the `owner` — i.e., the source token authority)

At transfer time, the Token 2022 runtime automatically derives the correct per-user WhitelistEntry PDA from the sender's public key. This means each transfer resolves to the specific user's whitelist entry without any on-chain iteration.

---

### The transfer hook will validate every token transfer:

```rust
#[derive(Accounts)]
pub struct TransferHook<'info> {
    #[account(
        token::mint = mint,
        token::authority = owner,
    )]
    pub source_token: InterfaceAccount<'info, TokenAccount>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        token::mint = mint,
    )]
    pub destination_token: InterfaceAccount<'info, TokenAccount>,
    /// CHECK: source token account owner, can be SystemAccount or PDA owned by another program
    pub owner: UncheckedAccount<'info>,
    /// CHECK: ExtraAccountMetaList Account,
    #[account(
        seeds = [b"extra-account-metas", mint.key().as_ref()],
        bump
    )]
    pub extra_account_meta_list: UncheckedAccount<'info>,
    #[account(
        seeds = [WhitelistEntry::SEED, owner.key().as_ref()],
        bump = whitelist_entry.bump,
    )]
    pub whitelist_entry: Account<'info, WhitelistEntry>,
}
```

In this context, we are passing all the accounts needed for transfer validation:

- **source_token**: The token account from which tokens are being transferred. We validate that it belongs to the correct mint and is owned by the owner.

- **mint**: The token mint being transferred.

- **destination_token**: The token account to which tokens are being transferred. We validate that it belongs to the correct mint.

- **owner**: The owner of the source token account. This can be a system account or a PDA owned by another program.

- **extra_account_meta_list**: The metadata account that contains information about extra accounts required for this transfer hook.

- **whitelist_entry**: The per-user WhitelistEntry PDA, derived from `[b"whitelist", owner_pubkey]`. If this account doesn't exist, the transaction fails automatically — meaning the user is not whitelisted. If it exists and the seeds match, Anchor's constraint validation passes, confirming the user is authorized.

### We then implement some functionality for our TransferHook context:

```rust
impl<'info> TransferHook<'info> {
    /// This function is called when the transfer hook is executed.
    pub fn transfer_hook(&mut self, _amount: u64) -> Result<()> {
        // Fail this instruction if it is not called from within a transfer hook
        self.check_is_transferring()?;

        // If we reach this point, the WhitelistEntry PDA exists and the
        // seeds constraint validated it matches the source token owner.
        // The user is whitelisted — O(1) lookup via PDA existence.
        msg!("Transfer allowed: {} is whitelisted", self.owner.key());

        Ok(())
    }

    /// Checks if the transfer hook is being executed during a transfer operation.
    fn check_is_transferring(&mut self) -> Result<()> {
        let source_token_info = self.source_token.to_account_info();
        let mut account_data_ref: RefMut<&mut [u8]> = source_token_info.try_borrow_mut_data()?;
        let mut account = PodStateWithExtensionsMut::<PodAccount>::unpack(*account_data_ref)?;
        let account_extension = account.get_extension_mut::<TransferHookAccount>()?;

        if !bool::from(account_extension.transferring) {
            panic!("TransferHook: Not transferring");
        }

        Ok(())
    }
}
```

In this implementation, we first verify that the hook is being called during an actual transfer operation by checking the transfer hook account extension. Then, since the `whitelist_entry` account's seed constraint already validated against the `owner` key, we know the sender is whitelisted. No `Vec::contains()` scan is needed — the PDA's existence **is** the whitelist check.

The transfer hook integrates seamlessly with the SPL Token 2022 transfer process, automatically validating every transfer attempt against individual whitelist entries without requiring additional user intervention.

## Vault Mint Creation

The `create_vault_mint` instruction prepares a mint that is ready for the rest of the vault workflow. It:

- allocates enough space for the mint, the TransferHook extension, the MetadataPointer extension, and token metadata
- initializes the transfer hook so transfers route through this program
- points mint metadata back to the mint account itself
- initializes the mint with 9 decimals
- writes token metadata fields such as name, symbol, and URI into the mint account
- stores the mint key in the config account for later vault initialization

This means the admin only needs to create the mint once, then the vault and transfer hook setup can build on top of it.

## Vault Lifecycle

The vault flow is split into a small set of instructions that work together:

- `initialize_vault` creates the vault token account as a PDA owned by the config account and tied to the configured mint.
- `initialize_transfer_hook` writes the extra account meta list for the mint so Token-2022 can resolve the whitelist PDA dynamically at transfer time.
- `deposit` transfers tokens from a whitelisted user into the vault.
- `withdraw` transfers tokens from the vault back to a whitelisted user.

In the current test flow, the admin first whitelists the user and the config PDA, creates the vault mint, initializes the vault, and then performs a deposit and withdraw against that mint.

---

## Design Comparison

| Aspect                   | Old (Vec\<Pubkey\>)                        | New (Per-User PDA)                                  |
| ------------------------ | ------------------------------------------ | --------------------------------------------------- |
| Lookup cost              | O(n) linear scan                           | O(1) — PDA exists or it doesn't                     |
| Add cost                 | `realloc` + manual lamport transfer + push | `init` a fixed 9-byte account                       |
| Remove cost              | `realloc` + manual lamport refund + remove | `close` the account (rent auto-refunded)            |
| Account size             | Grows without bound                        | Fixed 9 bytes per entry                             |
| Extra account resolution | Static (hardcoded PDA)                     | Dynamic (per-user seed derivation at transfer time) |

This whitelist transfer hook provides a robust, scalable access control mechanism for Token 2022 mints, ensuring that only pre-approved addresses can transfer tokens while maintaining the standard token interface that users and applications expect.
