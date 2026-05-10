use {
    anchor_lang::{
        solana_program::{
            self,
            instruction::{AccountMeta, Instruction},
            pubkey::Pubkey,
            system_instruction,
        },
        InstructionData, ToAccountMetas,
    },
    litesvm::LiteSVM,
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
    spl_associated_token_account_interface::{
        address::get_associated_token_address_with_program_id,
        instruction::create_associated_token_account,
    },
    spl_token_2022_interface::{
        extension::{transfer_hook::instruction::initialize as init_transfer_hook, ExtensionType},
        instruction::{initialize_mint2, mint_to, transfer_checked},
        state::Mint,
        ID as TOKEN_2022_ID,
    },
    whitelist_transfer_hook_q2 as program,
};

fn send(
    svm: &mut LiteSVM,
    ixs: &[Instruction],
    payer: &Keypair,
    signers: &[&Keypair],
) -> litesvm::types::TransactionResult {
    svm.expire_blockhash();
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(ixs, Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), signers).unwrap();
    svm.send_transaction(tx)
}

#[test]
fn test_full_flow() {
    let mut svm = LiteSVM::new();
    let payer = Keypair::new();
    let recipient = Keypair::new();

    let program_id = program::id();
    let bytes = include_bytes!("../../../target/deploy/whitelist_transfer_hook_q2.so");
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();

    let (config_pda, _) = Pubkey::find_program_address(&[b"config"], &program_id);
    let (whitelist_entry_pda, _) = Pubkey::find_program_address(
        &[b"whitelist", payer.pubkey().as_ref()],
        &program_id,
    );
    let (config_whitelist_pda, _) = Pubkey::find_program_address(
        &[b"whitelist", config_pda.as_ref()],
        &program_id,
    );
    let system_program_id = solana_program::system_program::id();
    let token_program_id = spl_token_2022_interface::ID;
    let associated_token_program_id = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL".parse::<Pubkey>().unwrap();

    // Step 1: Initialize config
    let ix = Instruction::new_with_bytes(
        program_id,
        &program::instruction::InitializeConfig {}.data(),
        program::accounts::InitializeConfig {
            admin: payer.pubkey(),
            config: config_pda,
            system_program: system_program_id,
        }
        .to_account_metas(None),
    );
    send(&mut svm, &[ix], &payer, &[&payer]).expect("initialize_config failed");

    // Step 2: Add user (payer) to whitelist
    let ix = Instruction::new_with_bytes(
        program_id,
        &program::instruction::AddToWhitelist {
            user: payer.pubkey(),
        }
        .data(),
        program::accounts::AddToWhitelist {
            admin: payer.pubkey(),
            config: config_pda,
            whitelist_entry: whitelist_entry_pda,
            system_program: system_program_id,
        }
        .to_account_metas(None),
    );
    send(&mut svm, &[ix], &payer, &[&payer]).expect("add_to_whitelist failed");

    // Step 2.5: Add config PDA to whitelist (needed for vault withdrawals)
    let ix = Instruction::new_with_bytes(
        program_id,
        &program::instruction::AddToWhitelist {
            user: config_pda,
        }
        .data(),
        program::accounts::AddToWhitelist {
            admin: payer.pubkey(),
            config: config_pda,
            whitelist_entry: config_whitelist_pda,
            system_program: system_program_id,
        }
        .to_account_metas(None),
    );
    send(&mut svm, &[ix], &payer, &[&payer]).expect("add_config_to_whitelist failed");

    // Step 3: Create Vault Mint
    let mint = Keypair::new();
    let ix = Instruction::new_with_bytes(
        program_id,
        &program::instruction::CreateVaultMint {
            name: "Vault Token".to_string(),
            symbol: "VTK".to_string(),
            uri: "https://example.com/meta.json".to_string(),
        }
        .data(),
        program::accounts::CreateVaultMint {
            admin: payer.pubkey(),
            config: config_pda,
            mint: mint.pubkey(),
            system_program: system_program_id,
            token_program: token_program_id,
        }
        .to_account_metas(None),
    );
    send(&mut svm, &[ix], &payer, &[&payer, &mint]).expect("create_vault_mint failed");

    // Step 4: Initialize Vault
    let (vault_token_account, _) = Pubkey::find_program_address(
        &[b"vault-token", config_pda.as_ref()],
        &program_id,
    );
    
    let ix = Instruction::new_with_bytes(
        program_id,
        &program::instruction::InitializeVault {}.data(),
        program::accounts::InitializeVault {
            admin: payer.pubkey(),
            config: config_pda,
            mint: mint.pubkey(),
            vault_token_account,
            system_program: system_program_id,
            token_program: token_program_id,
        }
        .to_account_metas(None),
    );
    send(&mut svm, &[ix], &payer, &[&payer]).expect("initialize_vault failed");

    // Step 5: Initialize ExtraAccountMetaList for the transfer hook
    let (extra_meta_pda, _) = Pubkey::find_program_address(
        &[b"extra-account-metas", mint.pubkey().as_ref()],
        &program_id,
    );

    let ix = Instruction::new_with_bytes(
        program_id,
        &program::instruction::InitializeTransferHook {}.data(),
        program::accounts::InitializeExtraAccountMetaList {
            payer: payer.pubkey(),
            extra_account_meta_list: extra_meta_pda,
            mint: mint.pubkey(),
            system_program: system_program_id,
        }
        .to_account_metas(None),
    );
    send(&mut svm, &[ix], &payer, &[&payer]).expect("initialize_transfer_hook failed");

    // Step 6: Create user ATA and mint some tokens directly (simulating initial distribution)
    let source_ata = get_associated_token_address_with_program_id(
        &payer.pubkey(),
        &mint.pubkey(),
        &token_program_id,
    );
    let create_source_ata = create_associated_token_account(
        &payer.pubkey(),
        &payer.pubkey(),
        &mint.pubkey(),
        &token_program_id,
    );
    let mint_amount = 100u64 * 10u64.pow(9);
    let mint_to_ix = mint_to(
        &token_program_id,
        &mint.pubkey(),
        &source_ata,
        &payer.pubkey(), // Admin is still mint authority
        &[],
        mint_amount,
    )
    .unwrap();
    send(&mut svm, &[create_source_ata, mint_to_ix], &payer, &[&payer])
        .expect("create user ata and mint_to failed");

    // Step 7: Deposit (User -> Vault)
    let deposit_amount = 50u64 * 10u64.pow(9);
    let deposit_ix = Instruction::new_with_bytes(
        program_id,
        &program::instruction::Deposit { amount: deposit_amount }.data(),
        program::accounts::Deposit {
            user: payer.pubkey(),
            config: config_pda,
            whitelist_entry: whitelist_entry_pda,
            mint: mint.pubkey(),
            user_token_account: source_ata,
            vault_token_account,
            extra_account_meta_list: extra_meta_pda,
            transfer_hook_program: program_id,
            token_program: token_program_id,
            associated_token_program: associated_token_program_id,
            system_program: system_program_id,
        }
        .to_account_metas(None),
    );
    send(&mut svm, &[deposit_ix], &payer, &[&payer]).expect("deposit failed");

    // Step 8: Withdraw (Vault -> User)
    let withdraw_amount = 20u64 * 10u64.pow(9);
    let withdraw_ix = Instruction::new_with_bytes(
        program_id,
        &program::instruction::Withdraw { amount: withdraw_amount }.data(),
        program::accounts::Withdraw {
            user: payer.pubkey(),
            config: config_pda,
            whitelist_entry: whitelist_entry_pda,
            mint: mint.pubkey(),
            user_token_account: source_ata,
            vault_token_account,
            extra_account_meta_list: extra_meta_pda,
            transfer_hook_program: program_id,
            config_whitelist_entry: config_whitelist_pda,
            token_program: token_program_id,
            associated_token_program: associated_token_program_id,
            system_program: system_program_id,
        }
        .to_account_metas(None),
    );
    send(&mut svm, &[withdraw_ix], &payer, &[&payer]).expect("withdraw failed");

    // Step 9: Remove user from whitelist
    let ix = Instruction::new_with_bytes(
        program_id,
        &program::instruction::RemoveFromWhitelist {
            user: payer.pubkey(),
        }
        .data(),
        program::accounts::RemoveFromWhitelist {
            admin: payer.pubkey(),
            config: config_pda,
            whitelist_entry: whitelist_entry_pda,
            system_program: system_program_id,
        }
        .to_account_metas(None),
    );
    send(&mut svm, &[ix], &payer, &[&payer]).expect("remove_from_whitelist failed");

    // Step 10: Deposit should fail
    let deposit_ix_fail = Instruction::new_with_bytes(
        program_id,
        &program::instruction::Deposit { amount: 10 * 10u64.pow(9) }.data(),
        program::accounts::Deposit {
            user: payer.pubkey(),
            config: config_pda,
            whitelist_entry: whitelist_entry_pda,
            mint: mint.pubkey(),
            user_token_account: source_ata,
            vault_token_account,
            extra_account_meta_list: extra_meta_pda,
            transfer_hook_program: program_id,
            token_program: token_program_id,
            associated_token_program: associated_token_program_id,
            system_program: system_program_id,
        }
        .to_account_metas(None),
    );
    let res = send(&mut svm, &[deposit_ix_fail], &payer, &[&payer]);
    assert!(res.is_err(), "deposit should fail — user is not whitelisted");
}
