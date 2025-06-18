#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod dataset_registry {
    use ink::storage::{Mapping};
    use ink::storage::traits::{SpreadLayout, PackedLayout, StorageLayout};
    use ink::prelude::vec::Vec;
    use ink::prelude::string::String;
    
    /// Dataset information structure
    #[derive(Debug, Clone, PartialEq, Eq, SpreadLayout, PackedLayout)]
    #[cfg_attr(feature = "std", derive(StorageLayout))]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    pub struct Dataset {
        pub id: u64,
        pub owner: AccountId,
        pub name: String,
        pub description: String,
        pub embedding_root: [u8; 32], // Merkle root of embeddings
        pub metadata_hash: [u8; 32],  // IPFS hash or similar
        pub price_per_query: Balance,
        pub is_active: bool,
        pub created_at: Timestamp,
        pub total_queries: u64,
        pub validator_nodes: Vec<AccountId>,
    }

    /// Dataset registry contract
    #[ink(storage)]
    pub struct DatasetRegistry {
        /// Mapping from dataset ID to dataset info
        datasets: Mapping<u64, Dataset>,
        /// Mapping from owner to their dataset IDs
        owner_datasets: Mapping<AccountId, Vec<u64>>,
        /// Next available dataset ID
        next_dataset_id: u64,
        /// Contract owner
        owner: AccountId,
        /// Registration fee
        registration_fee: Balance,
    }

    /// Events
    #[ink(event)]
    pub struct DatasetRegistered {
        #[ink(topic)]
        dataset_id: u64,
        #[ink(topic)]
        owner: AccountId,
        name: String,
        price_per_query: Balance,
    }

    #[ink(event)]
    pub struct DatasetUpdated {
        #[ink(topic)]
        dataset_id: u64,
        price_per_query: Balance,
        is_active: bool,
    }

    #[ink(event)]
    pub struct ValidatorAdded {
        #[ink(topic)]
        dataset_id: u64,
        validator: AccountId,
    }

    /// Errors
    #[derive(Debug, PartialEq, Eq)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    pub enum Error {
        DatasetNotFound,
        NotOwner,
        NotAuthorized,
        InsufficientFee,
        DatasetInactive,
        ValidatorAlreadyExists,
        InvalidParameters,
    }

    pub type Result<T> = core::result::Result<T, Error>;

    impl DatasetRegistry {
        /// Constructor
        #[ink(constructor)]
        pub fn new(registration_fee: Balance) -> Self {
            Self {
                datasets: Mapping::default(),
                owner_datasets: Mapping::default(),
                next_dataset_id: 1,
                owner: Self::env().caller(),
                registration_fee,
            }
        }

        /// Register a new dataset
        #[ink(message, payable)]
        pub fn register_dataset(
            &mut self,
            name: String,
            description: String,
            embedding_root: [u8; 32],
            metadata_hash: [u8; 32],
            price_per_query: Balance,
        ) -> Result<u64> {
            let caller = self.env().caller();
            let value = self.env().transferred_value();

            // Check registration fee
            if value < self.registration_fee {
                return Err(Error::InsufficientFee);
            }

            // Validate parameters
            if name.is_empty() || price_per_query == 0 {
                return Err(Error::InvalidParameters);
            }

            let dataset_id = self.next_dataset_id;
            let now = self.env().block_timestamp();

            let dataset = Dataset {
                id: dataset_id,
                owner: caller,
                name: name.clone(),
                description,
                embedding_root,
                metadata_hash,
                price_per_query,
                is_active: true,
                created_at: now,
                total_queries: 0,
                validator_nodes: Vec::new(),
            };

            // Store dataset
            self.datasets.insert(dataset_id, &dataset);

            // Update owner's dataset list
            let mut owner_list = self.owner_datasets.get(&caller).unwrap_or_default();
            owner_list.push(dataset_id);
            self.owner_datasets.insert(&caller, &owner_list);

            self.next_dataset_id += 1;

            // Emit event
            self.env().emit_event(DatasetRegistered {
                dataset_id,
                owner: caller,
                name,
                price_per_query,
            });

            Ok(dataset_id)
        }

        /// Update dataset parameters
        #[ink(message)]
        pub fn update_dataset(
            &mut self,
            dataset_id: u64,
            price_per_query: Option<Balance>,
            is_active: Option<bool>,
        ) -> Result<()> {
            let caller = self.env().caller();
            let mut dataset = self.datasets.get(&dataset_id).ok_or(Error::DatasetNotFound)?;

            if dataset.owner != caller {
                return Err(Error::NotOwner);
            }

            if let Some(price) = price_per_query {
                dataset.price_per_query = price;
            }

            if let Some(active) = is_active {
                dataset.is_active = active;
            }

            self.datasets.insert(dataset_id, &dataset);

            self.env().emit_event(DatasetUpdated {
                dataset_id,
                price_per_query: dataset.price_per_query,
                is_active: dataset.is_active,
            });

            Ok(())
        }

        /// Add validator node to dataset
        #[ink(message)]
        pub fn add_validator(
            &mut self,
            dataset_id: u64,
            validator: AccountId,
        ) -> Result<()> {
            let caller = self.env().caller();
            let mut dataset = self.datasets.get(&dataset_id).ok_or(Error::DatasetNotFound)?;

            if dataset.owner != caller {
                return Err(Error::NotOwner);
            }

            if dataset.validator_nodes.contains(&validator) {
                return Err(Error::ValidatorAlreadyExists);
            }

            dataset.validator_nodes.push(validator);
            self.datasets.insert(dataset_id, &dataset);

            self.env().emit_event(ValidatorAdded {
                dataset_id,
                validator,
            });

            Ok(())
        }

        /// Get dataset information
        #[ink(message)]
        pub fn get_dataset(&self, dataset_id: u64) -> Option<Dataset> {
            self.datasets.get(&dataset_id)
        }

        /// Get datasets by owner
        #[ink(message)]
        pub fn get_datasets_by_owner(&self, owner: AccountId) -> Vec<u64> {
            self.owner_datasets.get(&owner).unwrap_or_default()
        }

        /// Increment query count (called by payment contract)
        #[ink(message)]
        pub fn increment_query_count(&mut self, dataset_id: u64) -> Result<()> {
            let mut dataset = self.datasets.get(&dataset_id).ok_or(Error::DatasetNotFound)?;
            dataset.total_queries += 1;
            self.datasets.insert(dataset_id, &dataset);
            Ok(())
        }

        /// Check if dataset is active and get price
        #[ink(message)]
        pub fn get_query_price(&self, dataset_id: u64) -> Result<Balance> {
            let dataset = self.datasets.get(&dataset_id).ok_or(Error::DatasetNotFound)?;
            
            if !dataset.is_active {
                return Err(Error::DatasetInactive);
            }

            Ok(dataset.price_per_query)
        }

        /// Get validator nodes for dataset
        #[ink(message)]
        pub fn get_validators(&self, dataset_id: u64) -> Result<Vec<AccountId>> {
            let dataset = self.datasets.get(&dataset_id).ok_or(Error::DatasetNotFound)?;
            Ok(dataset.validator_nodes)
        }

        /// Admin functions
        #[ink(message)]
        pub fn set_registration_fee(&mut self, fee: Balance) -> Result<()> {
            if self.env().caller() != self.owner {
                return Err(Error::NotAuthorized);
            }
            self.registration_fee = fee;
            Ok(())
        }

        #[ink(message)]
        pub fn get_registration_fee(&self) -> Balance {
            self.registration_fee
        }
    }

    /// Unit tests
    #[cfg(test)]
    mod tests {
        use super::*;

        #[ink::test]
        fn test_register_dataset() {
            let mut contract = DatasetRegistry::new(1000);
            
            let result = contract.register_dataset(
                "Test Dataset".to_string(),
                "Description".to_string(),
                [0u8; 32],
                [1u8; 32],
                100,
            );
            
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), 1);
        }

        #[ink::test]
        fn test_get_dataset() {
            let mut contract = DatasetRegistry::new(1000);
            
            contract.register_dataset(
                "Test Dataset".to_string(),
                "Description".to_string(),
                [0u8; 32],
                [1u8; 32],
                100,
            ).unwrap();

            let dataset = contract.get_dataset(1);
            assert!(dataset.is_some());
            assert_eq!(dataset.unwrap().name, "Test Dataset");
        }
    }
}