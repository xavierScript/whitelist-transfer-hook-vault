use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};

use crate::state::{Config, WhitelistEntry};

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        seeds = [Config::SEED],
        bump = config.bump,
        has_one = mint, // ensures correct mint
    )]
    pub config: Account<'info, Config>,

    #[account(
        seeds = [WhitelistEntry::SEED, user.key().as_ref()],
        bump = whitelist_entry.bump,
    )]
    pub whitelist_entry: Account<'info, WhitelistEntry>,

    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = user,
    )]
    pub user_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [b"vault-token", config.key().as_ref()],
        bump,
    )]
    pub vault_token_account: InterfaceAccount<'info, TokenAccount>,

    /// CHECK: Extra account meta list for the transfer hook
    #[account(
        seeds = [b"extra-account-metas", mint.key().as_ref()],
        bump,
    )]
    pub extra_account_meta_list: UncheckedAccount<'info>,

    /// CHECK: Transfer hook program id
    #[account(address = crate::ID)]
    pub transfer_hook_program: UncheckedAccount<'info>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> Deposit<'info> {
    pub fn deposit(&mut self, amount: u64) -> Result<()> {
        msg!("Depositing {} tokens to the vault", amount);

        // We use token_interface::transfer_checked to trigger the transfer hook
        let cpi_accounts = TransferChecked {
            from: self.user_token_account.to_account_info(),
            mint: self.mint.to_account_info(),
            to: self.vault_token_account.to_account_info(),
            authority: self.user.to_account_info(),
        };

        // For transfer hook to work, we must pass the extra account meta list,
        // the transfer hook program, and the whitelist entry (which the hook expects)
        let mut cpi_ctx = CpiContext::new(self.token_program.key(), cpi_accounts);
        
        cpi_ctx = cpi_ctx.with_remaining_accounts(vec![
            self.extra_account_meta_list.to_account_info(),
            self.whitelist_entry.to_account_info(), // The hook resolves this dynamically via seed!
            self.transfer_hook_program.to_account_info(),
        ]);

        transfer_checked(cpi_ctx, amount, self.mint.decimals)?;

        Ok(())
    }
}