#![allow(clippy::integer_arithmetic)]
#![cfg(feature = "test-sbf")]
mod helpers;

use {
    helpers::*,
    mpl_token_metadata::{
        state::{MAX_NAME_LENGTH, MAX_SYMBOL_LENGTH, MAX_URI_LENGTH},
        utils::puffed_out_string,
    },
    solana_program_test::*,
    solana_sdk::{signature::Signer, transaction::Transaction},
    spl_single_validator_pool::{error::SinglePoolError, id, instruction},
    test_case::test_case,
};

const UPDATED_NAME: &str = "updated_name";
const UPDATED_SYMBOL: &str = "USYM";
const UPDATED_URI: &str = "updated_uri";

#[tokio::test]
async fn success_update_pool_token_metadata() {
    let mut context = program_test().start_with_context().await;
    let accounts = SinglePoolAccounts::default();
    accounts.initialize(&mut context).await;

    let puffed_name = puffed_out_string(UPDATED_NAME, MAX_NAME_LENGTH);
    let puffed_symbol = puffed_out_string(UPDATED_SYMBOL, MAX_SYMBOL_LENGTH);
    let puffed_uri = puffed_out_string(UPDATED_URI, MAX_URI_LENGTH);

    let instruction = instruction::update_token_metadata(
        &id(),
        &accounts.vote_account.pubkey(),
        &accounts.withdrawer.pubkey(),
        UPDATED_NAME.to_string(),
        UPDATED_SYMBOL.to_string(),
        UPDATED_URI.to_string(),
    );

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&context.payer.pubkey()),
        &[&context.payer, &accounts.withdrawer],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let metadata = get_metadata_account(&mut context.banks_client, &accounts.mint).await;

    assert_eq!(metadata.data.name, puffed_name);
    assert_eq!(metadata.data.symbol, puffed_symbol);
    assert_eq!(metadata.data.uri, puffed_uri);
}

#[tokio::test]
async fn fail_no_signature() {
    let mut context = program_test().start_with_context().await;
    let accounts = SinglePoolAccounts::default();
    accounts.initialize(&mut context).await;

    let mut instruction = instruction::update_token_metadata(
        &id(),
        &accounts.vote_account.pubkey(),
        &accounts.withdrawer.pubkey(),
        UPDATED_NAME.to_string(),
        UPDATED_SYMBOL.to_string(),
        UPDATED_URI.to_string(),
    );
    assert_eq!(instruction.accounts[3].pubkey, accounts.withdrawer.pubkey());
    instruction.accounts[3].is_signer = false;

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    let e = context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err();
    check_error(e, SinglePoolError::SignatureMissing);
}

enum BadWithdrawer {
    Validator,
    Voter,
    VoteAccount,
}

#[test_case(BadWithdrawer::Validator; "validator")]
#[test_case(BadWithdrawer::Voter; "voter")]
#[test_case(BadWithdrawer::VoteAccount; "vote_account")]
#[tokio::test]
async fn fail_bad_withdrawer(withdrawer_type: BadWithdrawer) {
    let mut context = program_test().start_with_context().await;
    let accounts = SinglePoolAccounts::default();
    accounts.initialize(&mut context).await;

    let withdrawer = match withdrawer_type {
        BadWithdrawer::Validator => &accounts.validator,
        BadWithdrawer::Voter => &accounts.voter,
        BadWithdrawer::VoteAccount => &accounts.vote_account,
    };

    let instruction = instruction::update_token_metadata(
        &id(),
        &accounts.vote_account.pubkey(),
        &withdrawer.pubkey(),
        UPDATED_NAME.to_string(),
        UPDATED_SYMBOL.to_string(),
        UPDATED_URI.to_string(),
    );

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&context.payer.pubkey()),
        &[&context.payer, withdrawer],
        context.last_blockhash,
    );

    let e = context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err();
    check_error(e, SinglePoolError::InvalidMetadataSigner);
}
