//! JSON-RPC calls to X1, and the mint transaction.

use crate::network::{Network, TOKEN_2022_PROGRAM_ID};
use gloo_utils::format::JsValueSerdeExt;
use sha2::{Digest, Sha256};
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction,
    instruction::{AccountMeta, Instruction},
    message::Message,
    pubkey::Pubkey,
    transaction::Transaction,
};
use std::str::FromStr;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};

/// The memo every mint carries unless the user writes their own (memo-mint needs at least 69 bytes)
pub const DEFAULT_MEMO: &str = r#"{"action":"mint","platform":"memo-app","version":"2.0.0","chain":"X1"}"#;
pub const MIN_MEMO_BYTES: usize = 69;
pub const MAX_MEMO_BYTES: usize = 800;

pub struct Rpc {
    network: Network,
}

fn pubkey(s: &str) -> Result<Pubkey, String> {
    Pubkey::from_str(s).map_err(|e| format!("Invalid address {}: {}", s, e))
}

/// The program's own message from simulation or preflight logs ("Error Message: ..."), if any
fn program_error(logs: Option<&serde_json::Value>) -> Option<String> {
    logs?.as_array()?.iter().filter_map(|l| l.as_str()).find_map(|l| {
        l.find("Error Message: ").map(|i| l[i + 15..].trim_end_matches('.').to_string())
    })
}

impl Rpc {
    pub fn new(network: Network) -> Self {
        Self { network }
    }

    async fn call(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value, String> {
        let body = serde_json::json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params });
        let opts = RequestInit::new();
        opts.set_method("POST");
        opts.set_mode(RequestMode::Cors);
        opts.set_body(&JsValue::from_str(&body.to_string()));
        let request = Request::new_with_str_and_init(self.network.rpc_url(), &opts)
            .map_err(|e| format!("{:?}", e))?;
        request.headers().set("Content-Type", "application/json").map_err(|e| format!("{:?}", e))?;

        let window = web_sys::window().ok_or("No window")?;
        let resp: Response = JsFuture::from(window.fetch_with_request(&request))
            .await
            .map_err(|_| "Network request failed".to_string())?
            .dyn_into()
            .map_err(|_| "Bad response".to_string())?;
        if !resp.ok() {
            return Err(format!("RPC HTTP {}", resp.status()));
        }
        let json = JsFuture::from(resp.json().map_err(|e| format!("{:?}", e))?)
            .await
            .map_err(|e| format!("{:?}", e))?;
        let value: serde_json::Value = json.into_serde().map_err(|e| e.to_string())?;

        if let Some(err) = value.get("error") {
            let logs = err.get("data").and_then(|d| d.get("logs"));
            let message = err.get("message").and_then(|m| m.as_str()).unwrap_or("RPC error");
            return Err(program_error(logs).unwrap_or_else(|| message.to_string()));
        }
        value.get("result").cloned().ok_or_else(|| "RPC response has no result".to_string())
    }

    /// Total MEMO supply in units (6 decimals)
    pub async fn token_supply(&self) -> Result<u64, String> {
        let r = self
            .call("getTokenSupply", serde_json::json!([self.network.token_mint(), { "commitment": "confirmed" }]))
            .await?;
        r["value"]["amount"].as_str().and_then(|a| a.parse().ok()).ok_or_else(|| "Bad supply".to_string())
    }

    /// Block times (unix seconds) of recent transactions that call memo-mint, i.e. mints (burns also
    /// touch the token mint, so it is not used here), newest first. Pages back until `since` is covered or `max_pages` pages of 1,000 are read.
    pub async fn recent_tx_times(&self, since: i64, max_pages: usize) -> Result<Vec<i64>, String> {
        let mut times = Vec::new();
        let mut before: Option<String> = None;
        for _ in 0..max_pages {
            let mut opts = serde_json::json!({ "limit": 1000, "commitment": "confirmed" });
            if let Some(b) = &before {
                opts["before"] = serde_json::json!(b);
            }
            let page = self.call("getSignaturesForAddress", serde_json::json!([self.network.mint_program_id(), opts])).await?;
            let page = page.as_array().cloned().unwrap_or_default();
            times.extend(page.iter().filter_map(|s| s["blockTime"].as_i64()));
            match page.last() {
                Some(last) if page.len() == 1000 && last["blockTime"].as_i64().is_some_and(|t| t >= since) => {
                    before = last["signature"].as_str().map(str::to_string);
                }
                _ => break,
            }
        }
        Ok(times)
    }

    /// XNT balance in lamports
    pub async fn xnt_balance(&self, owner: &str) -> Result<u64, String> {
        let r = self.call("getBalance", serde_json::json!([owner, { "commitment": "confirmed" }])).await?;
        r["value"].as_u64().ok_or_else(|| "Bad balance".to_string())
    }

    fn token_account(&self, owner: &Pubkey) -> Result<Pubkey, String> {
        Ok(spl_associated_token_account::get_associated_token_address_with_program_id(
            owner,
            &pubkey(self.network.token_mint())?,
            &pubkey(TOKEN_2022_PROGRAM_ID)?,
        ))
    }

    /// MEMO balance in units; 0 if the wallet has no MEMO account yet
    pub async fn memo_balance(&self, owner: &str) -> Result<u64, String> {
        let account = self.token_account(&pubkey(owner)?)?;
        let r = self
            .call("getAccountInfo", serde_json::json!([account.to_string(), { "encoding": "jsonParsed", "commitment": "confirmed" }]))
            .await?;
        if r["value"].is_null() {
            return Ok(0);
        }
        r["value"]["data"]["parsed"]["info"]["tokenAmount"]["amount"]
            .as_str()
            .and_then(|a| a.parse().ok())
            .ok_or_else(|| "Bad token account".to_string())
    }

    /// An unsigned mint transaction: memo (index 0), create the MEMO account if missing, process_mint,
    /// then the compute unit limit from a simulation
    pub async fn build_mint_transaction(&self, owner: &str, memo: &str) -> Result<Transaction, String> {
        let user = pubkey(owner)?;
        let program = pubkey(self.network.mint_program_id())?;
        let mint = pubkey(self.network.token_mint())?;
        let token_program = pubkey(TOKEN_2022_PROGRAM_ID)?;
        let token_account = self.token_account(&user)?;
        let (mint_authority, _) = Pubkey::find_program_address(&[b"mint_authority"], &program);
        let (mint_state, _) = Pubkey::find_program_address(&[b"mint_state"], &program);

        let mut instructions = vec![spl_memo::build_memo(memo.as_bytes(), &[&user])];
        instructions.push(spl_associated_token_account::instruction::create_associated_token_account_idempotent(
            &user, &user, &mint, &token_program,
        ));
        let discriminator = Sha256::digest(b"global:process_mint");
        instructions.push(Instruction::new_with_bytes(
            program,
            &discriminator[..8],
            vec![
                AccountMeta::new(user, true),
                AccountMeta::new(mint, false),
                AccountMeta::new_readonly(mint_authority, false),
                AccountMeta::new(token_account, false),
                AccountMeta::new_readonly(token_program, false),
                AccountMeta::new_readonly(solana_sdk::sysvar::instructions::id(), false),
                AccountMeta::new(mint_state, false),
            ],
        ));

        let blockhash = self.call("getLatestBlockhash", serde_json::json!([{ "commitment": "confirmed" }])).await?;
        let blockhash = blockhash["value"]["blockhash"]
            .as_str()
            .and_then(|b| solana_sdk::hash::Hash::from_str(b).ok())
            .ok_or_else(|| "Bad blockhash".to_string())?;

        // Simulate with a generous limit to learn the real compute units; the fee follows the limit we request
        let mut sim = instructions.clone();
        sim.push(ComputeBudgetInstruction::set_compute_unit_limit(400_000));
        let mut sim_tx = Transaction::new_unsigned(Message::new(&sim, Some(&user)));
        sim_tx.message.recent_blockhash = blockhash;
        let sim_b64 = base64::encode(bincode::serialize(&sim_tx).map_err(|e| e.to_string())?);
        let r = self
            .call(
                "simulateTransaction",
                serde_json::json!([sim_b64, { "encoding": "base64", "sigVerify": false, "replaceRecentBlockhash": true, "commitment": "confirmed" }]),
            )
            .await?;
        if !r["value"]["err"].is_null() {
            return Err(program_error(r["value"].get("logs"))
                .unwrap_or_else(|| format!("Simulation failed: {}", r["value"]["err"])));
        }
        let units = r["value"]["unitsConsumed"].as_u64().ok_or("Simulation returned no compute units")?;

        // Memo stays at index 0: the compute budget goes last
        instructions.push(ComputeBudgetInstruction::set_compute_unit_limit((units + units / 10 + 1_000) as u32));
        let mut tx = Transaction::new_unsigned(Message::new(&instructions, Some(&user)));
        tx.message.recent_blockhash = blockhash;
        Ok(tx)
    }

    /// Send a signed transaction (base64) and return its signature
    pub async fn send(&self, signed_b64: &str) -> Result<String, String> {
        let r = self
            .call("sendTransaction", serde_json::json!([signed_b64, { "encoding": "base64", "preflightCommitment": "confirmed" }]))
            .await?;
        r.as_str().map(str::to_string).ok_or_else(|| "Bad signature".to_string())
    }

    /// Wait up to ~40 s for the transaction to be confirmed
    pub async fn confirm(&self, signature: &str) -> Result<(), String> {
        for _ in 0..40 {
            gloo_timers::future::TimeoutFuture::new(1_000).await;
            let r = self.call("getSignatureStatuses", serde_json::json!([[signature]])).await?;
            let status = &r["value"][0];
            if status.is_null() {
                continue;
            }
            if !status["err"].is_null() {
                return Err(format!("Transaction failed: {}", status["err"]));
            }
            if matches!(status["confirmationStatus"].as_str(), Some("confirmed" | "finalized")) {
                return Ok(());
            }
        }
        Err("Not confirmed yet. Check the explorer.".to_string())
    }
}
