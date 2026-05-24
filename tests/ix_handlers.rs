use anchor_lang::system_program;
use anchor_spl::associated_token;
use litesvm::LiteSVM;
use litesvm_token::CreateAssociatedTokenAccount;
use sha2::{Digest, Sha256};
use solana_keypair::Keypair;
use solana_message::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;
use solana_signer::Signer;

fn disc(ix_name: &str) -> Vec<u8> {
    Sha256::digest(format!("global:{ix_name}").as_bytes())[..8].to_vec()
}

fn token_program_id() -> Pubkey { anchor_spl::token::ID }
fn ata_program_id()   -> Pubkey { anchor_spl::associated_token::ID }

const SEED: u64     = 12364;
const FEE: u16      = 100;        // 1%
const DEPOSIT: u64  = 5_000_000;
const SWAP_IN: u64  = 500_000;
const SWAP_MIN: u64 = 1;

fn ata_address(owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    associated_token::get_associated_token_address(owner, mint)
}

pub fn get_or_create_ata(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint: &Pubkey,
    owner: &Pubkey,
) -> Pubkey {
    let ata = associated_token::get_associated_token_address(owner, mint);
    if svm.get_account(&ata).is_none() {
        CreateAssociatedTokenAccount::new(svm, payer, mint)
            .owner(owner)
            .send()
            .unwrap();
    }
    ata
}

pub fn create_initialise_ix(
    _svm: &mut LiteSVM,
    payer: &Keypair,
    mint_x: Pubkey,
    mint_y: Pubkey,
    config: Pubkey,
    mint_lp: Pubkey,
    vault_x: Pubkey,
    vault_y: Pubkey,
) -> Instruction {
    let mut data = disc("initialize");
    data.extend_from_slice(&SEED.to_le_bytes());
    data.extend_from_slice(&FEE.to_le_bytes());
    data.push(0u8); // authority = None

    Instruction {
        program_id: amm_program::id(),
        accounts: vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new_readonly(mint_x, false),
            AccountMeta::new_readonly(mint_y, false),
            AccountMeta::new(mint_lp, false),
            AccountMeta::new(vault_x, false),
            AccountMeta::new(vault_y, false),
            AccountMeta::new(config, false),
            AccountMeta::new_readonly(token_program_id(), false),
            AccountMeta::new_readonly(ata_program_id(), false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data,
    }
}

pub fn create_deposit_ix(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint_x: Pubkey,
    mint_y: Pubkey,
    mint_lp: Pubkey,
    config: Pubkey,
    vault_x: Pubkey,
    vault_y: Pubkey,
) -> Instruction {
    let user_x  = get_or_create_ata(svm, payer, &mint_x, &payer.pubkey());
    let user_y  = get_or_create_ata(svm, payer, &mint_y, &payer.pubkey());

    let user_lp = ata_address(&payer.pubkey(), &mint_lp);

    let mut data = disc("deposit");
    data.extend_from_slice(&DEPOSIT.to_le_bytes()); // amount
    data.extend_from_slice(&DEPOSIT.to_le_bytes()); // max_x
    data.extend_from_slice(&DEPOSIT.to_le_bytes()); // max_y

    Instruction {
        program_id: amm_program::id(),
        accounts: vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new_readonly(mint_x, false),
            AccountMeta::new_readonly(mint_y, false),
            AccountMeta::new_readonly(config, false),
            AccountMeta::new(mint_lp, false),
            AccountMeta::new(vault_x, false),
            AccountMeta::new(vault_y, false),
            AccountMeta::new(user_x, false),
            AccountMeta::new(user_y, false),
            AccountMeta::new(user_lp, false),
            AccountMeta::new_readonly(token_program_id(), false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(ata_program_id(), false),
        ],
        data,
    }
}

pub fn create_withdraw_ix(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint_x: Pubkey,
    mint_y: Pubkey,
    mint_lp: Pubkey,
    config: Pubkey,
    vault_x: Pubkey,
    vault_y: Pubkey,
) -> Instruction {
    let user_x  = get_or_create_ata(svm, payer, &mint_x, &payer.pubkey());
    let user_y  = get_or_create_ata(svm, payer, &mint_y, &payer.pubkey());
    let user_lp = ata_address(&payer.pubkey(), &mint_lp); // exists post-deposit

    let mut data = disc("withdraw");
    data.extend_from_slice(&DEPOSIT.to_le_bytes()); // amount (LP to burn)
    data.extend_from_slice(&0u64.to_le_bytes());    // min_x
    data.extend_from_slice(&0u64.to_le_bytes());    // min_y

    Instruction {
        program_id: amm_program::id(),
        accounts: vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new_readonly(mint_x, false),
            AccountMeta::new_readonly(mint_y, false),
            AccountMeta::new_readonly(config, false),
            AccountMeta::new(mint_lp, false),
            AccountMeta::new(vault_x, false),
            AccountMeta::new(vault_y, false),
            AccountMeta::new(user_x, false),
            AccountMeta::new(user_y, false),
            AccountMeta::new(user_lp, false),
            AccountMeta::new_readonly(token_program_id(), false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(ata_program_id(), false),
        ],
        data,
    }
}

pub fn create_swap_ix(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint_x: Pubkey,
    mint_y: Pubkey,
    _mint_lp: Pubkey,
    config: Pubkey,
    vault_x: Pubkey,
    vault_y: Pubkey,
) -> Instruction {
    let user_x = get_or_create_ata(svm, payer, &mint_x, &payer.pubkey());
    let user_y = get_or_create_ata(svm, payer, &mint_y, &payer.pubkey());

    let mut data = disc("swap");
    data.push(1u8);                                  // is_x = true
    data.extend_from_slice(&SWAP_IN.to_le_bytes());
    data.extend_from_slice(&SWAP_MIN.to_le_bytes());

    Instruction {
        program_id: amm_program::id(),
        accounts: vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new_readonly(mint_x, false),
            AccountMeta::new_readonly(mint_y, false),
            AccountMeta::new(user_x, false),
            AccountMeta::new(user_y, false),
            AccountMeta::new(vault_x, false),
            AccountMeta::new(vault_y, false),
            AccountMeta::new_readonly(config, false),
            AccountMeta::new_readonly(token_program_id(), false),
            AccountMeta::new_readonly(ata_program_id(), false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data,
    }
}