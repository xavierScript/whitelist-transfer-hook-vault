use anchor_lang::prelude::*;

use crate::state::Config;

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

impl<'info> InitializeConfig<'info> {
    pub fn initialize_config(&mut self, bumps: InitializeConfigBumps) -> Result<()> {
        self.config.set_inner(Config {
            admin: self.admin.key(),
            mint: Pubkey::default(), // Set later when mint is created
            bump: bumps.config,
        });
        msg!("Config initialized. Admin: {}", self.admin.key());
        Ok(())
    }
}
