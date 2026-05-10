use anchor_lang::prelude::*;
use anchor_spl::{
    token_interface::{Mint, TokenAccount, TokenInterface},
};

use crate::state::Config;

#[derive(Accounts)]
pub struct InitializeVault<'info> {
    #[account(
        mut,
        constraint = admin.key() == config.admin @ ErrorCode::ConstraintAddress
    )]
    pub admin: Signer<'info>,

    #[account(
        seeds = [Config::SEED],
        bump = config.bump,
        has_one = mint, // ensure this is the vault's mint
    )]
    pub config: Account<'info, Config>,

    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        init,
        payer = admin,
        seeds = [b"vault-token", config.key().as_ref()],
        bump,
        token::mint = mint,
        token::authority = config,
    )]
    pub vault_token_account: InterfaceAccount<'info, TokenAccount>,

    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
}

impl<'info> InitializeVault<'info> {
    pub fn initialize_vault(&mut self, _bumps: InitializeVaultBumps) -> Result<()> {
        msg!("Vault initialized. Admin: {}", self.admin.key());
        Ok(())
    }
}