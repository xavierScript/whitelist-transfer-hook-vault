use anchor_lang::prelude::*;

#[account]
pub struct Config {
    pub admin: Pubkey,
    pub mint: Pubkey,
    pub bump: u8,
}

impl Config {
    pub const SEED: &'static [u8] = b"config";
    // 8 (discriminator) + 32 (admin) + 32 (mint) + 1 (bump) = 73
    pub const SIZE: usize = 8 + 32 + 32 + 1;
}

#[account]
pub struct WhitelistEntry {
    pub bump: u8,
}

impl WhitelistEntry {
    pub const SEED: &'static [u8] = b"whitelist";
    // 8 (discriminator) + 1 (bump) = 9
    pub const SIZE: usize = 8 + 1;
}