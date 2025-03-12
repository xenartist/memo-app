use std::str::FromStr;
use solana_sdk::{
    pubkey::Pubkey,
    instruction::{Instruction, AccountMeta},
    signature::{Keypair, Signer},
    commitment_config::CommitmentConfig,
    transaction::Transaction,
    compute_budget::ComputeBudgetInstruction,
};
use solana_client::rpc_client::RpcClient;
use sha2::{Sha256, Digest};
use spl_associated_token_account::get_associated_token_address;

// Constants
const RPC_URL: &str = "https://rpc.testnet.x1.xyz";
const PROGRAM_ID: &str = "TD8dwXKKg7M3QpWa9mQQpcvzaRasDU1MjmQWqZ9UZiw";
const MINT_ADDRESS: &str = "CrfhYtP7XtqFyHTWMyXp25CCzhjhzojngrPCZJ7RarUz";

pub struct MemoClient {
    // RPC client
    client: RpcClient,
    // Program ID
    program_id: Pubkey,
    // Token mint address
    mint: Pubkey,
    // Payer's keypair
    payer: Keypair,
}

impl MemoClient {
    pub fn new(payer: Keypair) -> Result<Self, String> {
        // Parse program ID and mint address
        let program_id = Pubkey::from_str(PROGRAM_ID)
            .map_err(|e| format!("Invalid program ID: {}", e))?;
        let mint = Pubkey::from_str(MINT_ADDRESS)
            .map_err(|e| format!("Invalid mint address: {}", e))?;

        // Create RPC client
        let client = RpcClient::new_with_commitment(
            RPC_URL.to_string(),
            CommitmentConfig::confirmed(),
        );

        Ok(Self {
            client,
            program_id,
            mint,
            payer,
        })
    }

    // Determine required compute units based on memo length
    fn get_required_compute_units(&self, memo_length: usize) -> u32 {
        // Based on the ranges in mint-test.rs
        match memo_length {
            0..=100 => 120_000,
            101..=200 => 160_000,
            201..=300 => 200_000,
            301..=400 => 250_000,
            401..=500 => 300_000,
            501..=600 => 350_000,
            601..=700 => 400_000,
            _ => 450_000, // Default for larger memos
        }
    }

    // Mint tokens with memo
    pub async fn mint_with_memo(&self, memo: String) -> Result<String, String> {
        // Calculate PDA for mint authority
        let (mint_authority_pda, _bump) = Pubkey::find_program_address(
            &[b"mint_authority"],
            &self.program_id,
        );

        // Get user's token account
        let token_account = get_associated_token_address(
            &self.payer.pubkey(),
            &self.mint,
        );

        // Check if token account exists, if not create it
        let mut instructions = vec![];
        
        match self.client.get_account(&token_account) {
            Ok(_) => {
                println!("Token account already exists");
            },
            Err(_) => {
                println!("Creating token account...");
                // Create token account instruction
                let create_token_account_ix = 
                    spl_associated_token_account::instruction::create_associated_token_account(
                        &self.payer.pubkey(),
                        &self.payer.pubkey(),
                        &self.mint,
                        &spl_token::id(),
                    );
                instructions.push(create_token_account_ix);
            }
        }

        // Calculate required compute units based on memo length
        let compute_units = self.get_required_compute_units(memo.len());
        println!("Setting compute units to {} for memo length {}", compute_units, memo.len());
        
        // Create compute budget instruction to request specific CU amount
        let compute_budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(compute_units);
        instructions.push(compute_budget_ix);

        // Create memo instruction
        let memo_ix = spl_memo::build_memo(
            memo.as_bytes(),
            &[&self.payer.pubkey()],
        );
        instructions.push(memo_ix);

        // Calculate Anchor instruction sighash
        let mut hasher = Sha256::new();
        hasher.update(b"global:process_transfer");
        let result = hasher.finalize();
        let instruction_data = result[..8].to_vec();

        // Create mint instruction
        let mint_ix = Instruction::new_with_bytes(
            self.program_id,
            &instruction_data,
            vec![
                AccountMeta::new(self.payer.pubkey(), true),         // user
                AccountMeta::new(self.mint, false),                  // mint
                AccountMeta::new(mint_authority_pda, false),         // mint_authority (PDA)
                AccountMeta::new(token_account, false),              // token_account
                AccountMeta::new_readonly(spl_token::id(), false),   // token_program
                AccountMeta::new_readonly(solana_sdk::sysvar::instructions::id(), false), // instructions sysvar
            ],
        );
        instructions.push(mint_ix);

        // Build and send transaction
        let recent_blockhash = self.client.get_latest_blockhash()
            .map_err(|e| format!("Failed to get recent blockhash: {}", e))?;
            
        let transaction = Transaction::new_signed_with_payer(
            &instructions,
            Some(&self.payer.pubkey()),
            &[&self.payer],
            recent_blockhash,
        );
        
        let signature = self.client.send_and_confirm_transaction(&transaction)
            .map_err(|e| format!("Failed to send transaction: {}", e))?;

        // Print token balance
        match self.client.get_token_account_balance(&token_account) {
            Ok(balance) => {
                println!("New token balance: {}", balance.ui_amount.unwrap_or(0.0));
            }
            Err(_) => {
                println!("Failed to get token balance");
            }
        }

        Ok(signature.to_string())
    }

    #[allow(dead_code)]
    pub fn get_balance(&self) -> Result<f64, String> {
        let token_account = get_associated_token_address(
            &self.payer.pubkey(),
            &self.mint,
        );

        match self.client.get_token_account_balance(&token_account) {
            Ok(balance) => {
                Ok(balance.ui_amount.unwrap_or(0.0))
            }
            Err(e) => {
                Err(format!("Failed to get token balance: {}", e))
            }
        }
    }

    #[allow(dead_code)]
    pub fn get_account_info(&self) -> String {
        let token_account = get_associated_token_address(
            &self.payer.pubkey(),
            &self.mint,
        );

        let (mint_authority_pda, _) = Pubkey::find_program_address(
            &[b"mint_authority"],
            &self.program_id,
        );

        format!(
            "Account Info:\nProgram ID: {}\nMint: {}\nMint Authority (PDA): {}\nWallet: {}\nToken Account: {}",
            self.program_id,
            self.mint,
            mint_authority_pda,
            self.payer.pubkey(),
            token_account
        )
    }

    pub fn get_balance_for_address(&self, wallet_address: &str) -> Result<f64, String> {
        use solana_sdk::pubkey::Pubkey;
        use std::str::FromStr;
        
        // get wallet pubkey
        let wallet_pubkey = match Pubkey::from_str(wallet_address) {
            Ok(pubkey) => pubkey,
            Err(e) => return Err(format!("Invalid wallet address: {}", e)),
        };
        
        // get associated token address
        let token_account = get_associated_token_address(
            &wallet_pubkey,
            &self.mint,
        );
        
        // query token account balance
        match self.client.get_token_account_balance(&token_account) {
            Ok(balance) => Ok(balance.ui_amount.unwrap_or(0.0)),
            Err(e) => Err(format!("Failed to get token balance: {}", e)),
        }
    }
}

// Helper function to create new memo client
pub fn create_memo_client(payer: Keypair) -> Result<MemoClient, String> {
    MemoClient::new(payer)
}

// helper function to get token balance for address
pub fn get_token_balance_for_address(wallet_address: &str) -> Result<f64, String> {
    // create a temporary keypair to initialize the client
    let dummy_keypair = solana_sdk::signer::keypair::Keypair::new();
    
    // create memo client
    let client = create_memo_client(dummy_keypair)?;
    
    // get token balance for address
    client.get_balance_for_address(wallet_address)
} 