use anchor_spl::associated_token;
use litesvm::LiteSVM;
use litesvm_token::{CreateAssociatedTokenAccount, CreateMint, MintTo};
use solana_hash::Hash;
use solana_keypair::Keypair;
use solana_message::{Instruction, Message, VersionedMessage};
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::versioned::VersionedTransaction;

mod ix_handlers;
use ix_handlers::*;

fn send(
    svm: &mut LiteSVM,
    ixs: &[Instruction],
    payer: &Keypair,
    signers: &[&Keypair],
) -> litesvm::types::TransactionResult {
    svm.expire_blockhash();
    let blockhash: Hash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(ixs, Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), signers).unwrap();
    svm.send_transaction(tx)
}

fn token_balance(svm: &LiteSVM, ata: &Pubkey) -> u64 {
    let account = svm.get_account(ata).expect("account not found");
    u64::from_le_bytes(account.data[64..72].try_into().unwrap())
}

fn setup() -> (LiteSVM, Keypair, Pubkey, Pubkey, Pubkey, Pubkey, Pubkey, Pubkey) {
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();

    let bytes: &[u8] = include_bytes!("../target/deploy/amm_program.so");
    svm.add_program(amm_program::id(), bytes).unwrap();
    svm.airdrop(&payer.pubkey(), 1_000_000_000).unwrap();

    let mint_x = CreateMint::new(&mut svm, &payer)
        .decimals(6)
        .send()
        .unwrap();

    let mint_y = CreateMint::new(&mut svm, &payer)
        .decimals(6)
        .send()
        .unwrap();

    let seed: u64 = 12364;
    let config =
        Pubkey::find_program_address(&[b"config", &seed.to_le_bytes()], &amm_program::id()).0;
    let mint_lp =
        Pubkey::find_program_address(&[b"lp", config.as_ref()], &amm_program::id()).0;
    let vault_x = associated_token::get_associated_token_address(&config, &mint_x);
    let vault_y = associated_token::get_associated_token_address(&config, &mint_y);

    (svm, payer, mint_x, mint_y, config, mint_lp, vault_x, vault_y)
}

fn fund_payer_accounts(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint_x: &Pubkey,
    mint_y: &Pubkey,
    amount: u64,
) -> (Pubkey, Pubkey) {
    let payer_x = CreateAssociatedTokenAccount::new(svm, payer, mint_x)
        .send()
        .unwrap();
    let payer_y = CreateAssociatedTokenAccount::new(svm, payer, mint_y)
        .send()
        .unwrap();

    MintTo::new(svm, payer, mint_x, &payer_x, amount)
        .send()
        .unwrap();
    MintTo::new(svm, payer, mint_y, &payer_y, amount)
        .send()
        .unwrap();

    (payer_x, payer_y)
}

#[test]
fn test_initialize() {
    let (mut svm, payer, mint_x, mint_y, config, mint_lp, vault_x, vault_y) = setup();

    let ix = create_initialise_ix(
        &mut svm, &payer, mint_x, mint_y, config, mint_lp, vault_x, vault_y,
    );
    let res = send(&mut svm, &[ix], &payer, &[&payer]);
    assert!(res.is_ok(), "initialize failed: {res:?}");

    assert!(svm.get_account(&config).is_some(),  "config PDA not created");
    assert!(svm.get_account(&mint_lp).is_some(), "LP mint not created");
    assert!(svm.get_account(&vault_x).is_some(), "vault_x not created");
    assert!(svm.get_account(&vault_y).is_some(), "vault_y not created");
}

#[test]
pub fn test_deposit() {
    let (mut svm, payer, mint_x, mint_y, config, mint_lp, vault_x, vault_y) = setup();
    fund_payer_accounts(&mut svm, &payer, &mint_x, &mint_y, 10_000_000);

    let init_ix = create_initialise_ix(
        &mut svm, &payer, mint_x, mint_y, config, mint_lp, vault_x, vault_y,
    );
    let deposit_ix = create_deposit_ix(
        &mut svm, &payer, mint_x, mint_y, mint_lp, config, vault_x, vault_y,
    );
    let res = send(&mut svm, &[init_ix, deposit_ix], &payer, &[&payer]);
    assert!(res.is_ok(), "deposit failed: {res:?}");

    assert!(token_balance(&svm, &vault_x) > 0, "vault_x must hold tokens");
    assert!(token_balance(&svm, &vault_y) > 0, "vault_y must hold tokens");

    let payer_lp = associated_token::get_associated_token_address(&payer.pubkey(), &mint_lp);
    assert!(token_balance(&svm, &payer_lp) > 0, "payer must hold LP tokens");
}

#[test]
pub fn test_withdraw() {
    let (mut svm, payer, mint_x, mint_y, config, mint_lp, vault_x, vault_y) = setup();
    let (payer_x, payer_y) =
        fund_payer_accounts(&mut svm, &payer, &mint_x, &mint_y, 10_000_000);

    let init_ix = create_initialise_ix(
        &mut svm, &payer, mint_x, mint_y, config, mint_lp, vault_x, vault_y,
    );
    let deposit_ix = create_deposit_ix(
        &mut svm, &payer, mint_x, mint_y, mint_lp, config, vault_x, vault_y,
    );
    let withdraw_ix = create_withdraw_ix(
        &mut svm, &payer, mint_x, mint_y, mint_lp, config, vault_x, vault_y,
    );
    let res = send(
        &mut svm,
        &[init_ix, deposit_ix, withdraw_ix],
        &payer,
        &[&payer],
    );
    assert!(res.is_ok(), "withdraw failed: {res:?}");

    assert_eq!(token_balance(&svm, &vault_x), 0, "vault_x must be empty");
    assert_eq!(token_balance(&svm, &vault_y), 0, "vault_y must be empty");
    assert!(token_balance(&svm, &payer_x) > 0, "payer_x must be restored");
    assert!(token_balance(&svm, &payer_y) > 0, "payer_y must be restored");
}


#[test]
pub fn test_swap() {
    let (mut svm, payer, mint_x, mint_y, config, mint_lp, vault_x, vault_y) = setup();
    fund_payer_accounts(&mut svm, &payer, &mint_x, &mint_y, 10_000_000);

    let init_ix = create_initialise_ix(
        &mut svm, &payer, mint_x, mint_y, config, mint_lp, vault_x, vault_y,
    );
    let deposit_ix = create_deposit_ix(
        &mut svm, &payer, mint_x, mint_y, mint_lp, config, vault_x, vault_y,
    );
    // Send init + deposit first so we can snapshot state before the swap
    let res = send(&mut svm, &[init_ix, deposit_ix], &payer, &[&payer]);
    assert!(res.is_ok(), "init+deposit failed: {res:?}");

    let vx_before = token_balance(&svm, &vault_x) as u128;
    let vy_before = token_balance(&svm, &vault_y) as u128;
    let k_before  = vx_before * vy_before;

    let payer_y_ata = associated_token::get_associated_token_address(&payer.pubkey(), &mint_y);
    let y_before = token_balance(&svm, &payer_y_ata);

    let swap_ix = create_swap_ix(
        &mut svm, &payer, mint_x, mint_y, mint_lp, config, vault_x, vault_y,
    );
    let res = send(&mut svm, &[swap_ix], &payer, &[&payer]);
    assert!(res.is_ok(), "swap failed: {res:?}");

    // Payer received Y
    assert!(token_balance(&svm, &payer_y_ata) > y_before, "payer must receive Y");
    // vault_x grew (X sold into pool)
    assert!(token_balance(&svm, &vault_x) as u128 > vx_before, "vault_x must grow");
    // Constant-product k must not decrease
    let k_after =
        token_balance(&svm, &vault_x) as u128 * token_balance(&svm, &vault_y) as u128;
    assert!(k_after >= k_before, "k violated: before={k_before} after={k_after}");
}