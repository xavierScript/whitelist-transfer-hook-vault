use anchor_lang::{prelude::*, system_program::{create_account, CreateAccount}, solana_program::program::invoke};
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenInterface},
};
use spl_token_2022_interface::{
    extension::{
        transfer_hook::instruction::initialize as init_transfer_hook,
        ExtensionType,
    },
    state::Mint as MintState,
};
use spl_token_metadata_interface::{
    instruction::initialize as init_token_metadata,
    state::TokenMetadata,
};
// use solana_program::program::invoke;

use crate::state::Config;

#[derive(Accounts)]
pub struct CreateVaultMint<'info> {
    #[account(
        mut,
        constraint = admin.key() == config.admin @ ErrorCode::ConstraintAddress
    )]
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [Config::SEED],
        bump = config.bump,
    )]
    pub config: Account<'info, Config>,

    #[account(mut)]
    pub mint: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
}

impl<'info> CreateVaultMint<'info> {
    pub fn create_vault_mint(
        &mut self,
        name: String,
        symbol: String,
        uri: String,
    ) -> Result<()> {
        let space = ExtensionType::try_calculate_account_len::<MintState>(&[
            ExtensionType::TransferHook,
            ExtensionType::MetadataPointer,
        ])
        .unwrap();

        // Calculate metadata space
        let meta_data_space = 250; // max size for metadata approx
        
        let total_space = space + meta_data_space;
        let rent = Rent::get()?.minimum_balance(total_space);

        // 1. Create Account
        create_account(
            CpiContext::new(
                self.system_program.key(),
                CreateAccount {
                    from: self.admin.to_account_info(),
                    to: self.mint.to_account_info(),
                },
            ),
            rent,
            total_space as u64,
            &self.token_program.key(),
        )?;

        // 2. Initialize Transfer Hook Extension
        let transfer_hook_ix = init_transfer_hook(
            &self.token_program.key(),
            &self.mint.key(),
            Some(self.admin.key()),
            Some(crate::ID),
        )?;
        invoke(
            &transfer_hook_ix,
            &[
                self.token_program.to_account_info(),
                self.mint.to_account_info(),
            ],
        )?;

        // 3. Initialize Metadata Pointer Extension
        let init_meta_pointer_ix = spl_token_2022_interface::extension::metadata_pointer::instruction::initialize(
            &self.token_program.key(),
            &self.mint.key(),
            Some(self.admin.key()),
            Some(self.mint.key()), // metadata is inside the mint itself
        )?;
        invoke(
            &init_meta_pointer_ix,
            &[
                self.mint.to_account_info(),
            ],
        )?;

        // 4. Initialize Mint
        let init_mint_ix = spl_token_2022_interface::instruction::initialize_mint2(
            &self.token_program.key(),
            &self.mint.key(),
            &self.admin.key(),
            None,
            9,
        )?;
        invoke(
            &init_mint_ix,
            &[
                self.mint.to_account_info(),
            ],
        )?;

        // 5. Initialize Token Metadata Extension
        let init_metadata_ix = init_token_metadata(
            &self.token_program.key(),
            &self.mint.key(),
            &self.admin.key(),
            &self.mint.key(),
            &self.admin.key(),
            name,
            symbol,
            uri,
        );
        invoke(
            &init_metadata_ix,
            &[
                self.mint.to_account_info(),
                self.admin.to_account_info(),
            ],
        )?;

        // Store the mint in config
        self.config.mint = self.mint.key();

        msg!("Vault Mint created successfully");

        Ok(())
    }
}
