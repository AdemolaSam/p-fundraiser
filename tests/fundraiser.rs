#![allow(dead_code)]

use {
    litesvm::LiteSVM,
    litesvm_token::{CreateAssociatedTokenAccount, CreateMint, MintTo, TOKEN_ID, get_spl_account},
    solana_address::Address,
    solana_clock::Clock,
    solana_instruction::{AccountMeta, Instruction},
    solana_keypair::Keypair,
    solana_signer::Signer,
    solana_transaction::Transaction,
    spl_associated_token_account::get_associated_token_address_with_program_id,
    std::{path::PathBuf, str::FromStr},
};

const PROGRAM_ID: &str = "BHxV2zsi55UNqDL4ns2e6iyQWTJj78qeyRYcc9N4RoT1";
const INITIALIZE: u8 = 0;
const CONTRIBUTE: u8 = 1;
const REFUND: u8 = 2;
const WITHDRAW: u8 = 3;
const TARGET: u64 = 1_000_000;
const CONTRIBUTION: u64 = 250_000;

fn program_id() -> Address {
    Address::from_str(PROGRAM_ID).unwrap()
}

fn program_so() -> PathBuf {
    if let Ok(path) = std::env::var("FUNDRAISER_SO") {
        return PathBuf::from(path);
    }

    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/deploy/p_fundraiser.so")
}

fn setup() -> (LiteSVM, Keypair, Address) {
    let mut svm = LiteSVM::new();
    let so = program_so();
    assert!(
        so.exists(),
        "missing SBF artifact: {}. Build it first or set FUNDRAISER_SO=/path/to/p_fundraiser.so",
        so.display()
    );
    svm.add_program_from_file(program_id(), so).unwrap();

    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();

    let mint = CreateMint::new(&mut svm, &payer)
        .decimals(0)
        .send()
        .unwrap();
    (svm, payer, mint)
}

fn fundraiser_pda(maker: &Address) -> (Address, u8) {
    Address::find_program_address(&[b"fundraiser", maker.as_ref()], &program_id())
}

fn contributor_pda(fundraiser: &Address, contributor: &Address) -> (Address, u8) {
    Address::find_program_address(
        &[b"contributor", fundraiser.as_ref(), contributor.as_ref()],
        &program_id(),
    )
}

fn vault_ata(fundraiser: &Address, mint: &Address) -> Address {
    get_associated_token_address_with_program_id(fundraiser, mint, &TOKEN_ID)
}

fn send_ix(svm: &mut LiteSVM, payer: &Keypair, signers: &[&Keypair], ix: Instruction) -> bool {
    let mut all_signers = Vec::with_capacity(signers.len() + 1);
    all_signers.push(payer);
    all_signers.extend_from_slice(signers);

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &all_signers,
        svm.latest_blockhash(),
    );

    match svm.send_transaction(tx) {
        Ok(_) => true,
        Err(err) => {
            eprintln!("transaction failed: {err:?}");
            false
        }
    }
}

fn initialize_ix(maker: &Address, mint: &Address, amount: u64, duration: u8) -> Instruction {
    let (fundraiser, bump) = fundraiser_pda(maker);
    let vault = vault_ata(&fundraiser, mint);
    let mut data = vec![INITIALIZE, bump, duration];
    data.extend_from_slice(&amount.to_le_bytes());

    Instruction {
        program_id: program_id(),
        accounts: vec![
            AccountMeta::new(*maker, true),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new(fundraiser, false),
            AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false),
            AccountMeta::new_readonly(TOKEN_ID, false),
            AccountMeta::new(vault, false),
            AccountMeta::new_readonly(spl_associated_token_account::ID, false),
        ],
        data,
    }
}

fn contribute_ix(
    maker: &Address,
    contributor: &Address,
    mint: &Address,
    amount: u64,
) -> Instruction {
    let (fundraiser, bump) = fundraiser_pda(maker);
    let (contributor_state, contributor_bump) = contributor_pda(&fundraiser, contributor);
    let contributor_ata =
        get_associated_token_address_with_program_id(contributor, mint, &TOKEN_ID);
    let vault = vault_ata(&fundraiser, mint);
    let mut data = vec![CONTRIBUTE, bump, contributor_bump];
    data.extend_from_slice(&amount.to_le_bytes());

    Instruction {
        program_id: program_id(),
        accounts: vec![
            AccountMeta::new_readonly(*maker, false),
            AccountMeta::new(fundraiser, false),
            AccountMeta::new(*contributor, true),
            AccountMeta::new(contributor_state, false),
            AccountMeta::new(contributor_ata, false),
            AccountMeta::new(vault, false),
            AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false),
            AccountMeta::new_readonly(TOKEN_ID, false),
            AccountMeta::new_readonly(spl_associated_token_account::ID, false),
        ],
        data,
    }
}

fn refund_ix(maker: &Address, contributor: &Address, mint: &Address) -> Instruction {
    let (fundraiser, bump) = fundraiser_pda(maker);
    let (contributor_state, contributor_bump) = contributor_pda(&fundraiser, contributor);
    let contributor_ata =
        get_associated_token_address_with_program_id(contributor, mint, &TOKEN_ID);
    let vault = vault_ata(&fundraiser, mint);

    Instruction {
        program_id: program_id(),
        accounts: vec![
            AccountMeta::new_readonly(*maker, false),
            AccountMeta::new(*contributor, true),
            AccountMeta::new(contributor_ata, false),
            AccountMeta::new(contributor_state, false),
            AccountMeta::new(fundraiser, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false),
            AccountMeta::new_readonly(TOKEN_ID, false),
            AccountMeta::new(vault, false),
            AccountMeta::new_readonly(spl_associated_token_account::ID, false),
        ],
        data: vec![REFUND, bump, contributor_bump],
    }
}

fn withdraw_ix(maker: &Address, mint: &Address) -> Instruction {
    let (fundraiser, _bump) = fundraiser_pda(maker);
    let maker_ata = get_associated_token_address_with_program_id(maker, mint, &TOKEN_ID);
    let vault = vault_ata(&fundraiser, mint);

    Instruction {
        program_id: program_id(),
        accounts: vec![
            AccountMeta::new(*maker, true),
            AccountMeta::new(maker_ata, false),
            AccountMeta::new(fundraiser, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new(vault, false),
            AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false),
            AccountMeta::new_readonly(TOKEN_ID, false),
            AccountMeta::new_readonly(spl_associated_token_account::ID, false),
        ],
        data: vec![WITHDRAW],
    }
}

fn init_and_fund_contributor(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint: &Address,
    contributor: &Keypair,
    amount: u64,
) -> Address {
    svm.airdrop(&contributor.pubkey(), 1_000_000_000).unwrap();
    let ata = CreateAssociatedTokenAccount::new(svm, payer, mint)
        .owner(&contributor.pubkey())
        .send()
        .unwrap();
    MintTo::new(svm, payer, mint, &ata, amount).send().unwrap();
    ata
}

fn current_amount(svm: &LiteSVM, fundraiser: &Address) -> u64 {
    let account = svm.get_account(fundraiser).unwrap();
    u64::from_le_bytes(account.data[72..80].try_into().unwrap())
}

fn contributor_amount(svm: &LiteSVM, contributor_state: &Address) -> u64 {
    let account = svm.get_account(contributor_state).unwrap();
    u64::from_le_bytes(account.data[0..8].try_into().unwrap())
}

#[test]
fn initialize_happy_path() {
    let (mut svm, maker, mint) = setup();
    assert!(send_ix(
        &mut svm,
        &maker,
        &[],
        initialize_ix(&maker.pubkey(), &mint, TARGET, 10)
    ));

    let (fundraiser, _) = fundraiser_pda(&maker.pubkey());
    assert!(svm.get_account(&fundraiser).is_some());
    assert!(svm.get_account(&vault_ata(&fundraiser, &mint)).is_some());
}

#[test]
fn contribute_happy_path() {
    let (mut svm, maker, mint) = setup();
    let contributor = Keypair::new();
    init_and_fund_contributor(&mut svm, &maker, &mint, &contributor, CONTRIBUTION);
    assert!(send_ix(
        &mut svm,
        &maker,
        &[],
        initialize_ix(&maker.pubkey(), &mint, TARGET, 10)
    ));
    assert!(send_ix(
        &mut svm,
        &maker,
        &[&contributor],
        contribute_ix(&maker.pubkey(), &contributor.pubkey(), &mint, CONTRIBUTION),
    ));

    let (fundraiser, _) = fundraiser_pda(&maker.pubkey());
    let (contributor_state, _) = contributor_pda(&fundraiser, &contributor.pubkey());
    assert_eq!(current_amount(&svm, &fundraiser), CONTRIBUTION);
    assert_eq!(contributor_amount(&svm, &contributor_state), CONTRIBUTION);
}

#[test]
fn refund_happy_path() {
    let (mut svm, maker, mint) = setup();
    let contributor = Keypair::new();
    let contributor_ata =
        init_and_fund_contributor(&mut svm, &maker, &mint, &contributor, CONTRIBUTION);
    assert!(send_ix(
        &mut svm,
        &maker,
        &[],
        initialize_ix(&maker.pubkey(), &mint, TARGET, 0)
    ));
    assert!(send_ix(
        &mut svm,
        &maker,
        &[&contributor],
        contribute_ix(&maker.pubkey(), &contributor.pubkey(), &mint, CONTRIBUTION),
    ));

    let mut clock = svm.get_sysvar::<Clock>();
    clock.unix_timestamp += 1;
    svm.set_sysvar(&clock);

    assert!(send_ix(
        &mut svm,
        &maker,
        &[&contributor],
        refund_ix(&maker.pubkey(), &contributor.pubkey(), &mint),
    ));

    let token_account: litesvm_token::spl_token::state::Account =
        get_spl_account(&svm, &contributor_ata).unwrap();
    assert_eq!(token_account.amount, CONTRIBUTION);
}

#[test]
fn withdraw_happy_path() {
    let (mut svm, maker, mint) = setup();
    let contributor = Keypair::new();
    init_and_fund_contributor(&mut svm, &maker, &mint, &contributor, TARGET);
    CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint)
        .send()
        .unwrap();
    assert!(send_ix(
        &mut svm,
        &maker,
        &[],
        initialize_ix(&maker.pubkey(), &mint, TARGET, 10)
    ));
    assert!(send_ix(
        &mut svm,
        &maker,
        &[&contributor],
        contribute_ix(&maker.pubkey(), &contributor.pubkey(), &mint, TARGET),
    ));
    assert!(send_ix(
        &mut svm,
        &maker,
        &[],
        withdraw_ix(&maker.pubkey(), &mint)
    ));
}

#[test]
fn initialize_unhappy_invalid_amount() {
    let (mut svm, maker, mint) = setup();
    assert!(!send_ix(
        &mut svm,
        &maker,
        &[],
        initialize_ix(&maker.pubkey(), &mint, 1, 10)
    ));
}

#[test]
fn contribute_unhappy_invalid_contributor_pda() {
    let (mut svm, maker, mint) = setup();
    let contributor = Keypair::new();
    init_and_fund_contributor(&mut svm, &maker, &mint, &contributor, CONTRIBUTION);
    assert!(send_ix(
        &mut svm,
        &maker,
        &[],
        initialize_ix(&maker.pubkey(), &mint, TARGET, 10)
    ));

    let mut ix = contribute_ix(&maker.pubkey(), &contributor.pubkey(), &mint, CONTRIBUTION);
    ix.accounts[3] = AccountMeta::new(Address::new_unique(), false);
    assert!(!send_ix(&mut svm, &maker, &[&contributor], ix));
}

#[test]
fn refund_unhappy_still_ongoing() {
    let (mut svm, maker, mint) = setup();
    let contributor = Keypair::new();
    init_and_fund_contributor(&mut svm, &maker, &mint, &contributor, CONTRIBUTION);
    assert!(send_ix(
        &mut svm,
        &maker,
        &[],
        initialize_ix(&maker.pubkey(), &mint, TARGET, 10)
    ));
    assert!(send_ix(
        &mut svm,
        &maker,
        &[&contributor],
        contribute_ix(&maker.pubkey(), &contributor.pubkey(), &mint, CONTRIBUTION),
    ));
    assert!(!send_ix(
        &mut svm,
        &maker,
        &[&contributor],
        refund_ix(&maker.pubkey(), &contributor.pubkey(), &mint),
    ));
}

#[test]
fn withdraw_unhappy_target_not_met() {
    let (mut svm, maker, mint) = setup();
    let contributor = Keypair::new();
    init_and_fund_contributor(&mut svm, &maker, &mint, &contributor, CONTRIBUTION);
    CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint)
        .send()
        .unwrap();
    assert!(send_ix(
        &mut svm,
        &maker,
        &[],
        initialize_ix(&maker.pubkey(), &mint, TARGET, 10)
    ));
    assert!(send_ix(
        &mut svm,
        &maker,
        &[&contributor],
        contribute_ix(&maker.pubkey(), &contributor.pubkey(), &mint, CONTRIBUTION),
    ));
    assert!(!send_ix(
        &mut svm,
        &maker,
        &[],
        withdraw_ix(&maker.pubkey(), &mint)
    ));
}
