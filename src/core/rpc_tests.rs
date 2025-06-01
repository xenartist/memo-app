#[cfg(test)]
mod tests {
    use crate::core::base_rpc::{RpcConnection, RpcError};
    use wasm_bindgen_test::*;
    use wasm_bindgen_test::console_log;
    use solana_sdk::pubkey::Pubkey;
    use base64;
    use flate2::read::DeflateDecoder;
    use std::io::Read;

    wasm_bindgen_test_configure!(run_in_browser);

    fn log_info(msg: &str) {
        console_log!("ℹ️  {}", msg);
    }

    fn log_error(msg: &str) {
        console_log!("❌ {}", msg);
    }

    fn log_success(msg: &str) {
        console_log!("✅ {}", msg);
    }

    fn log_json(prefix: &str, value: &serde_json::Value) {
        console_log!("📄 {}:", prefix);
        match serde_json::to_string_pretty(value) {
            Ok(pretty) => {
                for line in pretty.lines() {
                    console_log!("   {}", line);
                }
            }
            Err(e) => log_error(&format!("Failed to format JSON: {}", e))
        }
    }

    fn print_separator() {
        console_log!("\n----------------------------------------");
    }

    fn load_test_wallet() -> Result<(String, Vec<u8>), RpcError> {
        let keypair_json = include_str!("../../test-keypair/memo-test-keypair.json");
        
        let keypair_bytes: Vec<u8> = serde_json::from_str(keypair_json)
            .map_err(|e| RpcError::Other(format!("Failed to parse keypair JSON: {}", e)))?;
        
        let pubkey = bs58::encode(&keypair_bytes[32..64]).into_string();
        log_info(&format!("Successfully loaded wallet from embedded keypair file"));
        Ok((pubkey, keypair_bytes))
    }

    #[wasm_bindgen_test]
    async fn test_get_version() {
        print_separator();
        log_info("Starting version test");
        
        let rpc = RpcConnection::new();
        log_info(&format!("Using RPC endpoint: {}", "https://rpc.testnet.x1.xyz"));
        
        match rpc.get_version().await {
            Ok(version) => {
                print_separator();
                
                let version_value: serde_json::Value = serde_json::from_str(&version)
                    .expect("Failed to parse version JSON");
                
                log_json("RPC Version Response", &version_value);
                
                assert!(version_value.is_object(), "Version response should be an object");
                assert!(version_value.get("solana-core").is_some(), "Should contain solana-core version");
                
                print_separator();
                log_success("Version test completed successfully");
            },
            Err(e) => {
                print_separator();
                log_error(&format!("Version test failed: {}", e));
                panic!("Test failed");
            }
        }
    }

    #[wasm_bindgen_test]
    async fn test_get_balance() {
        print_separator();
        log_info("Starting balance test");

        match load_test_wallet() {
            Ok((pubkey, _)) => {
                log_info(&format!("Test wallet public key: {}", pubkey));

                let rpc = RpcConnection::new();
                log_info(&format!("Using RPC endpoint: {}", "https://rpc.testnet.x1.xyz"));

                match rpc.get_balance(&pubkey).await {
                    Ok(balance_response) => {
                        print_separator();
                        
                        let balance_value: serde_json::Value = serde_json::from_str(&balance_response)
                            .expect("Failed to parse balance JSON");
                        
                        log_json("Balance Response", &balance_value);

                        assert!(balance_value.get("value").is_some(), "Response should contain 'value' field");

                        if let Some(lamports) = balance_value.get("value").and_then(|v| v.as_u64()) {
                            let sol = lamports as f64 / 1_000_000_000.0;
                            log_info(&format!("\nWallet balance: {} SOL ({} lamports)", sol, lamports));
                        }

                        print_separator();
                        log_success("Balance test completed successfully");
                    },
                    Err(e) => {
                        print_separator();
                        log_error(&format!("Failed to get balance: {}", e));
                        panic!("Balance test failed");
                    }
                }
            },
            Err(e) => {
                print_separator();
                log_error(&format!("Failed to load test wallet: {}", e));
                panic!("Failed to load wallet");
            }
        }
    }

    #[wasm_bindgen_test]
    async fn test_user_profile_sequence() {
        // 0. Close profile
        test_a3_close_user_profile().await;

        // 1. Initialize profile
        test_a1_initialize_user_profile().await;
        
        // 2. Get profile
        test_a2_get_user_profile().await;

        // 3. Test mint with various memo lengths
        test_mint_with_various_memo_lengths().await;
        
        // 4. Close profile
        test_a3_close_user_profile().await;
    }

    async fn test_a2_get_user_profile() {
        print_separator();
        log_info("Starting user profile test sequence (2/3): Get Profile");

        // using load_test_wallet to get test wallet pubkey
        match load_test_wallet() {
            Ok((pubkey, _)) => {
                log_info(&format!("Test wallet public key: {}", pubkey));

                let rpc = RpcConnection::new();
                log_info(&format!("Using RPC endpoint: {}", "https://rpc.testnet.x1.xyz"));

                match rpc.get_user_profile(&pubkey).await {
                    Ok(account_info_str) => {
                        print_separator();
                        log_info("Raw account info received:");
                        log_info(&account_info_str);

                        // parse account info JSON
                        let account_info: serde_json::Value = serde_json::from_str(&account_info_str)
                            .expect("Failed to parse account info JSON");

                        // get base64 encoded data
                        if let Some(data) = account_info["value"]["data"].get(0).and_then(|v| v.as_str()) {
                            // decode base64 data
                            let decoded = base64::decode(data)
                                .expect("Failed to decode base64 data");

                            // start parsing data structure (simulate UI behavior) - updated for new UserProfile
                            let mut data = &decoded[8..]; // Skip discriminator

                            // Read pubkey
                            let mut pubkey_bytes = [0u8; 32];
                            pubkey_bytes.copy_from_slice(&data[..32]);
                            let account_pubkey = Pubkey::new_from_array(pubkey_bytes);
                            data = &data[32..];

                            // Read stats (mint/burn data only)
                            let total_minted = u64::from_le_bytes([
                                data[0], data[1], data[2], data[3], 
                                data[4], data[5], data[6], data[7]
                            ]);
                            data = &data[8..];

                            let total_burned = u64::from_le_bytes([
                                data[0], data[1], data[2], data[3], 
                                data[4], data[5], data[6], data[7]
                            ]);
                            data = &data[8..];

                            let mint_count = u64::from_le_bytes([
                                data[0], data[1], data[2], data[3], 
                                data[4], data[5], data[6], data[7]
                            ]);
                            data = &data[8..];

                            let burn_count = u64::from_le_bytes([
                                data[0], data[1], data[2], data[3], 
                                data[4], data[5], data[6], data[7]
                            ]);
                            data = &data[8..];

                            // Read timestamps
                            let created_at = i64::from_le_bytes([
                                data[0], data[1], data[2], data[3], 
                                data[4], data[5], data[6], data[7]
                            ]);
                            data = &data[8..];

                            let last_updated = i64::from_le_bytes([
                                data[0], data[1], data[2], data[3], 
                                data[4], data[5], data[6], data[7]
                            ]);

                            // display parsed user info (no username/image)
                            print_separator();
                            log_info("==== USER PROFILE ====");
                            log_info("Note: Username and profile image now handled by separate contract");
                            
                            log_info("\n==== TOKEN STATISTICS ====");
                            log_info(&format!("Total Minted: {} tokens", total_minted));
                            log_info(&format!("Total Burned: {} tokens", total_burned));
                            log_info(&format!("Net Balance: {} tokens", 
                                (total_minted as i64 - total_burned as i64)));
                            log_info(&format!("Mint Operations: {}", mint_count));
                            log_info(&format!("Burn Operations: {}", burn_count));
                            
                            log_info("\n==== ACCOUNT INFO ====");
                            log_info(&format!("Owner: {}", account_pubkey));
                            log_info(&format!("Created: {}", format_timestamp(created_at)));
                            log_info(&format!("Last Updated: {}", format_timestamp(last_updated)));
                        } else {
                            log_error("No account data found");
                        }

                        print_separator();
                        log_success("User profile test completed successfully");
                    },
                    Err(e) => {
                        print_separator();
                        log_error(&format!("Failed to get user profile: {}", e));
                        log_error("Error occurred during RPC call");
                        panic!("User profile test failed");
                    }
                }
            },
            Err(e) => {
                print_separator();
                log_error(&format!("Failed to load test wallet: {}", e));
                panic!("Failed to load wallet");
            }
        }
    }

    async fn test_a3_close_user_profile() {
        print_separator();
        log_info("Starting user profile test sequence (3/3): Close Profile");

        match load_test_wallet() {
            Ok((pubkey, keypair_bytes)) => {
                log_info(&format!("Test wallet public key: {}", pubkey));

                let rpc = RpcConnection::new();
                log_info(&format!("Using RPC endpoint: {}", "https://rpc.testnet.x1.xyz"));

                // first check if user profile exists
                match rpc.get_user_profile(&pubkey).await {
                    Ok(account_info_str) => {
                        let account_info: serde_json::Value = serde_json::from_str(&account_info_str)
                            .expect("Failed to parse account info JSON");

                        if account_info["value"].is_null() {
                            log_info("No user profile found to close");
                            return;
                        }

                        // if account exists, attempt to close it
                        log_info("Found existing user profile, attempting to close...");
                        
                        match rpc.close_user_profile(&keypair_bytes).await {
                            Ok(response) => {
                                // print raw response
                                print_separator();
                                log_info("Raw Close Profile Response:");
                                log_info(&response);
                                print_separator();

                                // try to parse response as JSON (but don't let parsing fail affect test flow)
                                match serde_json::from_str::<serde_json::Value>(&response) {
                                    Ok(json_response) => {
                                        log_json("Parsed Close Profile Response", &json_response);
                                    }
                                    Err(e) => {
                                        log_error(&format!("Failed to parse response as JSON: {}", e));
                                    }
                                }

                                // wait for transaction confirmation
                                log_info("Waiting for transaction confirmation...");
                                
                                // try 10 times, 10 seconds interval
                                for i in 1..=10 {
                                    // use gloo_timers future version for delay
                                    gloo_timers::future::TimeoutFuture::new(10_000).await;
                                    
                                    log_info(&format!("Checking account status (attempt {}/10)...", i));
                                    
                                    match rpc.get_user_profile(&pubkey).await {
                                        Ok(verify_info_str) => {
                                            let verify_info: serde_json::Value = serde_json::from_str(&verify_info_str)
                                                .expect("Failed to parse verification info JSON");

                                            if verify_info["value"].is_null() {
                                                log_success("User profile successfully closed and removed");
                                                print_separator();
                                                log_success("Close user profile test completed");
                                                return;
                                            } else {
                                                log_info("Profile still exists, waiting for confirmation...");
                                            }
                                        },
                                        Err(e) => {
                                            log_error(&format!("Failed to verify account closure: {}", e));
                                        }
                                    }
                                }

                                // if all attempts fail
                                log_error("Profile still exists after maximum retries");
                                panic!("Close operation failed - account still exists after timeout");
                            },
                            Err(e) => {
                                log_error(&format!("Failed to close user profile: {}", e));
                                panic!("Close operation failed");
                            }
                        }
                    },
                    Err(e) => {
                        log_error(&format!("Failed to check user profile: {}", e));
                        panic!("Failed to check user profile existence");
                    }
                }
            },
            Err(e) => {
                print_separator();  
                log_error(&format!("Failed to load test wallet: {}", e));
                panic!("Failed to load wallet");
            }
        }
    }

    async fn test_a1_initialize_user_profile() {
        print_separator();
        log_info("Starting user profile test sequence (1/3): Initialize Profile");

        match load_test_wallet() {
            Ok((pubkey, keypair_bytes)) => {
                log_info(&format!("Test wallet public key: {}", pubkey));

                let rpc = RpcConnection::new();
                log_info(&format!("Using RPC endpoint: {}", "https://rpc.testnet.x1.xyz"));

                // First check if user profile already exists
                match rpc.get_user_profile(&pubkey).await {
                    Ok(account_info_str) => {
                        let account_info: serde_json::Value = serde_json::from_str(&account_info_str)
                            .expect("Failed to parse account info JSON");

                        if !account_info["value"].is_null() {
                            log_info("User profile already exists, skipping initialization");
                            return;
                        }

                        // If account doesn't exist, proceed with initialization (no username/image)
                        log_info("No existing user profile found, proceeding with initialization...");
                        
                        match rpc.initialize_user_profile(&keypair_bytes).await {
                            Ok(response) => {
                                // Print raw response
                                print_separator();
                                log_info("Raw Initialize Profile Response:");
                                log_info(&response);
                                print_separator();

                                // Try to parse response as JSON
                                match serde_json::from_str::<serde_json::Value>(&response) {
                                    Ok(json_response) => {
                                        log_json("Parsed Initialize Profile Response", &json_response);
                                    }
                                    Err(e) => {
                                        log_error(&format!("Failed to parse response as JSON: {}", e));
                                    }
                                }

                                // Wait for transaction confirmation
                                log_info("Waiting for transaction confirmation...");
                                
                                // Try 10 times, 10 seconds interval
                                for i in 1..=10 {
                                    gloo_timers::future::TimeoutFuture::new(10_000).await;
                                    
                                    log_info(&format!("Checking account status (attempt {}/10)...", i));
                                    
                                    match rpc.get_user_profile(&pubkey).await {
                                        Ok(verify_info_str) => {
                                            let verify_info: serde_json::Value = serde_json::from_str(&verify_info_str)
                                                .expect("Failed to parse verification info JSON");

                                            if !verify_info["value"].is_null() {
                                                log_success("User profile successfully initialized (mint/burn tracking only)");
                                                print_separator();
                                                log_success("Initialize user profile test completed");
                                                return;
                                            } else {
                                                log_info("Profile not yet created, waiting for confirmation...");
                                            }
                                        },
                                        Err(e) => {
                                            log_error(&format!("Failed to verify account creation: {}", e));
                                        }
                                    }
                                }

                                // If all attempts fail
                                log_error("Profile not created after maximum retries");
                                panic!("Initialize operation failed - account not created after timeout");
                            },
                            Err(e) => {
                                log_error(&format!("Failed to initialize user profile: {}", e));
                                panic!("Initialize operation failed");
                            }
                        }
                    },
                    Err(e) => {
                        log_error(&format!("Failed to check user profile: {}", e));
                        panic!("Failed to check user profile existence");
                    }
                }
            },
            Err(e) => {
                print_separator();  
                log_error(&format!("Failed to load test wallet: {}", e));
                panic!("Failed to load wallet");
            }
        }
    }

    // Helper functions for the test
    fn format_timestamp(timestamp: i64) -> String {
        let secs = timestamp as u64;
        let days = secs / 86400;
        let hours = (secs % 86400) / 3600;
        let minutes = (secs % 3600) / 60;
        let seconds = secs % 60;
        
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
            1970 + (days / 365),
            ((days % 365) / 30) + 1,
            ((days % 365) % 30) + 1,
            hours,
            minutes,
            seconds
        )
    }

    fn display_pixel_art(profile_image: &str) {
        if profile_image.is_empty() {
            return;
        }

        // parse prefix and data
        let (prefix, data) = match profile_image.split_once(':') {
            Some(("c", compressed)) => {
                // handle compressed data
                match decompress_with_deflate(compressed) {
                    Ok(decompressed) => ("n", decompressed),
                    Err(e) => {
                        log_error(&format!("Error decompressing profile image: {}", e));
                        return;
                    }
                }
            },
            Some(("n", uncompressed)) => ("n", uncompressed.to_string()),
            _ => {
                log_error("Invalid profile image format");
                return;
            }
        };

        // display pixel art
        log_info("\nPixel Art Representation:");
        let mut bit_count = 0;
        let mut current_row = String::with_capacity(32);

        for c in data.chars() {
            if let Some(value) = map_from_safe_char(c) {
                for i in (0..6).rev() {
                    let bit = (value & (1 << i)) != 0;
                    current_row.push_str(if bit { "⬛" } else { "⬜" });
                    bit_count += 1;
                    
                    if bit_count % 32 == 0 {
                        console_log!("{}\n", current_row);
                        current_row.clear();
                    }
                }
            }
        }

        if !current_row.is_empty() {
            console_log!("{}\n", current_row);
        }
        console_log!("\n");
    }

    fn map_from_safe_char(c: char) -> Option<u8> {
        let ascii = c as u8;
        
        if c == ':' || c == '\\' || c == '"' {
            return None;
        }
        
        if ascii < 35 || ascii > 126 {
            return None;
        }
        
        let mut value = ascii - 35;
        if ascii > 92 { value -= 1; }  // adjust '\'
        if ascii > 58 { value -= 1; }  // adjust ':'
        
        if value >= 64 {
            return None;
        }
        
        Some(value)
    }

    fn decompress_with_deflate(input: &str) -> Result<String, String> {
        let bytes = base64::decode(input)
            .map_err(|e| format!("Base64 decode error: {}", e))?;
            
        let mut decoder = DeflateDecoder::new(&bytes[..]);
        let mut decompressed = Vec::new();
        
        decoder.read_to_end(&mut decompressed)
            .map_err(|e| format!("Decompression error: {}", e))?;
            
        let result: String = decompressed.into_iter()
            .map(|b| b as char)
            .collect();
            
        Ok(result)
    }

    #[wasm_bindgen_test]
    async fn test_get_latest_burn_shard() {
        print_separator();
        log_info("Starting latest burn shard test");

        match load_test_wallet() {
            Ok((pubkey, _)) => {
                log_info(&format!("Test wallet public key: {}", pubkey));

                let rpc = RpcConnection::new();
                log_info(&format!("Using RPC endpoint: {}", "https://rpc.testnet.x1.xyz"));

                match rpc.get_latest_burn_shard().await {
                    Ok(account_info_str) => {
                        print_separator();
                        log_info("Raw burn shard info received:");
                        log_info(&account_info_str);

                        // parse account info JSON
                        let account_info: serde_json::Value = serde_json::from_str(&account_info_str)
                            .expect("Failed to parse account info JSON");

                        // get base64 encoded data
                        if let Some(data) = account_info["value"]["data"].get(0).and_then(|v| v.as_str()) {
                            // decode base64 data
                            let decoded = base64::decode(data)
                                .expect("Failed to decode base64 data");

                            // start parsing data structure (simulate UI behavior)
                            let mut data = &decoded[8..]; // Skip discriminator

                            // parse current_index (1 byte)
                            let current_index = data[0];
                            data = &data[1..];
                            
                            // parse records vector length
                            let vec_len = u32::from_le_bytes(data[..4].try_into().unwrap()) as usize;
                            data = &data[4..];

                            // display parsed info
                            print_separator();
                            log_info("==== BURN SHARD INFO ====");
                            log_info(&format!("Current Index: {}", current_index));
                            log_info(&format!("Number of Records: {}", vec_len));

                            // parse and display each record
                            for i in 0..vec_len {
                                // parse pubkey (32 bytes)
                                let mut pubkey_bytes = [0u8; 32];
                                pubkey_bytes.copy_from_slice(&data[..32]);
                                let record_pubkey = Pubkey::new_from_array(pubkey_bytes);
                                data = &data[32..];
                                
                                // parse signature string
                                let sig_len = u32::from_le_bytes(data[..4].try_into().unwrap()) as usize;
                                data = &data[4..];
                                let signature = String::from_utf8(data[..sig_len].to_vec())
                                    .expect("Invalid signature encoding");
                                data = &data[sig_len..];
                                
                                // parse slot (8 bytes)
                                let slot = u64::from_le_bytes(data[..8].try_into().unwrap());
                                data = &data[8..];
                                
                                // parse blocktime (8 bytes)
                                let blocktime = i64::from_le_bytes(data[..8].try_into().unwrap());
                                data = &data[8..];
                                
                                // parse amount (8 bytes)
                                let amount = u64::from_le_bytes(data[..8].try_into().unwrap());
                                data = &data[8..];

                                log_info(&format!("\nRecord #{}", i + 1));
                                log_info(&format!("  Burner: {}", record_pubkey));
                                log_info(&format!("  Signature: {}", signature));
                                log_info(&format!("  Slot: {}", slot));
                                log_info(&format!("  Blocktime: {}", format_timestamp(blocktime)));
                                log_info(&format!("  Amount: {} tokens", amount as f64 / 1_000_000_000.0));
                            }

                            print_separator();
                            log_success("Latest burn shard test completed successfully");
                        } else {
                            log_error("No account data found");
                        }
                    },
                    Err(e) => {
                        print_separator();
                        log_error(&format!("Failed to get latest burn shard: {}", e));
                        panic!("Latest burn shard test failed");
                    }
                }
            },
            Err(e) => {
                print_separator();
                log_error(&format!("Failed to load test wallet: {}", e));
                panic!("Failed to load wallet");
            }
        }
    }

    fn create_test_memo(target_length: usize) -> String {
        // fixed signature
        let signature = "2ZaX";
        
        // calculate base JSON structure length (excluding message content)
        // {"message":"","signature":""}
        let base_length = 29 + signature.len();
        
        // calculate required message length
        let message_length = target_length - base_length;
        let message = "a".repeat(message_length);
        
        // create complete JSON
        let memo_json = serde_json::json!({
            "message": message,
            "signature": signature
        });
        
        // convert to string
        let memo = serde_json::to_string(&memo_json).unwrap();
        
        // verify length
        assert_eq!(memo.len(), target_length, 
            "Generated memo length {} does not match target length {}", 
            memo.len(), target_length);
        
        memo
    }

    async fn test_mint_with_various_memo_lengths() {
        print_separator();
        log_info("Starting mint test with various memo lengths");

        match load_test_wallet() {
            Ok((pubkey, keypair_bytes)) => {
                log_info(&format!("Test wallet public key: {}", pubkey));

                let rpc = RpcConnection::new();
                log_info(&format!("Using RPC endpoint: {}", "https://rpc.testnet.x1.xyz"));

                // define lengths to test
                let test_lengths = vec![100, 200, 300, 400, 500, 600, 700];

                for length in test_lengths {
                    print_separator();
                    log_info(&format!("\nTesting memo with length: {} bytes", length));

                    // create memo of specified length
                    let memo = create_test_memo(length);
                    log_info(&format!("Generated memo length: {}", memo.len()));
                    log_info(&format!("Memo content: {}", memo));

                    // send mint transaction
                    match rpc.mint(&memo, &keypair_bytes).await {
                        Ok(response) => {
                            // parse complete JSON response
                            let json_response: serde_json::Value = serde_json::from_str(&response)
                                .expect("Failed to parse response");
                            
                            // get signature
                            let signature = json_response.as_str()
                                .expect("Failed to get signature from response");
                            
                            log_success(&format!("Transaction signature: {}", signature));
                            
                            // wait for transaction confirmation
                            log_info("Waiting for transaction confirmation...");
                            
                            // try 5 times, 10 seconds interval
                            for i in 1..=5 {
                                gloo_timers::future::TimeoutFuture::new(10_000).await;
                                
                                match rpc.get_transaction_status(signature).await {
                                    Ok(status_response) => {
                                        let status: serde_json::Value = serde_json::from_str(&status_response)
                                            .expect("Failed to parse status response");
                                        
                                        if let Some(value) = status["value"].get(0) {
                                            if value["confirmationStatus"].as_str() == Some("finalized") {
                                                log_success(&format!("Transaction confirmed for memo length {}", length));
                                                break;
                                            }
                                        }
                                        
                                        if i == 5 {
                                            log_error(&format!("Transaction not confirmed after maximum attempts for memo length {}", length));
                                        } else {
                                            log_info(&format!("Checking confirmation status (attempt {}/5)...", i));
                                        }
                                    },
                                    Err(e) => {
                                        log_error(&format!("Failed to check transaction status: {}", e));
                                    }
                                }
                            }
                        },
                        Err(e) => {
                            log_error(&format!("Failed to mint with memo length {}: {}", length, e));
                        }
                    }

                    // add delay between tests
                    gloo_timers::future::TimeoutFuture::new(5_000).await;
                }

                print_separator();
                log_success("Mint test with various memo lengths completed");
            },
            Err(e) => {
                print_separator();
                log_error(&format!("Failed to load test wallet: {}", e));
                panic!("Failed to load wallet");
            }
        }
    }
} 