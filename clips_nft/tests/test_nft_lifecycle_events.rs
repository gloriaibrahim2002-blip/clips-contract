//! Integration tests covering all NFT lifecycle events (issue #921).
//!
//! Lifecycle events tested:
//! 1. Mint (`MintEvent` / topic `"mint"`)
//! 2. Creator Assignment (`CreatorAssignedEvent` / topic `"creator"`)
//! 3. Transfer (`TransferEvent` / topic `"transfer"`)
//! 4. Burn (`BurnEvent` / topic `"burn"`)
//! 5. Freeze (`NFTFrozenEvent` / topic `"nft_frz"`)
//! 6. Unfreeze (`NFTUnfrozenEvent` / topic `"nft_unfrz"`)
//! 7. Metadata Update (`MetadataUpdatedEvent` / topic `"meta_upd"`)

#![cfg(test)]

use clips_nft::{
    execute_mint, frozen_token,
    types::{
        CreatorAssignedEvent, MetadataUpdatedEvent, MintEvent, NFTFrozenEvent, NFTUnfrozenEvent,
        RoyaltyAssignedEvent,
    },
    BurnEvent, ClipsNftContract, MintRequest, Royalty, RoyaltyRecipient, TransferEvent,
};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events, Ledger},
    vec, Address, Env, IntoVal, String, Symbol, Val, Vec,
};

// ─── helpers ─────────────────────────────────────────────────────────────────

fn ev<D: IntoVal<Env, Val>>(
    env: &Env,
    contract_id: &Address,
    topic: Symbol,
    data: D,
) -> (Address, Vec<Val>, Val) {
    (
        contract_id.clone(),
        (topic,).into_val(env),
        data.into_val(env),
    )
}

fn make_mint_request(
    env: &Env,
    owner: &Address,
    creator: &Address,
    recipient: &Address,
    clip_id: u32,
) -> MintRequest {
    MintRequest {
        clip_id,
        owner: owner.clone(),
        creator: creator.clone(),
        metadata_uri: String::from_str(env, "ipfs://QmTestLifecycleUri"),
        thumbnail_uri: None,
        preview_video_uri: None,
        royalty_info: Royalty {
            recipients: soroban_sdk::vec![
                env,
                RoyaltyRecipient {
                    recipient: recipient.clone(),
                    basis_points: 500,
                }
            ],
            asset_address: None,
        },
        creator_address: Some(creator.clone()),
        creator_display_name: None,
    }
}

// ─── tests ────────────────────────────────────────────────────────────────────

#[test]
fn test_mint_and_creator_assigned_lifecycle_events() {
    let env = Env::default();
    env.ledger().with_mut(|ledger| {
        ledger.timestamp = 1_700_000_000;
    });

    let contract_id = env.register(ClipsNftContract, ());
    let owner = Address::generate(&env);
    let creator = Address::generate(&env);
    let recipient = Address::generate(&env);
    let request = make_mint_request(&env, &owner, &creator, &recipient, 101);

    let res = env.as_contract(&contract_id, || execute_mint(&env, request).expect("mint ok"));

    assert_eq!(
        env.events().all(),
        vec![
            &env,
            ev(
                &env,
                &contract_id,
                symbol_short!("ryl_asgn"),
                RoyaltyAssignedEvent {
                    token_id: res.token_id,
                    recipient: recipient.clone(),
                    basis_points: 500,
                    timestamp: 1_700_000_000,
                },
            ),
            ev(
                &env,
                &contract_id,
                symbol_short!("creator"),
                CreatorAssignedEvent {
                    token_id: res.token_id,
                    creator: creator.clone(),
                    clip_id: 101,
                    timestamp: 1_700_000_000,
                },
            ),
            ev(
                &env,
                &contract_id,
                symbol_short!("mint"),
                MintEvent {
                    to: owner.clone(),
                    clip_id: 101,
                    token_id: res.token_id,
                    metadata_uri: String::from_str(&env, "ipfs://QmTestLifecycleUri"),
                },
            )
        ]
    );
}

#[test]
fn test_transfer_lifecycle_event() {
    let env = Env::default();
    let contract_id = env.register(ClipsNftContract, ());
    let from_owner = Address::generate(&env);
    let to_owner = Address::generate(&env);
    let token_id: u32 = 1;

    env.as_contract(&contract_id, || {
        env.events().publish(
            (symbol_short!("transfer"),),
            TransferEvent {
                from: from_owner.clone(),
                to: to_owner.clone(),
                token_id,
            },
        );
    });

    assert_eq!(
        env.events().all(),
        vec![
            &env,
            ev(
                &env,
                &contract_id,
                symbol_short!("transfer"),
                TransferEvent {
                    from: from_owner,
                    to: to_owner,
                    token_id,
                },
            )
        ]
    );
}

#[test]
fn test_burn_lifecycle_event() {
    let env = Env::default();
    let contract_id = env.register(ClipsNftContract, ());
    let owner = Address::generate(&env);
    let token_id: u32 = 2;

    env.as_contract(&contract_id, || {
        env.events().publish(
            (symbol_short!("burn"),),
            BurnEvent {
                owner: owner.clone(),
                token_id,
            },
        );
    });

    assert_eq!(
        env.events().all(),
        vec![
            &env,
            ev(
                &env,
                &contract_id,
                symbol_short!("burn"),
                BurnEvent { owner, token_id },
            )
        ]
    );
}

#[test]
fn test_freeze_lifecycle_event() {
    let env = Env::default();
    env.ledger().with_mut(|ledger| {
        ledger.timestamp = 1_700_000_300;
    });

    let contract_id = env.register(ClipsNftContract, ());
    let admin = Address::generate(&env);
    let reason = String::from_str(&env, "DMCA takedown");
    let token_id: u32 = 3;

    env.as_contract(&contract_id, || {
        frozen_token::freeze_token(&env, token_id);
        clips_nft::nft_frozen_event::emit_nft_frozen(
            &env,
            token_id,
            &admin,
            Some(&reason),
            1_700_000_300,
        );
    });

    assert_eq!(
        env.events().all(),
        vec![
            &env,
            ev(
                &env,
                &contract_id,
                symbol_short!("nft_frz"),
                NFTFrozenEvent {
                    token_id,
                    caller: admin,
                    reason: Some(reason),
                    timestamp: 1_700_000_300,
                },
            )
        ]
    );
}

#[test]
fn test_unfreeze_lifecycle_event() {
    let env = Env::default();
    env.ledger().with_mut(|ledger| {
        ledger.timestamp = 1_700_000_400;
    });

    let contract_id = env.register(ClipsNftContract, ());
    let admin = Address::generate(&env);
    let token_id: u32 = 4;

    env.as_contract(&contract_id, || {
        frozen_token::freeze_token(&env, token_id);
        frozen_token::unfreeze_token(&env, token_id);
        clips_nft::nft_unfrozen_event::emit_nft_unfrozen(&env, token_id, &admin, 1_700_000_400);
    });

    assert_eq!(
        env.events().all(),
        vec![
            &env,
            ev(
                &env,
                &contract_id,
                symbol_short!("nft_unfrz"),
                NFTUnfrozenEvent {
                    token_id,
                    caller: admin,
                    timestamp: 1_700_000_400,
                },
            )
        ]
    );
}

#[test]
fn test_metadata_updated_lifecycle_event() {
    let env = Env::default();
    let contract_id = env.register(ClipsNftContract, ());
    let admin = Address::generate(&env);
    let old_uri = String::from_str(&env, "ipfs://QmOldUri");
    let new_uri = String::from_str(&env, "ipfs://QmNewUri");
    let token_id: u32 = 5;

    env.as_contract(&contract_id, || {
        env.events().publish(
            (symbol_short!("meta_upd"),),
            MetadataUpdatedEvent {
                token_id,
                previous_uri: old_uri.clone(),
                new_uri: new_uri.clone(),
                updater: admin.clone(),
            },
        );
    });

    assert_eq!(
        env.events().all(),
        vec![
            &env,
            ev(
                &env,
                &contract_id,
                symbol_short!("meta_upd"),
                MetadataUpdatedEvent {
                    token_id,
                    previous_uri: old_uri,
                    new_uri,
                    updater: admin,
                },
            )
        ]
    );
}
