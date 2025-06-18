#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod payment_manager {
    use ink::storage::Mapping;
    use ink::storage::traits::{SpreadLayout, PackedLayout, StorageLayout};
    use ink::prelude::vec::Vec;
    use ink::prelude::string::String;
    
    /// Query payment information
    #[derive(Debug, Clone, PartialEq, Eq, SpreadLayout, PackedLayout)]
    #[cfg_attr(feature = "std", derive(StorageLayout))]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    pub struct Payment {
        pub query_id: u64,
        pub dataset_id: u64,
        pub user: AccountId,
        pub amount: Balance,
        pub timestamp: Timestamp,
        pub status: PaymentStatus,
        pub proof_hash: Option<[u8; 32]>,
    }

    /// Payment status
    #[derive(Debug, Clone, PartialEq, Eq, SpreadLayout, PackedLayout)]
    #[cfg_attr(feature = "std", derive(StorageLayout))]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    pub enum PaymentStatus {
        Pending,
        Completed,
        Disputed,
        Refunded,
    }

    /// Escrow information
    #[derive(Debug, Clone, PartialEq, Eq, SpreadLayout, PackedLayout)]
    #[cfg_attr(feature = "std", derive(StorageLayout))]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    pub struct Escrow {
        pub user: AccountId,
        pub dataset_owner: AccountId,
        pub amount: Balance,
        pub created_at: Timestamp,
        pub release_time: Timestamp,
    }

    /// Payment manager contract
    #[ink(storage)]
    pub struct PaymentManager {
        /// Dataset registry contract address
        dataset_registry: AccountId,
        /// ZK verifier contract address  
        zk_verifier: AccountId,
        /// Mapping from query ID to payment info
        payments: Mapping<u64, Payment>,
        /// Mapping from user to their payments
        user_payments: Mapping<AccountId, Vec<u64>>,
        /// Escrow storage
        escrows: Mapping<u64, Escrow>,
        /// Next query ID
        next_query_id: u64,
        /// Contract owner
        owner: AccountId,
        /// Platform fee percentage (basis points, e.g., 250 = 2.5%)
        platform_fee_bps: u16,
        /// Escrow period in milliseconds
        escrow_period: u64,
    }

    /// Events
    #[ink(event)]
    pub struct PaymentCreated {
        #[ink(topic)]
        query_id: u64,
        #[ink(topic)]
        user: AccountId,
        #[ink(topic)]
        dataset_id: u64,
        amount: Balance,
    }

    #[ink(event)]
    pub struct PaymentCompleted {
        #[ink(topic)]
        query_id: u64,
        proof_hash: [u8; 32],
    }

    #[ink(event)]
    pub struct PaymentRefunded {
        #[ink(topic)]
        query_id: u64,
        #[ink(topic)]
        user: AccountId,
        amount: Balance,
    }

    #[ink(event)]
    pub struct EscrowReleased {
        #[ink(topic)]
        query_id: u64,
        #[ink(topic)]
        dataset_owner: AccountId,
        amount: Balance,
    }

    /// Errors
    #[derive(Debug, PartialEq, Eq)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    pub enum Error {
        PaymentNotFound,
        InsufficientPayment,
        PaymentAlreadyCompleted,
        NotAuthorized,
        EscrowNotReady,
        TransferFailed,
        InvalidProof,
        DatasetNotFound,
    }

    pub type Result<T> = core::result::Result<T, Error>;

    impl PaymentManager {
        /// Constructor
        #[ink(constructor)]
        pub fn new(
            dataset_registry: AccountId,
            zk_verifier: AccountId,
            platform_fee_bps: u16,
            escrow_period: u64,
        ) -> Self {
            Self {
                dataset_registry,
                zk_verifier,
                payments: Mapping::default(),
                user_payments: Mapping::default(),
                escrows: Mapping::default(),
                next_query_id: 1,
                owner: Self::env().caller(),
                platform_fee_bps,
                escrow_period,
            }
        }

        /// Create payment for query
        #[ink(message, payable)]
        pub fn create_payment(&mut self, dataset_id: u64) -> Result<u64> {
            let caller = self.env().caller();
            let value = self.env().transferred_value();
            let now = self.env().block_timestamp();

            // Get dataset price from registry
            let price = self.get_dataset_price(dataset_id)?;
            
            if value < price {
                return Err(Error::InsufficientPayment);
            }

            let query_id = self.next_query_id;
            
            let payment = Payment {
                query_id,
                dataset_id,
                user: caller,
                amount: price,
                timestamp: now,
                status: PaymentStatus::Pending,
                proof_hash: None,
            };

            // Store payment
            self.payments.insert(query_id, &payment);

            // Update user payments list
            let mut user_list = self.user_payments.get(&caller).unwrap_or_default();
            user_list.push(query_id);
            self.user_payments.insert(&caller, &user_list);

            // Get dataset owner
            let dataset_owner = self.get_dataset_owner(dataset_id)?;

            // Create escrow
            let escrow = Escrow {
                user: caller,
                dataset_owner,
                amount: price,
                created_at: now,
                release_time: now + self.escrow_period,
            };
            self.escrows.insert(query_id, &escrow);

            self.next_query_id += 1;

            // Refund excess payment
            if value > price {
                let excess = value - price;
                self.env().transfer(caller, excess).map_err(|_| Error::TransferFailed)?;
            }

            self.env().emit_event(PaymentCreated {
                query_id,
                user: caller,
                dataset_id,
                amount: price,
            });

            Ok(query_id)
        }

        /// Complete payment with proof
        #[ink(message)]
        pub fn complete_payment(
            &mut self,
            query_id: u64,
            proof_hash: [u8; 32],
        ) -> Result<()> {
            let caller = self.env().caller();
            
            // Only ZK verifier can complete payments
            if caller != self.zk_verifier {
                return Err(Error::NotAuthorized);
            }

            let mut payment = self.payments.get(&query_id).ok_or(Error::PaymentNotFound)?;
            
            if payment.status != PaymentStatus::Pending {
                return Err(Error::PaymentAlreadyCompleted);
            }

            payment.status = PaymentStatus::Completed;
            payment.proof_hash = Some(proof_hash);
            self.payments.insert(query_id, &payment);

            self.env().emit_event(PaymentCompleted {
                query_id,
                proof_hash,
            });

            Ok(())
        }

        /// Release escrow to dataset owner
        #[ink(message)]
        pub fn release_escrow(&mut self, query_id: u64) -> Result<()> {
            let now = self.env().block_timestamp();
            let escrow = self.escrows.get(&query_id).ok_or(Error::PaymentNotFound)?;
            let payment = self.payments.get(&query_id).ok_or(Error::PaymentNotFound)?;

            // Check if payment is completed and escrow period has passed
            if payment.status != PaymentStatus::Completed || now < escrow.release_time {
                return Err(Error::EscrowNotReady);
            }

            // Calculate platform fee
            let platform_fee = (escrow.amount * self.platform_fee_bps as u128) / 10000;
            let owner_amount = escrow.amount - platform_fee;

            // Transfer to dataset owner
            self.env().transfer(escrow.dataset_owner, owner_amount)
                .map_err(|_| Error::TransferFailed)?;

            // Transfer platform fee to contract owner
            if platform_fee > 0 {
                self.env().transfer(self.owner, platform_fee)
                    .map_err(|_| Error::TransferFailed)?;
            }

            // Remove escrow
            self.escrows.remove(&query_id);

            self.env().emit_event(EscrowReleased {
                query_id,
                dataset_owner: escrow.dataset_owner,
                amount: owner_amount,
            });

            Ok(())
        }

        /// Refund payment (only for disputes or failed proofs)
        #[ink(message)]
        pub fn refund_payment(&mut self, query_id: u64) -> Result<()> {
            let caller = self.env().caller();
            
            // Only contract owner can initiate refunds
            if caller != self.owner {
                return Err(Error::NotAuthorized);
            }

            let mut payment = self.payments.get(&query_id).ok_or(Error::PaymentNotFound)?;
            let escrow = self.escrows.get(&query_id).ok_or(Error::PaymentNotFound)?;

            if payment.status == PaymentStatus::Completed {
                return Err(Error::PaymentAlreadyCompleted);
            }

            payment.status = PaymentStatus::Refunded;
            self.payments.insert(query_id, &payment);

            // Refund to user
            self.env().transfer(escrow.user, escrow.amount)
                .map_err(|_| Error::TransferFailed)?;

            // Remove escrow
            self.escrows.remove(&query_id);

            self.env().emit_event(PaymentRefunded {
                query_id,
                user: escrow.user,
                amount: escrow.amount,
            });

            Ok(())
        }

        /// Get payment information
        #[ink(message)]
        pub fn get_payment(&self, query_id: u64) -> Option<Payment> {
            self.payments.get(&query_id)
        }

        /// Get user payments
        #[ink(message)]
        pub fn get_user_payments(&self, user: AccountId) -> Vec<u64> {
            self.user_payments.get(&user).unwrap_or_default()
        }

        /// Get escrow information
        #[ink(message)]
        pub fn get_escrow(&self, query_id: u64) -> Option<Escrow> {
            self.escrows.get(&query_id)
        }

        /// Admin functions
        #[ink(message)]
        pub fn set_platform_fee(&mut self, fee_bps: u16) -> Result<()> {
            if self.env().caller() != self.owner {
                return Err(Error::NotAuthorized);
            }
            self.platform_fee_bps = fee_bps;
            Ok(())
        }

        #[ink(message)]
        pub fn set_escrow_period(&mut self, period: u64) -> Result<()> {
            if self.env().caller() != self.owner {
                return Err(Error::NotAuthorized);
            }
            self.escrow_period = period;
            Ok(())
        }

        // Helper functions (would typically be cross-contract calls)
        fn get_dataset_price(&self, dataset_id: u64) -> Result<Balance> {
            // This would be a cross-contract call to dataset registry
            // For now, return a mock price
            Ok(1000)
        }

        fn get_dataset_owner(&self, dataset_id: u64) -> Result<AccountId> {
            // This would be a cross-contract call to dataset registry
            // For now, return contract owner
            Ok(self.owner)
        }
    }

    /// Unit tests
    #[cfg(test)]
    mod tests {
        use super::*;

        #[ink::test]
        fn test_create_payment() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            let mut contract = PaymentManager::new(
                accounts.alice,
                accounts.bob,
                250, // 2.5% platform fee
                86400000, // 24 hours escrow
            );

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.charlie);
            ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(1000);

            let result = contract.create_payment(1);
            assert!(result.is_ok());
        }
    }
}