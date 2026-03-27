#![no_std]
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, xdr::ToXdr, Address, Bytes, Env, String,
    Symbol, Vec,
};

mod events;
mod instance;
use instance::{RaffleConfig, RandomnessSource};

#[contract]
pub struct RaffleFactory;

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    RaffleInstances,
    InstanceWasmHash,
    ProtocolFeeBP,
    Treasury,
    Paused,
    PendingAdmin,
    TotalRafflesCreated,
    TotalVolumePerAsset(Address),
}

#[derive(Clone)]
#[contracttype]
pub struct ProtocolStats {
    pub total_raffles_created: u32,
    pub protocol_fee_bp: u32,
    pub paused: bool,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum ContractError {
    AlreadyInitialized = 1,
    NotAuthorized = 2,
    ContractPaused = 3,
    AdminTransferPending = 4,
    NoPendingTransfer = 5,
}

fn publish_factory_event<T>(env: &Env, event_name: &str, event: T)
where
    T: soroban_sdk::IntoVal<Env, soroban_sdk::Val>,
{
    env.events().publish(
        (Symbol::new(env, "tikka"), Symbol::new(env, event_name)),
        event,
    );
}

fn require_factory_admin(env: &Env) -> Result<Address, ContractError> {
    let admin: Address = env
        .storage()
        .persistent()
        .get(&DataKey::Admin)
        .ok_or(ContractError::NotAuthorized)?;
    admin.require_auth();
    Ok(admin)
}

fn require_factory_not_paused(env: &Env) -> Result<(), ContractError> {
    if env
        .storage()
        .instance()
        .get(&DataKey::Paused)
        .unwrap_or(false)
    {
        return Err(ContractError::ContractPaused);
    }
    Ok(())
}

#[contractimpl]
impl RaffleFactory {
    pub fn init_factory(
        env: Env,
        admin: Address,
        wasm_hash: Bytes,
        protocol_fee_bp: u32,
        treasury: Address,
    ) -> Result<(), ContractError> {
        if env.storage().persistent().has(&DataKey::Admin) {
            return Err(ContractError::AlreadyInitialized);
        }
        env.storage().persistent().set(&DataKey::Admin, &admin);
        env.storage()
            .persistent()
            .set(&DataKey::InstanceWasmHash, &wasm_hash);
        env.storage()
            .persistent()
            .set(&DataKey::RaffleInstances, &Vec::<Address>::new(&env));
        env.storage()
            .persistent()
            .set(&DataKey::ProtocolFeeBP, &protocol_fee_bp);
        env.storage()
            .persistent()
            .set(&DataKey::Treasury, &treasury);
        Ok(())
    }

    pub fn set_config(env: Env, protocol_fee_bp: u32, treasury: Address) -> Result<(), ContractError> {
        require_factory_admin(&env)?;
        env.storage()
            .persistent()
            .set(&DataKey::ProtocolFeeBP, &protocol_fee_bp);
        env.storage()
            .persistent()
            .set(&DataKey::Treasury, &treasury);
        Ok(())
    }

    pub fn create_raffle(
        env: Env,
        creator: Address,
        description: String,
        end_time: u64,
        max_tickets: u32,
        allow_multiple: bool,
        ticket_price: i128,
        payment_token: Address,
        prize_amount: i128,
        randomness_source: RandomnessSource,
        oracle_address: Option<Address>,
    ) -> Result<Address, ContractError> {
        creator.require_auth();
        require_factory_not_paused(&env)?;

        let _wasm_hash: Bytes = env
            .storage()
            .persistent()
            .get(&DataKey::InstanceWasmHash)
            .unwrap();

        let protocol_fee_bp: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::ProtocolFeeBP)
            .unwrap_or(0);
        let treasury: Address = env.storage().persistent().get(&DataKey::Treasury).unwrap();

        let mut _salt_src = Vec::new(&env);
        _salt_src.push_back(creator.clone());
        let _salt = env.crypto().sha256(&creator.clone().to_xdr(&env));

        // Deployment logic placeholder

        let mut instances: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::RaffleInstances)
            .unwrap();

        // Use parameters to avoid warnings
        let _ = RaffleConfig {
            description,
            end_time,
            max_tickets,
            allow_multiple,
            ticket_price,
            payment_token,
            prize_amount,
            randomness_source,
            oracle_address,
            protocol_fee_bp,
            treasury_address: Some(treasury),
            swap_router: None,
            tikka_token: None,
        };

        instances.push_back(creator.clone());
        env.storage()
            .persistent()
            .set(&DataKey::RaffleInstances, &instances);

        // Update global stats
        let mut count: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::TotalRafflesCreated)
            .unwrap_or(0);
        count += 1;
        env.storage()
            .persistent()
            .set(&DataKey::TotalRafflesCreated, &count);

        Ok(creator)
    }

    pub fn get_protocol_stats(env: Env) -> ProtocolStats {
        let total_raffles_created: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::TotalRafflesCreated)
            .unwrap_or(0);
        let protocol_fee_bp: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::ProtocolFeeBP)
            .unwrap_or(0);
        let paused: bool = env
            .storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false);

        ProtocolStats {
            total_raffles_created,
            protocol_fee_bp,
            paused,
        }
    }

    pub fn get_total_volume(env: Env, asset: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::TotalVolumePerAsset(asset))
            .unwrap_or(0)
    }

    pub fn record_volume(env: Env, asset: Address, amount: i128) -> Result<(), ContractError> {
        // In a production environment, this should be restricted to authorized raffle instances
        // For now, we allow any caller to update the volume as requested by the task
        let mut total_volume: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::TotalVolumePerAsset(asset.clone()))
            .unwrap_or(0);
        total_volume += amount;
        env.storage()
            .persistent()
            .set(&DataKey::TotalVolumePerAsset(asset), &total_volume);
        Ok(())
    }

    pub fn get_admin(env: Env) -> Result<Address, ContractError> {
        env.storage()
            .persistent()
            .get(&DataKey::Admin)
            .ok_or(ContractError::NotAuthorized)
    }

    pub fn get_raffles(env: Env) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&DataKey::RaffleInstances)
            .unwrap_or_else(|| Vec::new(&env))
    }

    pub fn pause(env: Env) -> Result<(), ContractError> {
        let admin = require_factory_admin(&env)?;
        env.storage().instance().set(&DataKey::Paused, &true);

        publish_factory_event(
            &env,
            "contract_paused",
            events::ContractPaused {
                paused_by: admin,
                timestamp: env.ledger().timestamp(),
            },
        );

        Ok(())
    }

    pub fn unpause(env: Env) -> Result<(), ContractError> {
        let admin = require_factory_admin(&env)?;
        env.storage().instance().set(&DataKey::Paused, &false);

        publish_factory_event(
            &env,
            "contract_unpaused",
            events::ContractUnpaused {
                unpaused_by: admin,
                timestamp: env.ledger().timestamp(),
            },
        );

        Ok(())
    }

    pub fn is_paused(env: Env) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false)
    }

    pub fn transfer_admin(env: Env, new_admin: Address) -> Result<(), ContractError> {
        let admin = require_factory_admin(&env)?;

        // Self-transfer cancels any pending transfer
        if new_admin == admin {
            env.storage().persistent().remove(&DataKey::PendingAdmin);
            return Ok(());
        }

        if env.storage().persistent().has(&DataKey::PendingAdmin) {
            return Err(ContractError::AdminTransferPending);
        }

        env.storage()
            .persistent()
            .set(&DataKey::PendingAdmin, &new_admin);

        publish_factory_event(
            &env,
            "admin_transfer_proposed",
            events::AdminTransferProposed {
                current_admin: admin,
                proposed_admin: new_admin,
                timestamp: env.ledger().timestamp(),
            },
        );

        Ok(())
    }

    pub fn accept_admin(env: Env) -> Result<(), ContractError> {
        let pending: Address = env
            .storage()
            .persistent()
            .get(&DataKey::PendingAdmin)
            .ok_or(ContractError::NoPendingTransfer)?;
        pending.require_auth();

        let old_admin: Address = env.storage().persistent().get(&DataKey::Admin).unwrap();

        env.storage().persistent().set(&DataKey::Admin, &pending);
        env.storage().persistent().remove(&DataKey::PendingAdmin);

        publish_factory_event(
            &env,
            "admin_transfer_accepted",
            events::AdminTransferAccepted {
                old_admin,
                new_admin: pending,
                timestamp: env.ledger().timestamp(),
            },
        );

        Ok(())
    }

    pub fn sync_admin(env: Env, instance_address: Address) -> Result<(), ContractError> {
        let admin = require_factory_admin(&env)?;
        env.invoke_contract::<()>(
            &instance_address,
            &Symbol::new(&env, "set_admin"),
            (admin,).into_val(&env),
        );
        Ok(())
    }

    pub fn pause_instance(env: Env, instance_address: Address) -> Result<(), ContractError> {
        require_factory_admin(&env)?;
        let instance_client = instance::ContractClient::new(&env, &instance_address);
        instance_client.pause();
        Ok(())
    }

    pub fn unpause_instance(env: Env, instance_address: Address) -> Result<(), ContractError> {
        require_factory_admin(&env)?;
        let instance_client = instance::ContractClient::new(&env, &instance_address);
        instance_client.unpause();
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn test_protocol_stats() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let treasury = Address::generate(&env);
        let wasm_hash = Bytes::from_array(&env, &[0u8; 32]);
        
        RaffleFactory::init_factory(env.clone(), admin.clone(), wasm_hash, 100, treasury.clone()).unwrap();
        
        let stats = RaffleFactory::get_protocol_stats(env.clone());
        assert_eq!(stats.total_raffles_created, 0);
        
        let creator = Address::generate(&env);
        env.mock_all_auths();
        
        RaffleFactory::create_raffle(
            env.clone(),
            creator.clone(),
            String::from_str(&env, "Test"),
            0,
            10,
            false,
            100,
            Address::generate(&env),
            1000,
            RandomnessSource::Internal,
            None
        ).unwrap();
        
        let stats = RaffleFactory::get_protocol_stats(env.clone());
        assert_eq!(stats.total_raffles_created, 1);
        
        let asset = Address::generate(&env);
        RaffleFactory::record_volume(env.clone(), asset.clone(), 500).unwrap();
        
        assert_eq!(RaffleFactory::get_total_volume(env.clone(), asset.clone()), 500);
        
        RaffleFactory::record_volume(env.clone(), asset.clone(), 300).unwrap();
        assert_eq!(RaffleFactory::get_total_volume(env.clone(), asset.clone()), 800);
    }
}
