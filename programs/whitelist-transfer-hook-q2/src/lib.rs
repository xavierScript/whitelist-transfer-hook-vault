#![allow(unexpected_cfgs)]
#![allow(deprecated)]

pub mod instructions;
pub mod state;

pub use instructions::*;
pub use state::*;

use anchor_lang::prelude::*;
use spl_discriminator::SplDiscriminate;
use spl_tlv_account_resolution::state::ExtraAccountMetaList;
use spl_transfer_hook_interface::instruction::ExecuteInstruction;

declare_id!("GUwN4GZEyQ9dgZwJNt2c8iy2e9hYCUh6p6hHXJhpZwdJ");

#[program]
pub mod whitelist_transfer_hook_q2 {
    use super::*;

    pub fn initialize_config(ctx: Context<InitializeConfig>) -> Result<()> {
        ctx.accounts.initialize_config(ctx.bumps)
    }

    pub fn add_to_whitelist(ctx: Context<AddToWhitelist>, user: Pubkey) -> Result<()> {
        ctx.accounts.add_to_whitelist(user, ctx.bumps)
    }

    pub fn remove_from_whitelist(ctx: Context<RemoveFromWhitelist>, user: Pubkey) -> Result<()> {
        ctx.accounts.remove_from_whitelist(user)
    }

    pub fn create_vault_mint(
        ctx: Context<CreateVaultMint>, 
        name: String, 
        symbol: String, 
        uri: String
    ) -> Result<()> {
        ctx.accounts.create_vault_mint(name, symbol, uri)
    }

    pub fn initialize_vault(ctx: Context<InitializeVault>) -> Result<()> {
        ctx.accounts.initialize_vault(ctx.bumps)
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        ctx.accounts.deposit(amount)
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        ctx.accounts.withdraw(amount)
    }

    pub fn initialize_transfer_hook(ctx: Context<InitializeExtraAccountMetaList>) -> Result<()> {
        msg!("Initializing Transfer Hook...");

        // Get the extra account metas for the transfer hook
        let extra_account_metas = InitializeExtraAccountMetaList::extra_account_metas()?;

        msg!("Extra Account Metas: {:?}", extra_account_metas);
        msg!("Extra Account Metas Length: {}", extra_account_metas.len());

        // initialize ExtraAccountMetaList account with extra accounts
        ExtraAccountMetaList::init::<ExecuteInstruction>(
            &mut ctx.accounts.extra_account_meta_list.try_borrow_mut_data()?,
            &extra_account_metas,
        )
        .unwrap();

        Ok(())
    }

    #[instruction(discriminator = ExecuteInstruction::SPL_DISCRIMINATOR_SLICE)]
    pub fn transfer_hook(ctx: Context<TransferHook>, amount: u64) -> Result<()> {
        // Call the transfer hook logic
        ctx.accounts.transfer_hook(amount)
    }
}
