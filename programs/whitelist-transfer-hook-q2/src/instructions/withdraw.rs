use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};

use crate::state::{Config, WhitelistEntry};

#[derive(Accounts)]
pub struct Withdraw<'info> {
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

    /// CHECK: The whitelist entry for the config PDA (which is the source token owner)
    #[account(
        seeds = [WhitelistEntry::SEED, config.key().as_ref()],
        bump,
    )]
    pub config_whitelist_entry: UncheckedAccount<'info>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> Withdraw<'info> {
    pub fn withdraw(&mut self, amount: u64) -> Result<()> {
        msg!("Withdrawing {} tokens from the vault", amount);

        let config_key = self.config.key();
        let config_bump = self.config.bump;
        let config_seeds = &[
            b"config".as_ref(),
            &[config_bump],
        ];
        let signer = &[&config_seeds[..]];

        // We use token_interface::transfer_checked to trigger the transfer hook
        let cpi_accounts = TransferChecked {
            from: self.vault_token_account.to_account_info(),
            mint: self.mint.to_account_info(),
            to: self.user_token_account.to_account_info(),
            authority: self.config.to_account_info(), // config is the authority of vault token account
        };

        let mut cpi_ctx = CpiContext::new_with_signer(self.token_program.key(), cpi_accounts, signer);
        
        cpi_ctx = cpi_ctx.with_remaining_accounts(vec![
            self.extra_account_meta_list.to_account_info(),
            self.config_whitelist_entry.to_account_info(), // The hook expects the source owner's whitelist entry!
            self.transfer_hook_program.to_account_info(),
        ]);

        transfer_checked(cpi_ctx, amount, self.mint.decimals)?;

        Ok(())
    }
}
