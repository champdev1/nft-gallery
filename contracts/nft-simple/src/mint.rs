use crate::*;

#[near_bindgen]
impl Contract {
    #[payable]
    pub fn nft_mint(
        &mut self,
        token_id: Option<TokenId>,
        metadata: TokenMetadata,
        perpetual_royalties: Option<HashMap<AccountId, u32>>,
        receiver_id: Option<ValidAccountId>,
    ) {

        let mut final_token_id = format!("{}", self.token_metadata_by_id.len() + 1);
        if let Some(token_id) = token_id {
            final_token_id = token_id
        }

        let initial_storage_usage = env::storage_usage();
        let mut owner_id = env::predecessor_account_id();
        if let Some(receiver_id) = receiver_id {
            owner_id = receiver_id.into();
        }

        let mut royalty = HashMap::new();
        let mut total_perpetual = 0;
        // user added perpetual_royalties (percentage paid with every transfer)
        if let Some(perpetual_royalties) = perpetual_royalties {
            assert!(perpetual_royalties.len() < 7, "Cannot add more than 6 perpetual royalty amounts");
            for (account, amount) in perpetual_royalties {
                royalty.insert(account, amount);
                total_perpetual += amount;
            }
        }

        // arbitrary
        assert!(total_perpetual < 2001, "Perpetual royalties cannot be more than 20%");

        let mut owner_key = "owner:".to_string();
        if self.owner_id != owner_id {
            // protect owner_id royalty entry from being replaced in nft_transfer_payout
            owner_key.push_str(&owner_id);
            royalty.insert(owner_key, 10000 - total_perpetual - self.contract_royalty);
            royalty.insert(self.owner_id.clone(), self.contract_royalty);
        } else {
            owner_key.push_str(&self.owner_id);
            // contract owner minting for primary sale
            royalty.insert(owner_key, 10000 - total_perpetual);
        }

        env::log(format!("Token Royalties: {:?}", royalty).as_bytes());
        let sum: u32 = royalty.values().map(|a| *a).reduce(|a, b| a + b).unwrap();
        assert_eq!(sum, 10000, "Royalties sum must be exactly 10000");

        let token = Token {
            owner_id,
            approved_account_ids: Default::default(),
            next_approval_id: 0,
            royalty,
        };
        assert!(
            self.tokens_by_id.insert(&final_token_id, &token).is_none(),
            "Token already exists"
        );
        self.token_metadata_by_id.insert(&final_token_id, &metadata);
        self.internal_add_token_to_owner(&token.owner_id, &final_token_id);

        // custom enforce limits to special token types 
        // type is based on metadata.extra
        // check the hard_cap_by_type limits
        if metadata.extra.is_some() {
            let token_type = metadata.extra.unwrap();
            let cap = u64::from(*self.hard_cap_by_type.get(&token_type).expect("Token type must have hard cap."));
            let supply = u64::from(self.nft_supply_for_type(&token_type));
            assert!(supply < cap, "Cannot mint anymore of token type.");
            let mut tokens_per_type = self.tokens_per_type.get(&token_type).unwrap_or_else(|| {
                UnorderedSet::new(hash_account_id(&token_type).try_to_vec().unwrap())
            });
            tokens_per_type.insert(&final_token_id);
            self.tokens_per_type.insert(&token_type, &tokens_per_type);
        }
        
        let new_token_size_in_bytes = env::storage_usage() - initial_storage_usage;
        let required_storage_in_bytes =
            self.extra_storage_in_bytes_per_token + new_token_size_in_bytes;

        refund_deposit(required_storage_in_bytes);
    }
}