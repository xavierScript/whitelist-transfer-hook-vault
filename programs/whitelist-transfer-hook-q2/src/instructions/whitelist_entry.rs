use anchor_lang::prelude::*;

use crate::state::{Config, WhitelistEntry};

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

impl<'info> AddToWhitelist<'info> {
    pub fn add_to_whitelist(&mut self, _user: Pubkey, bumps: AddToWhitelistBumps) -> Result<()> {
        self.whitelist_entry.set_inner(WhitelistEntry {
            bump: bumps.whitelist_entry,
        });
        msg!("User added to whitelist");
        Ok(())
    }
}

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

impl<'info> RemoveFromWhitelist<'info> {
    pub fn remove_from_whitelist(&mut self, _user: Pubkey) -> Result<()> {
        msg!("User removed from whitelist");
        Ok(())
    }
}
