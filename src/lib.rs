use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault, Promise, PromiseResult,  Gas, Balance};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::collections::{LookupMap, UnorderedMap, UnorderedSet};
use near_sdk::BorshStorageKey;
use near_sdk::json_types::U128;

//use near_sdk::env::{account_balance, is_valid_account_id};
//use near_sdk::env::{account_balance};
//use near_sdk::env::{account_balance, current_account_id};

use near_sdk::ext_contract;


const NO_DEPOSIT: Balance = 0;
const BASE_GAS: Gas =  3_000_000_000_000;

const YOCTO_NEAR: u128 = 1_000_000_000_000_000_000_000_000; // 1 followed by 24 zeros

/*******************************/
/*********** STRUCTS ***********/
/*******************************/
pub type TransactionId = u128; //********* 
pub type Price = u128;
pub type TokenId = String;

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub enum TransactionStatus {
    Pending,
    TokensLocked,
    TokensAndNFTLocked,
    Completed,
    Cancelled,
}

/// Helper structure for keys of the persistent collections.
/*
    Since all data stored on the blockchain is kept in a single key-value store under the contract account, 
    you must always use a unique storage prefix for different collections to avoid data collision. It is used in the initialization funct.
    https://near.github.io/near-sdk-as/classes/_sdk_core_assembly_collections_persistentmap_.persistentmap.html#constructor
*/
#[derive(BorshStorageKey, BorshSerialize)]
pub enum StorageKeys {
    TransactionsPerAccount,
    SubAccount { account_hash: Vec<u8> },
    TransactionById,
    TransactionMetadataById,
}

//impl BorshIntoStorageKey for StorageKey {}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)] //why do we need serialize and deserialize?
#[serde(crate = "near_sdk::serde")]
pub struct Transaction {
    //transaction ID
    pub transaction_id: TransactionId,  //should be unique and generated by the contract
    //transaction creator ID
    pub creator_id: AccountId, //should be the current account
    //transaction seller ID
    pub seller_id: AccountId, 
    //transaction buyer ID
    pub buyer_id: AccountId, 
    //transaction price
    pub price: Price, 
    //token ID
    pub nft_id: TokenId,
    //token's contract ID
    pub nft_contract_id: AccountId,
    //price amount is in the contract custody or not
    pub amount_in_escrow: bool,
    //token is in the contract custody or not
    pub token_in_escrow: bool,
    //transaction is completed or not
    pub transaction_status: TransactionStatus,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct TransactionMetadata {
    pub categories: String  //a placeholder for now, we may not need metadata at all 
}

/*******************************/
/*******AUX FUNCTIONS **********/
/*******************************/

//used to generate a unique prefix in our storage collections (this is to avoid data collisions)
//pub(crate) fn hash_account_id(account_id: &AccountId) -> CryptoHash {
//    //get the default hash
//    let mut hash = CryptoHash::default();
//    //we hash the account ID and return it
//    hash.copy_from_slice(&env::sha256(account_id.as_bytes()));
//    hash
//}

/*******************************/
/*********** CONTRACT **********/
/*******************************/

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)] // panic on default to ensure that all fields are initialized
pub struct Contract {

    //total of transactions
    pub total_transactions: u128, // should it be a bigger number?

    //contract owner, some functions could only be use by the owner
    pub owner_id: AccountId,

    //transaction fee, 2% of the price, but could be changed by the owner
    pub transaction_fee: u128,

    //keeps track of all the transactions IDs for a given account
    pub transactions_per_account: LookupMap<AccountId, UnorderedSet<TransactionId>>,

    //keeps track of the transaction struct for a given transaction ID
    pub transaction_by_id: LookupMap<TransactionId, Transaction>,

    //keeps track of the transaction metadata for a given transaction ID [info that doesnt change during transaction]
    pub transaction_metadata_by_id: UnorderedMap<TransactionId, TransactionMetadata>,
}

/*******************************/
/******* INITIALIZATION ********/
/*******************************/

#[near_bindgen]
impl Contract {
    /*
        initialization function (can only be called once).
        sets the contract owner
    */    
    #[init]
    pub fn new(owner_id: AccountId) -> Self {
        //create a variable of type Self with all the fields initialized. 
        Self {
            total_transactions: 0, //total number of transactions, used for id generation
            //set the owner_id field equal to the passed in owner_id. 
            owner_id,
            transaction_fee: 2, //2% of the price
            //Storage keys are simply the prefixes used for the collections. This helps avoid data collision
            transactions_per_account: LookupMap::new(StorageKeys::TransactionsPerAccount),
            transaction_by_id: LookupMap::new(StorageKeys::TransactionById),
            transaction_metadata_by_id: UnorderedMap::new(StorageKeys::TransactionMetadataById),
        }
    }

    pub fn add_transaction_to_user(& mut self, account_id: &AccountId, transaction_id: &TransactionId){ //make it private
        let mut transaction_set = self.transactions_per_account.get(account_id).unwrap_or_else(|| {
            UnorderedSet::new(
                StorageKeys::SubAccount { account_hash: env::sha256(account_id.as_bytes()) }
            )
        });
        transaction_set.insert(transaction_id);

        //we insert that set for the given account ID. 
        self.transactions_per_account.insert(account_id, &transaction_set);
    }

    //pub fn get_lookup_map(&self, key: &AccountId) -> UnorderedSet<TransactionId> {
    //    match self.transactions_per_account.get(key) {
    //        Some(set) => {
    //            let log_message = format!("Value from LookupMap is {:?}", value.clone());
    //            env::log(log_message.as_bytes());
    //            value
    //        },
    //        None => "not found".to_string()
    //    }
    //}

    pub fn transactions_per_account( &self, account_id: AccountId) -> U128 {

        let transaction_set = self.transactions_per_account.get(&account_id);

        if let Some(transaction_set) = transaction_set {
            U128(transaction_set.len() as u128)
        } else {
            U128(0)
        }
    }


   //TODO: CHECK ACCOUNT ID ARE VALID
   //TODO: ADD STORAGE MANAGEMENT, IT MAY BE NECESSARY TO MERGE CREATION AND TRANSFERENCE OF TOKENS
    #[payable]
    pub fn create_transaction(       //implement storage management for transactions
        &mut self,
        seller_id: AccountId,
        buyer_id: AccountId,
        price: Price,
        nft_id: TokenId,
        nft_contract_id: AccountId,
        ) -> Transaction {

        let sender = env::predecessor_account_id();

        let transaction = Transaction {
            transaction_id: self.total_transactions,  //********** should be unique and generated by the contract
            creator_id: sender.clone(),
            seller_id: seller_id.clone(),
            buyer_id: buyer_id.clone(), 
            price: price*YOCTO_NEAR,
            nft_id: nft_id.clone(),
            nft_contract_id: nft_contract_id.clone(),
            amount_in_escrow: false,
            token_in_escrow: false,
            transaction_status: TransactionStatus::Pending,
        };


        //let account_as_key = transaction.creator_id.try_to_vec().unwrap();

        // update number of transactions
        self.total_transactions += 1;

        self.add_transaction_to_user(&transaction.creator_id, &transaction.transaction_id);
        
        self.transaction_by_id.insert(&transaction.transaction_id, &transaction);
       
        transaction
    }

    pub fn get_transaction_by_id(&self, transaction_id: TransactionId) -> Transaction {
        self.transaction_by_id.get(&transaction_id).unwrap_or_else(|| {
            panic!("Transaction not found")
        })
    }

    //Check if account_id is valid, note: eliminate this function
    pub fn verify_account_id(&self, account_id: AccountId) -> bool {
        let result = env::is_valid_account_id(account_id.as_ref());
        result
    }

    //Get transaction fee
    pub fn get_transaction_fee(&self, transaction_id: TransactionId) -> u128 {
        let transaction = self.get_transaction_by_id(transaction_id);
        let price = transaction.price;
        let fee = (price/100)*self.transaction_fee;
        fee
    }

    //It return the transaction fee in yocto near
    pub fn get_price_plus_fee(&self, transaction_id: TransactionId) -> u128 {
        let transaction = self.get_transaction_by_id(transaction_id);
        let price = transaction.price;
        let fee = self.get_transaction_fee(transaction_id);
        let price_plus_fee = price + fee;
        price_plus_fee
    }

    //Set new transaction fee parameter
    pub fn set_transaction_fee(&mut self, new_transaction_fee: u128) -> u128 {

        //only the owner can change the transaction fee
        if env::predecessor_account_id() == self.owner_id {
            self.transaction_fee = new_transaction_fee;
        } else {
            panic!("Only the owner can change the transaction fee");
        }

        self.transaction_fee
    }

    //Get transaction fee parameter
    pub fn get_transaction_fee_parameter(&self) -> u128 {
        self.transaction_fee
    }

    // DONE: CHECK STATUS UPDATE [TO PENDING FOR EXAMPLE]
    // DONE: CHECK TRANSACTION ID EXIST   [I could only verify it has a valid form]
    // DONE: CHECK ONLY OWNER/SELLER CAN MAKE THE DEPOSIT
    // DONE: CHECK DEPOSIT CAN ONLY BE MADE ONCE PER TRANSACTION
    // DONE: RETURN TRANSACTION OBJECT SO UPDATED STATUS CAN BE CHECKED
    // DONE: INCLUDE TRANSACTION FEES
    // DONE: FUNCTION TO CHANGE TRANSACTION FEE, AND ONLY OWNER CAN DO THIS
    // TODO: FUNCTION TO SEND FEES TO TREASURE CONTRACT
    // TODO: FUNCION TO FREE UP TOKENS AFTER TRANSACTION
    // TODO: HOW TO STORE SELLER INFO?
    // TODO: WRITE TEST
    // TODO: NFT TRANSFER FUNCTIONS
    // TODO: AFTER HAVING WRITEN TEST, REFACTOR CODE TO BE MORE LIGHTWEIGHT AND EFICIENT, EASIER TO READ AND SECURE
    #[payable]
    pub fn transfer_to_lock(&mut self, transaction_id: TransactionId) -> Transaction {  //should be private, and only be called under some conditions
        
        let sender = env::predecessor_account_id();

        let mut transaction = self.get_transaction_by_id(transaction_id.clone());
        
        //Verify deposit is equal to price + fee
        let total_price = self.get_price_plus_fee(transaction_id);

        env::log(format!("Total price plus fee: {}", total_price).as_bytes());
        env::log(format!("Attached deposit: {}", env::attached_deposit()).as_bytes());

        assert!(env::attached_deposit() == total_price, "Not enough Nears attached to cover price and fee"); // here im convertin a float to u128. Check if it will not bring a problem of rounding or something similar

        env::log(format!("sender: {}", env::predecessor_account_id()).as_bytes());
        env::log(format!("seller: {}", transaction.seller_id).as_bytes());
        //Sender must be the creator of the transaction
        assert!(sender == transaction.seller_id, "You are not the seller of this transaction");

        //Check if transaction is pending
        assert!(transaction.transaction_status == TransactionStatus::Pending, "Transference has already been made");
        








        //env::log(format!("thanks").as_bytes()); // signer account
        env::log(b"thanks");

        // transfer Nears
        //if env::attached_deposit() >= transaction.price {

        //    env::log_str("Thanks!");
        
        //} else {
        //    panic!("Not enough Nears");
        //    }


        //Update transaction status
        transaction.transaction_status = TransactionStatus::TokensLocked;

        //Update transaction in storage
        self.transaction_by_id.insert(&transaction.transaction_id, &transaction);

        //Return transaction
        let transaction_updated = self.get_transaction_by_id(transaction_id.clone());
        transaction_updated
    }


    pub fn test(&self) {
        env::log(format!("signer_account_id: {}", env::signer_account_id()).as_bytes()); // signer account
        env::log(format!("predecessor_account_id: {}", env::predecessor_account_id()).as_bytes()); // signer account when no callback
        env::log(format!("current_account_id: {}", env::current_account_id()).as_bytes()); //contract account
        env::log(format!("attached_deposit: {}", env::attached_deposit()).as_bytes());
        env::log(format!("account_balance: {}", env::account_balance()).as_bytes()); //balance of contract account
    }







    pub fn pay(receiver_id: AccountId) -> Promise {  //should be private, and only be called under some conditions, do i need &self? as argument
        let amount = env::account_balance() - 20*YOCTO_NEAR;  // should be replace by the amount transfered
        Promise::new(receiver_id).transfer(amount)
    }



    // to check if the user has nfts in the contract given
    pub fn check_nft(account_id: AccountId) {

    let nft_number = ext_contract_::nft_supply_for_owner(
        account_id.clone(),
        &"example-nft.testnet", // contract account id 
        NO_DEPOSIT, // yocto NEAR to attach
        BASE_GAS, // gas to attach
     );
     env::log(format!("llegué hasta el final").as_bytes());

     nft_number.then(ext_self::my_callback(
        &env::current_account_id(), // this contract's account id
        0, // yocto NEAR to attach to the callback
        5_000_000_000_000 // gas to attach to the callback
    ));
    }

    //callback function
    pub fn my_callback(&self) -> String {
        assert_eq!(
            env::promise_results_count(),
            1,
            "This is a callback method"
        );

        // handle the result from the cross contract call this method is a callback for
        match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Failed => "oops!".to_string(),
            PromiseResult::Successful(result) => {
                let balance = near_sdk::serde_json::from_slice::<U128>(&result).unwrap();
                env::log(format!("llegué hasta el final {:#?}", balance.0).as_bytes()); // remove later
                if balance.0 > 0 {
                    "yes".to_string()
                } else {
                    "no".to_string()
                }
            },
        }
    }
     
}





//function to be called
#[ext_contract(ext_contract_)]
trait ExtContract {
fn nft_supply_for_owner(&self, account_id: AccountId);
}

// define methods we'll use as callbacks on our contract
#[ext_contract(ext_self)]
pub trait MyContract {
    fn my_callback(&self) -> String;
}
