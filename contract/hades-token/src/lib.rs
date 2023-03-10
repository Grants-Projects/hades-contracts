/*!
Non-Fungible Token implementation with JSON serialization.
NOTES:
  - The maximum balance value is limited by U128 (2**128 - 1).
  - JSON calls should pass U128 as a base-10 string. E.g. "100".
  - The contract optimizes the inner trie structure by hashing account IDs. It will prevent some
    abuse of deep tries. Shouldn't be an issue, once NEAR clients implement full hashing of keys.
  - The contract tracks the change in storage before and after the call. If the storage increases,
    the contract requires the caller of the contract to attach enough deposit to the function call
    to cover the storage cost.
    This is done to prevent a denial of service attack on the contract by taking all available storage.
    If the storage decreases, the contract will issue a refund for the cost of the released storage.
    The unused tokens from the attached deposit are also refunded, so it's safe to
    attach more deposit than required.
  - To prevent the deployed contract from being modified or deleted, it should not have any access
    keys on its account.
*/
use near_contract_standards::non_fungible_token::metadata::{
    NFTContractMetadata, NonFungibleTokenMetadataProvider, TokenMetadata, NFT_METADATA_SPEC,
};
use near_contract_standards::non_fungible_token::NonFungibleToken;
use near_contract_standards::non_fungible_token::{Token, TokenId};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LazyOption, LookupMap, UnorderedSet};
use near_sdk::{
    env, near_bindgen, require, AccountId, BorshStorageKey, PanicOnDefault, Promise, PromiseOrValue,
};
use near_sdk::json_types::Base64VecU8;
use iso8601::datetime;
use std::collections::HashMap;


#[derive(Default, BorshDeserialize, BorshSerialize)]
pub struct TokenCounter {
    value: u128
}

impl TokenCounter {
    pub fn new() -> Self {
        Self {
            value: 0
        }
    }

    pub fn increment(&mut self) -> u128 {
        self.value = self.value + 1;
        self.value
    }
}

enum IdentityApplicationStatus {
    Pending,
    CoVerified,
    GovVerified
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct IdentityMetadata {
    data: String,
    image: String,
    status: u8,
    

}



#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Hades {
    tokens: NonFungibleToken,
    metadata: LazyOption<NFTContractMetadata>,
    token_counter: TokenCounter,
    
}

// const DATA_IMAGE_SVG_NEAR_ICON: &str = "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 288 288'%3E%3Cg id='l' data-name='l'%3E%3Cpath d='M187.58,79.81l-30.1,44.69a3.2,3.2,0,0,0,4.75,4.2L191.86,103a1.2,1.2,0,0,1,2,.91v80.46a1.2,1.2,0,0,1-2.12.77L102.18,77.93A15.35,15.35,0,0,0,90.47,72.5H87.34A15.34,15.34,0,0,0,72,87.84V201.16A15.34,15.34,0,0,0,87.34,216.5h0a15.35,15.35,0,0,0,13.08-7.31l30.1-44.69a3.2,3.2,0,0,0-4.75-4.2L96.14,186a1.2,1.2,0,0,1-2-.91V104.61a1.2,1.2,0,0,1,2.12-.77l89.55,107.23a15.35,15.35,0,0,0,11.71,5.43h3.13A15.34,15.34,0,0,0,216,201.16V87.84A15.34,15.34,0,0,0,200.66,72.5h0A15.35,15.35,0,0,0,187.58,79.81Z'/%3E%3C/g%3E%3C/svg%3E";

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    NonFungibleToken,
    Metadata,
    TokenMetadata,
    Enumeration,
    Approval,
}

#[near_bindgen]
impl Hades {

    #[init]
    pub fn new(owner_id: AccountId) -> Self {
        // require!(!env::state_exists(), "Already initialized");
    
        let metadata = NFTContractMetadata {
                        spec: NFT_METADATA_SPEC.to_string(),
                        name: "Hades Identity NFT Contract".to_string(),
                        symbol: "HADES".to_string(),
                        icon: None,
                        base_uri: None,
                        reference: None,
                        reference_hash: None,
                    };
        Self {
            tokens: NonFungibleToken::new(
                StorageKey::NonFungibleToken,
                owner_id,
                Some(StorageKey::TokenMetadata),
                Some(StorageKey::Enumeration),
                Some(StorageKey::Approval),
            ),
            metadata: LazyOption::new(StorageKey::Metadata, Some(&metadata)),
            token_counter: TokenCounter::new(),

        }
    }

    /// Mint a new token with ID=`token_id` belonging to `token_owner_id`.
    ///
    /// Since this example implements metadata, it also requires per-token metadata to be provided
    /// in this call. `self.tokens.mint` will also require it to be Some, since
    /// `StorageKey::TokenMetadata` was provided at initialization.
    ///
    /// `self.tokens.mint` will enforce `predecessor_account_id` to equal the `owner_id` given in
    /// initialization call to `new`.
    #[payable]
    pub fn nft_mint(
        &mut self,
        token_owner_id: &AccountId,
        property_identifier: String,
        split_identifier: String,
        doc_url: String,
        image_url: String,
    ) -> Token {
        let title = format!("Unit {split_identifier} of property {property_identifier}");
        let description = format!("Token for split {split_identifier} of property {property_identifier}");
        let token_metadata = TokenMetadata {
            title: Some(title),
            description: Some(description),
            media: Some(image_url.clone()),
            media_hash: Some(Base64VecU8::from(image_url.as_bytes().to_vec())),
            copies: Some(1u64),
            issued_at: None,
            expires_at: None,
            starts_at: None,
            updated_at: None,
            extra: None,
            reference: Some(doc_url.clone()),
            reference_hash: Some(Base64VecU8::from(doc_url.as_bytes().to_vec())),
        };
        let token_id = self.token_counter.increment().to_string();
        assert_eq!(env::predecessor_account_id(), self.tokens.owner_id, "Unauthorized");
        self.tokens.internal_mint(token_id, token_owner_id.clone(), Some(token_metadata))
    }

    #[payable]
    pub fn register_for_kyc(&mut self, passport_url: String, metadata_url: String, full_name: String, account_id: AccountId) -> Token {
        let names = full_name.split(" ");
        let mut iter = names.into_iter();

        iter.next();
        iter.next();

        require!(iter.next().is_some(), "Full name must contain at least 3 names separated by a space");
        let token_id = self.token_counter.increment().to_string();

        let zero_fill = "0".repeat(3 - token_id.len());

        let current_timestamp = env::block_timestamp();

        let id = format!("{}{}{}", current_timestamp, zero_fill, token_id);

        let title = format!("KYC for {full_name} with id {id}");
        let description = format!("Hades NFT representing identity for user with name {full_name} and identity number {id}");
        let token_metadata = TokenMetadata {
            title: Some(title),
            description: Some(description),
            media: Some(passport_url.clone()),
            media_hash: Some(Base64VecU8::from(passport_url.as_bytes().to_vec())),
            copies: Some(1u64),
            issued_at: None,
            expires_at: None,
            starts_at: None,
            updated_at: None,
            extra: None,
            reference: Some(metadata_url.clone()),
            reference_hash: Some(Base64VecU8::from(metadata_url.as_bytes().to_vec())),
        };

        self.tokens.internal_mint(id, env::signer_account_id(), Some(token_metadata))


    }


    pub fn set_contract_owner(&mut self, account_id: AccountId) {
        require!(env::signer_account_id() == self.tokens.owner_id, "Not authorized");
        self.tokens.owner_id = account_id;
    }


    pub fn get_user_identity_token(&self, account_id: AccountId) -> Vec<Token> {

        let mut res= Vec::new();

        if let Some(token_by_owner) = &self.tokens.tokens_per_owner {

            let tokens = token_by_owner.get(&account_id).unwrap_or(UnorderedSet::new(b"r"));

            for token_id in tokens.iter() {
                let token = self.nft_token(token_id).unwrap();

                res.push(token);
            }
        }

        res
    }

    


    


    

    pub fn get_token(&self, token_id: TokenId) -> Token {

        self.nft_token(token_id).unwrap_or_else(|| env::panic_str("Token with provided ID doesn't exist"))
    }

    pub fn nft_token(&self, token_id: TokenId) -> Option<Token> {

        let owner_id = self.tokens.owner_by_id.get(&token_id)?;

        let mut metadata = None;
        let mut approved_account_ids = None;

        if let Some(meta) = &self.tokens.token_metadata_by_id {
            metadata = meta.get(&token_id);
        } 

        if let Some(appr) = &self.tokens.approvals_by_id {
            approved_account_ids = appr.get(&token_id);
        }

        Some(Token { token_id, owner_id, metadata, approved_account_ids })

    }

    

    
}

//near_contract_standards::impl_non_fungible_token_core!(RietsToken, tokens);
near_contract_standards::impl_non_fungible_token_approval!(Hades, tokens);
near_contract_standards::impl_non_fungible_token_enumeration!(Hades, tokens);

#[near_bindgen]
impl NonFungibleTokenMetadataProvider for Hades {
    fn nft_metadata(&self) -> NFTContractMetadata {
        self.metadata.get().unwrap()
    }
}
