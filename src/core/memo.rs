use std::str::FromStr;
use solana_sdk::{
    pubkey::Pubkey,
    instruction::{Instruction, AccountMeta},
    system_program,
};
use anchor_client::{
    solana_sdk::{
        commitment_config::CommitmentConfig,
        signature::{Keypair, Signer},
    },
    Client,
};
use anchor_spl::token::{spl_token, TokenAccount, Mint};
use spl_associated_token_account::{
    get_associated_token_address,
    instruction::create_associated_token_account,
};

// Constants
const RPC_URL: &str = "https://rpc.testnet.x1.xyz";
const PROGRAM_ID: &str = "68ASgTRCbbwsfgvpkfp3LvdXbpn33QbxbV64jXVaW8Ap";
const MINT_ADDRESS: &str = "EfVqRhubT8JETBdFtJsggSEnoR25MxrAoakswyir1uM4";

pub struct MemoClient {
    // RPC client
    client: Client,
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
        let client = Client::new_with_options(
            RPC_URL.parse().unwrap(),
            &payer,
            CommitmentConfig::confirmed()
        );

        Ok(Self {
            client,
            program_id,
            mint,
            payer,
        })
    }

    // Mint tokens with memo
    pub async fn mint_with_memo(&self, memo: String) -> Result<String, String> {
        // Get program instance
        let program = self.client.program(self.program_id)
            .map_err(|e| format!("Failed to get program: {}", e))?;

        // Find PDA for mint authority
        let (mint_authority_pda, _bump) = Pubkey::find_program_address(
            &[b"mint_authority"],
            &self.program_id,
        );

        // Get user's associated token account
        let token_account = get_associated_token_address(
            &self.payer.pubkey(),
            &self.mint,
        );

        let rpc = program.rpc();
        let mut tx_builder = program.request();

        // Check if token account exists, if not create it
        if rpc.get_account(&token_account).is_err() {
            let create_token_account_ix = create_associated_token_account(
                &self.payer.pubkey(),  // Payer
                &self.payer.pubkey(),  // Wallet address
                &self.mint,            // Mint
                &spl_token::id(),      // Token program ID
            );
            tx_builder = tx_builder.instruction(create_token_account_ix);
        }

        // Create mint instruction
        let accounts = vec![
            AccountMeta::new(self.payer.pubkey(), true),
            AccountMeta::new(self.mint, false),
            AccountMeta::new(mint_authority_pda, false),
            AccountMeta::new(token_account, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ];

        let data = {
            let mut d = vec![0u8]; // Instruction discriminator
            d.extend_from_slice(&(memo.len() as u32).to_le_bytes());
            d.extend_from_slice(memo.as_bytes());
            d
        };

        let mint_ix = Instruction {
            program_id: self.program_id,
            accounts,
            data,
        };

        // Build and send transaction
        let signature = tx_builder
            .instruction(mint_ix)
            .send()
            .map_err(|e| format!("Failed to send transaction: {}", e))?;

        Ok(signature.to_string())
    }

    // Get token account balance
    pub async fn get_balance(&self) -> Result<u64, String> {
        let token_account = get_associated_token_address(
            &self.payer.pubkey(),
            &self.mint,
        );

        let rpc = self.client.program(self.program_id).rpc();
        
        match rpc.get_account(&token_account) {
            Ok(account) => {
                let token_account = TokenAccount::try_deserialize(&mut &account.data[..])
                    .map_err(|e| format!("Failed to deserialize token account: {}", e))?;
                Ok(token_account.amount)
            }
            Err(_) => Ok(0),
        }
    }
}

// Helper function to create new memo client
pub fn create_memo_client(payer: Keypair) -> Result<MemoClient, String> {
    MemoClient::new(payer)
} 