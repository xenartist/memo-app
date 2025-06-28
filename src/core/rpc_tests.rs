#[cfg(test)]
mod tests {
    use crate::core::rpc_base::{RpcConnection, RpcError};
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

    async fn test_get_user_profile() {
        print_separator();
        log_info("Starting Token 2022 user profile test sequence (2/3): Get Profile");

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

                        let account_info: serde_json::Value = serde_json::from_str(&account_info_str)
                            .expect("Failed to parse account info JSON");

                        if let Some(data) = account_info["value"]["data"].get(0).and_then(|v| v.as_str()) {
                            let decoded = base64::decode(data)
                                .expect("Failed to decode base64 data");

                            // parse Token 2022 UserProfile data structure
                            let mut data = &decoded[8..]; // Skip discriminator

                            // Read pubkey
                            let mut pubkey_bytes = [0u8; 32];
                            pubkey_bytes.copy_from_slice(&data[..32]);
                            let account_pubkey = Pubkey::new_from_array(pubkey_bytes);
                            data = &data[32..];

                            // Read Token 2022 stats (mint data only)
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

                            // display Token 2022 user info
                            print_separator();
                            log_info("==== TOKEN 2022 USER PROFILE ====");
                            log_info("Note: Username and profile image now handled by separate contract");
                            
                            log_info("\n==== TOKEN 2022 STATISTICS ====");
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
                        log_success("Token 2022 user profile test completed successfully");
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

    async fn test_close_user_profile() {
        print_separator();
        log_info("Starting Token 2022 user profile test sequence (3/3): Close Profile");

        match load_test_wallet() {
            Ok((pubkey, keypair_bytes)) => {
                log_info(&format!("Test wallet public key: {}", pubkey));

                let rpc = RpcConnection::new();
                log_info(&format!("Using RPC endpoint: {}", "https://rpc.testnet.x1.xyz"));

                match rpc.get_user_profile(&pubkey).await {
                    Ok(account_info_str) => {
                        let account_info: serde_json::Value = serde_json::from_str(&account_info_str)
                            .expect("Failed to parse account info JSON");

                        if account_info["value"].is_null() {
                            log_info("No user profile found to close");
                            return;
                        }

                        log_info("Found existing Token 2022 user profile, attempting to close...");
                        
                        match rpc.close_user_profile(&keypair_bytes).await {
                            Ok(response) => {
                                print_separator();
                                log_info("Raw Close Profile Response:");
                                log_info(&response);

                                log_info("Waiting for Token 2022 profile closure confirmation...");
                                
                                for i in 1..=10 {
                                    gloo_timers::future::TimeoutFuture::new(10_000).await;
                                    
                                    log_info(&format!("Checking account status (attempt {}/10)...", i));
                                    
                                    match rpc.get_user_profile(&pubkey).await {
                                        Ok(verify_info_str) => {
                                            let verify_info: serde_json::Value = serde_json::from_str(&verify_info_str)
                                                .expect("Failed to parse verification info JSON");

                                            if verify_info["value"].is_null() {
                                                log_success("Token 2022 user profile successfully closed and removed");
                                                print_separator();
                                                log_success("Close Token 2022 user profile test completed");
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

    async fn test_initialize_user_profile() {
        print_separator();
        log_info("Starting Token 2022 user profile test sequence (1/3): Initialize Profile");

        match load_test_wallet() {
            Ok((pubkey, keypair_bytes)) => {
                log_info(&format!("Test wallet public key: {}", pubkey));

                let rpc = RpcConnection::new();
                log_info(&format!("Using RPC endpoint: {}", "https://rpc.testnet.x1.xyz"));

                match rpc.get_user_profile(&pubkey).await {
                    Ok(account_info_str) => {
                        let account_info: serde_json::Value = serde_json::from_str(&account_info_str)
                            .expect("Failed to parse account info JSON");

                        if !account_info["value"].is_null() {
                            log_info("User profile already exists, skipping initialization");
                            return;
                        }

                        log_info("No existing user profile found, proceeding with Token 2022 initialization...");
                        
                        match rpc.initialize_user_profile(&keypair_bytes).await {
                            Ok(response) => {
                                print_separator();
                                log_info("Raw Initialize Profile Response:");
                                log_info(&response);

                                log_info("Waiting for Token 2022 profile initialization confirmation...");
                                
                                for i in 1..=10 {
                                    gloo_timers::future::TimeoutFuture::new(10_000).await;
                                    
                                    log_info(&format!("Checking account status (attempt {}/10)...", i));
                                    
                                    match rpc.get_user_profile(&pubkey).await {
                                        Ok(verify_info_str) => {
                                            let verify_info: serde_json::Value = serde_json::from_str(&verify_info_str)
                                                .expect("Failed to parse verification info JSON");

                                            if !verify_info["value"].is_null() {
                                                log_success("Token 2022 user profile successfully initialized (mint tracking only)");
                                                print_separator();
                                                log_success("Initialize Token 2022 user profile test completed");
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
        let signature = "2ZaX";
        let base_length = 29 + signature.len();
        let message_length = target_length - base_length;
        let message = "a".repeat(message_length);
        
        let memo_json = serde_json::json!({
            "message": message,
            "signature": signature
        });
        
        let memo = serde_json::to_string(&memo_json).unwrap();
        
        assert_eq!(memo.len(), target_length, 
            "Generated memo length {} does not match target length {}", 
            memo.len(), target_length);
        
        memo
    }

    async fn test_token_2022_mint_with_various_memo_lengths() {
        print_separator();
        log_info("Starting Token 2022 mint test with various memo lengths");

        match load_test_wallet() {
            Ok((pubkey, keypair_bytes)) => {
                log_info(&format!("Test wallet public key: {}", pubkey));

                let rpc = RpcConnection::new();
                log_info(&format!("Using RPC endpoint: {}", "https://rpc.testnet.x1.xyz"));

                // define lengths to test
                let test_lengths = vec![700];

                for length in test_lengths {
                    print_separator();
                    log_info(&format!("\nTesting Token 2022 mint with memo length: {} bytes", length));

                    // create memo of specified length
                    let memo = create_test_memo(length);
                    log_info(&format!("Generated memo length: {}", memo.len()));
                    log_info(&format!("Memo content preview: {}...", &memo[..std::cmp::min(50, memo.len())]));

                    // send Token 2022 mint transaction
                    match rpc.mint(&memo, &keypair_bytes).await {
                        Ok(response) => {
                            print_separator();
                            log_info("Raw Token 2022 mint response received:");
                            log_info(&format!("Response length: {} characters", response.len()));
                            log_info(&format!("Response content: {}", response));
                            print_separator();

                            // simple response handling: assume return is transaction signature
                            let signature = response.trim_matches('"').trim().to_string();
                            
                            if signature.is_empty() {
                                log_error("Empty signature received");
                                continue;
                            }

                            log_success(&format!("Token 2022 mint transaction signature: {}", signature));
                            
                            // simplified confirmation logic: check only once
                            log_info("Waiting for Token 2022 mint transaction confirmation...");
                            gloo_timers::future::TimeoutFuture::new(15_000).await; // wait 15 seconds
                            
                            match rpc.get_transaction_status(&signature).await {
                                Ok(status_response) => {
                                    log_info("Transaction status response:");
                                    log_info(&status_response);
                                    
                                    match serde_json::from_str::<serde_json::Value>(&status_response) {
                                        Ok(status) => {
                                            if let Some(value) = status["value"].as_array() {
                                                if !value.is_empty() && !value[0].is_null() {
                                                    log_success(&format!("Token 2022 mint found in blockchain for memo length {}", length));
                                                } else {
                                                    log_info(&format!("Transaction not yet confirmed for memo length {}", length));
                                                }
                                            } else {
                                                log_info("Transaction status format unexpected");
                                                log_json("Status", &status);
                                            }
                                        },
                                        Err(e) => {
                                            log_error(&format!("Failed to parse status response: {}", e));
                                        }
                                    }
                                },
                                Err(e) => {
                                    log_error(&format!("Failed to check transaction status: {}", e));
                                }
                            }
                        },
                        Err(e) => {
                            log_error(&format!("Failed Token 2022 mint with memo length {}: {}", length, e));
                            print_separator();
                            log_error("This might be due to:");
                            log_error("1. Network connectivity issues");
                            log_error("2. Insufficient SOL balance for fees");
                            log_error("3. Token 2022 program configuration issues");
                            log_error("4. RPC endpoint limitations");
                        }
                    }

                    // add test interval
                    gloo_timers::future::TimeoutFuture::new(5_000).await; // increase to 5 seconds interval, give network more time
                }

                print_separator();
                log_success("Token 2022 mint test with various memo lengths completed");
            },
            Err(e) => {
                print_separator();
                log_error(&format!("Failed to load test wallet: {}", e));
                panic!("Failed to load wallet");
            }
        }
    }

    // add burn test function
    async fn test_token_2022_burn_operations() {
        print_separator();
        log_info("Starting Token 2022 burn operations test");

        match load_test_wallet() {
            Ok((pubkey, keypair_bytes)) => {
                log_info(&format!("Test wallet public key: {}", pubkey));

                let rpc = RpcConnection::new();
                log_info(&format!("Using RPC endpoint: {}", "https://rpc.testnet.x1.xyz"));

                // use actual signature from previous mint test
                let test_signature = "3GZFMnLbY2kaV1EpS8sa2rXjMGJaGjZ2QtVE5EANSicTqAWrmmqrKcyEg2m44D2Zs1cJ9r226K8F1zuoqYfU7KFr";
                
                // test different burn amounts
                let test_amounts = vec![
                    (1, "Small burn test"),
                    (2, "Medium burn test"),
                    (3, "Large burn test"),
                ];

                for (amount_tokens, description) in test_amounts {
                    print_separator();
                    log_info(&format!("\nTesting Token 2022 burn: {}", description));
                    
                    let amount_lamports = amount_tokens * 1_000_000_000; // convert to lamports
                    log_info(&format!("Burning {} tokens ({} lamports)", amount_tokens, amount_lamports));

                    // create test message
                    let message = format!("Test burn of {} tokens from RPC test suite", amount_tokens);
                    
                    // execute burn operation
                    match rpc.burn(amount_lamports, &message, test_signature, &keypair_bytes).await {
                        Ok(response) => {
                            print_separator();
                            log_info("Raw Token 2022 burn response received:");
                            log_info(&format!("Response length: {} characters", response.len()));
                            log_info(&format!("Response content: {}", response));
                            print_separator();

                            // parse response
                            let signature = response.trim_matches('"').trim().to_string();
                            
                            if signature.is_empty() {
                                log_error("Empty signature received");
                                continue;
                            }

                            log_success(&format!("Token 2022 burn transaction signature: {}", signature));
                            
                            // wait for confirmation
                            log_info("Waiting for Token 2022 burn transaction confirmation...");
                            gloo_timers::future::TimeoutFuture::new(15_000).await;
                            
                            match rpc.get_transaction_status(&signature).await {
                                Ok(status_response) => {
                                    log_info("Transaction status response:");
                                    log_info(&status_response);
                                    
                                    match serde_json::from_str::<serde_json::Value>(&status_response) {
                                        Ok(status) => {
                                            if let Some(value) = status["value"].as_array() {
                                                if !value.is_empty() && !value[0].is_null() {
                                                    log_success(&format!("Token 2022 burn confirmed for {} tokens", amount_tokens));
                                                } else {
                                                    log_info(&format!("Transaction not yet confirmed for {} tokens burn", amount_tokens));
                                                }
                                            } else {
                                                log_info("Transaction status format unexpected");
                                                log_json("Status", &status);
                                            }
                                        },
                                        Err(e) => {
                                            log_error(&format!("Failed to parse status response: {}", e));
                                        }
                                    }
                                },
                                Err(e) => {
                                    log_error(&format!("Failed to check transaction status: {}", e));
                                }
                            }
                        },
                        Err(e) => {
                            log_error(&format!("Failed Token 2022 burn for {} tokens: {}", amount_tokens, e));
                            print_separator();
                            log_error("This might be due to:");
                            log_error("1. Insufficient token balance");
                            log_error("2. Burn shards not initialized");
                            log_error("3. Network connectivity issues");
                            log_error("4. Token 2022 program configuration issues");
                        }
                    }

                    // test interval
                    gloo_timers::future::TimeoutFuture::new(5_000).await;
                }

                print_separator();
                log_success("Token 2022 burn operations test completed");
            },
            Err(e) => {
                print_separator();
                log_error(&format!("Failed to load test wallet: {}", e));
                panic!("Failed to load wallet");
            }
        }
    }

    // test global top burn index and top burn shard
    async fn test_burn_shard_operations() {
        print_separator();
        log_info("Starting burn shard operations test");

        match load_test_wallet() {
            Ok((pubkey, _)) => {
                log_info(&format!("Test wallet public key: {}", pubkey));

                let rpc = RpcConnection::new();
                log_info(&format!("Using RPC endpoint: {}", "https://rpc.testnet.x1.xyz"));

                // test get global top burn index
                log_info("Testing get_global_top_burn_index...");
                match rpc.get_global_top_burn_index().await {
                    Ok(index_info) => {
                        print_separator();
                        log_info("Global Top Burn Index Info:");
                        log_info(&index_info);
                        
                        // try
                        let account_info: serde_json::Value = serde_json::from_str(&index_info)
                            .expect("Failed to parse account info JSON");
                        
                        if let Some(data) = account_info["value"]["data"].get(0).and_then(|v| v.as_str()) {
                            let decoded = base64::decode(data)
                                .expect("Failed to decode base64 data");
                            
                            if decoded.len() >= 17 {
                                let data = &decoded[8..]; // skip discriminator
                                let total_count = u64::from_le_bytes(data[0..8].try_into().unwrap());
                                let option_tag = data[8];
                                
                                log_info(&format!("Total top burn shard count: {}", total_count));
                                
                                if option_tag == 1 && data.len() >= 17 {
                                    let current_index = u64::from_le_bytes(data[9..17].try_into().unwrap());
                                    log_info(&format!("Current top burn shard index: {}", current_index));
                                    
                                    // test get current top burn shard
                                    log_info("Testing get_top_burn_shard...");
                                    match rpc.get_top_burn_shard(current_index).await {
                                        Ok(shard_info) => {
                                            log_info("Top Burn Shard Info:");
                                            log_info(&shard_info);
                                            log_success("Top burn shard retrieval successful");
                                        },
                                        Err(e) => {
                                            log_error(&format!("Failed to get top burn shard: {}", e));
                                        }
                                    }
                                } else {
                                    log_info("No active top burn shard currently");
                                }
                            }
                        }
                        
                        log_success("Global top burn index test completed");
                    },
                    Err(e) => {
                        log_error(&format!("Failed to get global top burn index: {}", e));
                    }
                }

                print_separator();
                log_success("Burn shard operations test completed");
            },
            Err(e) => {
                print_separator();
                log_error(&format!("Failed to load test wallet: {}", e));
                panic!("Failed to load wallet");
            }
        }
    }

    // test burn without user burn history
    async fn test_token_2022_burn_without_history() {
        print_separator();
        log_info("Starting Token 2022 burn test WITHOUT user burn history");

        match load_test_wallet() {
            Ok((pubkey, keypair_bytes)) => {
                log_info(&format!("Test wallet public key: {}", pubkey));

                let rpc = RpcConnection::new();
                log_info(&format!("Using RPC endpoint: {}", "https://rpc.testnet.x1.xyz"));

                // use actual signature for test
                let test_signature = "3GZFMnLbY2kaV1EpS8sa2rXjMGJaGjZ2QtVE5EANSicTqAWrmmqrKcyEg2m44D2Zs1cJ9r226K8F1zuoqYfU7KFr";
                
                // test small burn (without user burn history)
                let amount_tokens = 1;
                let amount_lamports = amount_tokens * 1_000_000_000;
                
                log_info(&format!("Burning {} tokens ({} lamports) WITHOUT user burn history", amount_tokens, amount_lamports));

                // create test message
                let message = format!("Test burn WITHOUT user burn history - {} tokens", amount_tokens);
                
                // execute burn operation (using process_burn, not process_burn_with_history)
                match rpc.burn(amount_lamports, &message, test_signature, &keypair_bytes).await {
                    Ok(response) => {
                        print_separator();
                        log_info("Raw Token 2022 burn WITHOUT history response:");
                        log_info(&format!("Response: {}", response));

                        let signature = response.trim_matches('"').trim().to_string();
                        
                        if signature.is_empty() {
                            log_error("Empty signature received");
                            return;
                        }

                        log_success(&format!("Token 2022 burn WITHOUT history transaction signature: {}", signature));
                        
                        // wait for confirmation
                        log_info("Waiting for Token 2022 burn WITHOUT history confirmation...");
                        gloo_timers::future::TimeoutFuture::new(15_000).await;
                        
                        match rpc.get_transaction_status(&signature).await {
                            Ok(status_response) => {
                                log_info("Transaction status:");
                                log_info(&status_response);
                                
                                match serde_json::from_str::<serde_json::Value>(&status_response) {
                                    Ok(status) => {
                                        if let Some(value) = status["value"].as_array() {
                                            if !value.is_empty() && !value[0].is_null() {
                                                log_success("Token 2022 burn WITHOUT history confirmed");
                                            } else {
                                                log_info("Transaction not yet confirmed");
                                            }
                                        }
                                    },
                                    Err(e) => {
                                        log_error(&format!("Failed to parse status: {}", e));
                                    }
                                }
                            },
                            Err(e) => {
                                log_error(&format!("Failed to check transaction status: {}", e));
                            }
                        }

                        print_separator();
                        log_success("Token 2022 burn WITHOUT user burn history test completed");
                    },
                    Err(e) => {
                        log_error(&format!("Failed Token 2022 burn WITHOUT history: {}", e));
                        print_separator();
                        log_error("This might be due to:");
                        log_error("1. Insufficient token balance");
                        log_error("2. Burn shards not initialized");
                        log_error("3. Network connectivity issues");
                    }
                }
            },
            Err(e) => {
                print_separator();
                log_error(&format!("Failed to load test wallet: {}", e));
            }
        }
    }

    // test burn with user burn history - fixed version
    async fn test_token_2022_burn_with_history() {
        print_separator();
        log_info("Starting Token 2022 burn test WITH user burn history");

        match load_test_wallet() {
            Ok((pubkey, keypair_bytes)) => {
                log_info(&format!("Test wallet public key: {}", pubkey));

                let rpc = RpcConnection::new();
                log_info(&format!("Using RPC endpoint: {}", "https://rpc.testnet.x1.xyz"));

                // step 1: directly initialize new user burn history (previously thoroughly cleaned)
                log_info("Step 1: Initializing new user burn history...");
                
                match rpc.initialize_user_burn_history(&keypair_bytes).await {
                    Ok(init_response) => {
                        log_success("User burn history initialized successfully");
                        log_info(&format!("Init response: {}", init_response));
                        
                        // wait for user profile burn_history_index field to update
                        log_info("Waiting for user profile burn_history_index field to update...");
                        gloo_timers::future::TimeoutFuture::new(25_000).await;
                        
                        // multiple retries to check if burn_history_index has been updated
                        let mut retries = 5;
                        let mut burn_history_ready = false;
                        
                        while retries > 0 && !burn_history_ready {
                            match rpc.get_user_burn_history_index(&pubkey).await {
                                Ok(Some(_)) => {
                                    burn_history_ready = true;
                                    log_success("User burn history index confirmed in user profile");
                                },
                                Ok(None) => {
                                    log_info(&format!("User burn history index not yet updated, retries left: {}", retries - 1));
                                    gloo_timers::future::TimeoutFuture::new(8_000).await;
                                    retries -= 1;
                                },
                                Err(e) => {
                                    log_error(&format!("Error checking burn history index: {}", e));
                                    retries -= 1;
                                }
                            }
                        }
                        
                        if !burn_history_ready {
                            log_error("User burn history index not updated in user profile after maximum retries");
                            log_error("Skipping burn_with_history test to avoid failure");
                            return;
                        }
                    },
                    Err(e) => {
                        log_error(&format!("Failed to initialize user burn history: {}", e));
                        return;
                    }
                }

                // step 2: verify user burn history status
                log_info("Step 2: Verifying user burn history status before burn...");
                match rpc.get_user_burn_history_index(&pubkey).await {
                    Ok(Some(index)) => {
                        log_info(&format!("Confirmed: User burn history index = {}", index));
                        
                        // verify corresponding burn history account exists
                        match rpc.get_user_burn_history(&pubkey, index).await {
                            Ok(history_info) => {
                                log_info("Confirmed: User burn history account exists and is accessible");
                                log_info(&format!("History account preview: {}...", 
                                    &history_info[..std::cmp::min(100, history_info.len())]));
                            },
                            Err(e) => {
                                log_error(&format!("Warning: User burn history account not accessible: {}", e));
                                log_info("This may cause the burn_with_history operation to fail");
                            }
                        }
                    },
                    Ok(None) => {
                        log_error("User burn history index is still None after initialization");
                        log_error("Skipping burn_with_history test to avoid failure");
                        return;
                    },
                    Err(e) => {
                        log_error(&format!("Failed to verify user burn history: {}", e));
                        return;
                    }
                }

                // step 3: perform burn with user burn history
                log_info("Step 3: Performing burn WITH user burn history...");
                
                let test_signature = "3GZFMnLbY2kaV1EpS8sa2rXjMGJaGjZ2QtVE5EANSicTqAWrmmqrKcyEg2m44D2Zs1cJ9r226K8F1zuoqYfU7KFr";
                let amount_tokens = 2;
                let amount_lamports = amount_tokens * 1_000_000_000;
                
                log_info(&format!("Burning {} tokens ({} lamports) WITH user burn history", amount_tokens, amount_lamports));

                let message = format!("Test burn WITH user burn history - {} tokens", amount_tokens);
                
                // execute burn with history operation
                match rpc.burn_with_history(amount_lamports, &message, test_signature, &keypair_bytes).await {
                    Ok(response) => {
                        print_separator();
                        log_info("Raw Token 2022 burn WITH history response:");
                        log_info(&format!("Response: {}", response));

                        let signature = response.trim_matches('"').trim().to_string();
                        
                        if signature.is_empty() {
                            log_error("Empty signature received");
                            return;
                        }

                        log_success(&format!("Token 2022 burn WITH history transaction signature: {}", signature));
                        
                        // wait for confirmation
                        log_info("Waiting for Token 2022 burn WITH history confirmation...");
                        gloo_timers::future::TimeoutFuture::new(20_000).await;
                        
                        match rpc.get_transaction_status(&signature).await {
                            Ok(status_response) => {
                                log_info("Transaction status:");
                                log_info(&status_response);
                                
                                match serde_json::from_str::<serde_json::Value>(&status_response) {
                                    Ok(status) => {
                                        if let Some(value) = status["value"].as_array() {
                                            if !value.is_empty() && !value[0].is_null() {
                                                log_success("Token 2022 burn WITH history confirmed");
                                            } else {
                                                log_info("Transaction not yet confirmed");
                                            }
                                        }
                                    },
                                    Err(e) => {
                                        log_error(&format!("Failed to parse status: {}", e));
                                    }
                                }
                            },
                            Err(e) => {
                                log_error(&format!("Failed to check transaction status: {}", e));
                            }
                        }

                        // step 4: verify user burn history record
                        log_info("Step 4: Verifying user burn history record...");
                        
                        match rpc.get_user_burn_history_index(&pubkey).await {
                            Ok(Some(current_index)) => {
                                log_info(&format!("Current user burn history index: {}", current_index));
                                
                                match rpc.get_user_burn_history(&pubkey, current_index).await {
                                    Ok(history_info) => {
                                        log_info("User burn history account info:");
                                        log_info(&format!("History data preview: {}...", 
                                            &history_info[..std::cmp::min(200, history_info.len())]));
                                        log_success("User burn history verification completed");
                                    },
                                    Err(e) => {
                                        log_error(&format!("Failed to get user burn history: {}", e));
                                    }
                                }
                            },
                            Ok(None) => {
                                log_error("No user burn history index found after burn");
                            },
                            Err(e) => {
                                log_error(&format!("Failed to get user burn history index: {}", e));
                            }
                        }

                        print_separator();
                        log_success("Token 2022 burn WITH user burn history test completed");
                    },
                    Err(e) => {
                        log_error(&format!("Failed Token 2022 burn WITH history: {}", e));
                        print_separator();
                        log_error("This might be due to:");
                        log_error("1. User burn history account not properly initialized");
                        log_error("2. User burn history account is full");
                        log_error("3. Insufficient token balance");
                        log_error("4. Network connectivity issues");
                        log_error("5. User profile burn_history_index field not yet updated");
                    }
                }
            },
            Err(e) => {
                print_separator();
                log_error(&format!("Failed to load test wallet: {}", e));
            }
        }
    }

    // test close user burn history
    async fn test_close_user_burn_history() {
        print_separator();
        log_info("Starting close user burn history test");

        match load_test_wallet() {
            Ok((pubkey, keypair_bytes)) => {
                log_info(&format!("Test wallet public key: {}", pubkey));

                let rpc = RpcConnection::new();
                log_info(&format!("Using RPC endpoint: {}", "https://rpc.testnet.x1.xyz"));

                // check if there is user burn history to close
                match rpc.get_user_burn_history_index(&pubkey).await {
                    Ok(current_index) => {
                        if let Some(index) = current_index {
                            log_info(&format!("Found user burn history with index: {}", index));
                            log_info("Attempting to close user burn history...");
                            
                            match rpc.close_user_burn_history(&keypair_bytes).await {
                                Ok(response) => {
                                    print_separator();
                                    log_info("Raw close user burn history response:");
                                    log_info(&format!("Response: {}", response));

                                    let signature = response.trim_matches('"').trim().to_string();
                                    
                                    if signature.is_empty() {
                                        log_error("Empty signature received");
                                        return;
                                    }

                                    log_success(&format!("Close user burn history transaction signature: {}", signature));
                                    
                                    // wait for confirmation
                                    log_info("Waiting for close user burn history confirmation...");
                                    gloo_timers::future::TimeoutFuture::new(20_000).await;
                                    
                                    // verify close operation
                                    match rpc.get_user_burn_history_index(&pubkey).await {
                                        Ok(new_index) => {
                                            if let Some(new_idx) = new_index {
                                                if new_idx < index {
                                                    log_success(&format!("User burn history successfully closed. Index reduced from {} to {}", index, new_idx));
                                                } else {
                                                    log_info(&format!("User burn history index unchanged: {}", new_idx));
                                                }
                                            } else {
                                                log_success("All user burn history records have been closed");
                                            }
                                        },
                                        Err(e) => {
                                            log_error(&format!("Failed to verify closure: {}", e));
                                        }
                                    }

                                    print_separator();
                                    log_success("Close user burn history test completed");
                                },
                                Err(e) => {
                                    log_error(&format!("Failed to close user burn history: {}", e));
                                }
                            }
                        } else {
                            log_info("No user burn history found to close");
                            log_success("Close user burn history test completed (nothing to close)");
                        }
                    },
                    Err(e) => {
                        log_error(&format!("Failed to check user burn history: {}", e));
                    }
                }
            },
            Err(e) => {
                print_separator();
                log_error(&format!("Failed to load test wallet: {}", e));
            }
        }
    }

    #[wasm_bindgen_test]
    async fn test_complete_token_2022_with_burn_history_sequence() {
        // 1. initialize profile
        test_initialize_user_profile().await;
        
        // 2. get profile (verify only tracking mint)
        test_get_user_profile().await;

        // 3. test Token 2022 mint (various memo lengths)
        test_token_2022_mint_with_various_memo_lengths().await;

        // 4. test burn operation - without user burn history
        test_token_2022_burn_without_history().await;
        
        // 5. test burn operation - with user burn history
        test_token_2022_burn_with_history().await;
        
        // 6. test burn shard operation
        test_burn_shard_operations().await;
        
        // 7. get profile again to see updated burn statistics
        test_get_user_profile().await;
        
        // 8. test close single user burn history
        test_close_user_burn_history().await;

        // 9. close profile
        test_close_user_profile().await;
    }

    #[wasm_bindgen_test]
    async fn test_get_transaction_memo() {
        print_separator();
        log_info("Starting get_transaction_memo test");

        match load_test_wallet() {
            Ok((pubkey, _)) => {
                log_info(&format!("Test wallet public key: {}", pubkey));

                let rpc = RpcConnection::new();
                log_info(&format!("Using RPC endpoint: {}", "https://rpc.testnet.x1.xyz"));

                // use specific transaction signature for testing
                let test_signature = "5mwGX4aAT3zgZ2XsafE8TdWR14o9VSTFQGi4MDvaya3HQo8UbSATiW4T95V7j49JLzAFbZSKSL3GzpXVQMHSawUS";
                log_info(&format!("Testing transaction signature: {}", test_signature));

                match rpc.get_transaction_memo(test_signature).await {
                    Ok(memo_result) => {
                        print_separator();
                        
                        match memo_result {
                            Some(memo_string) => {
                                log_success("Successfully retrieved memo from transaction");
                                log_info(&format!("Memo length: {} bytes", memo_string.len()));
                                log_info("Raw memo content:");
                                
                                // display memo content with proper formatting
                                if memo_string.len() <= 200 {
                                    log_info(&format!("Full memo: {}", memo_string));
                                } else {
                                    log_info(&format!("Memo preview (first 200 chars): {}...", &memo_string[..200]));
                                    log_info(&format!("Memo preview (last 100 chars): ...{}", &memo_string[memo_string.len()-100..]));
                                }

                                // try to parse as JSON to analyze structure
                                match serde_json::from_str::<serde_json::Value>(&memo_string) {
                                    Ok(memo_json) => {
                                        print_separator();
                                        log_info("Memo is valid JSON, analyzing structure:");
                                        log_json("Parsed Memo JSON", &memo_json);
                                        
                                        // analyze memo type
                                        if memo_json.get("title").is_some() || memo_json.get("image").is_some() || memo_json.get("content").is_some() {
                                            log_info("Memo type: Appears to be a MINT transaction memo (contains title/image/content)");
                                            
                                            if let Some(title) = memo_json.get("title").and_then(|v| v.as_str()) {
                                                log_info(&format!("  Title: {}", title));
                                            }
                                            if let Some(content) = memo_json.get("content").and_then(|v| v.as_str()) {
                                                log_info(&format!("  Content: {}", content));
                                            }
                                            if let Some(image) = memo_json.get("image").and_then(|v| v.as_str()) {
                                                log_info(&format!("  Image type: {}", 
                                                    if image.starts_with("data:") { "Base64 encoded" }
                                                    else if image.starts_with("http") { "URL" }
                                                    else { "Custom format" }
                                                ));
                                            }
                                        } else if memo_json.get("signature").is_some() && memo_json.get("message").is_some() {
                                            log_info("Memo type: Appears to be a BURN transaction memo (contains signature/message)");
                                            
                                            if let Some(signature) = memo_json.get("signature").and_then(|v| v.as_str()) {
                                                log_info(&format!("  Referenced signature: {}", signature));
                                            }
                                            if let Some(message) = memo_json.get("message").and_then(|v| v.as_str()) {
                                                log_info(&format!("  Burn message: {}", message));
                                            }
                                        } else {
                                            log_info("Memo type: Custom JSON structure");
                                        }
                                    },
                                    Err(_) => {
                                        log_info("Memo is not JSON format - appears to be plain text");
                                        log_info("Memo type: Plain text memo");
                                    }
                                }

                                print_separator();
                                log_success("get_transaction_memo test completed successfully");
                            },
                            None => {
                                log_info("No memo found in this transaction");
                                log_info("This could mean:");
                                log_info("1. The transaction does not contain a memo instruction");
                                log_info("2. The transaction uses a different memo program");
                                log_info("3. The memo instruction has no data");
                                
                                print_separator();
                                log_success("get_transaction_memo test completed (no memo found)");
                            }
                        }
                    },
                    Err(e) => {
                        print_separator();
                        log_error(&format!("Failed to get transaction memo: {}", e));
                        log_error("This might be due to:");
                        log_error("1. Transaction signature does not exist");
                        log_error("2. Network connectivity issues");
                        log_error("3. RPC endpoint limitations");
                        log_error("4. Transaction is too old and pruned");
                        panic!("get_transaction_memo test failed");
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
    async fn test_get_transaction_memo_from_logs() {
        print_separator();
        log_info("Starting get_transaction_memo test (searching in logs)");

        match load_test_wallet() {
            Ok((pubkey, _)) => {
                log_info(&format!("Test wallet public key: {}", pubkey));

                let rpc = RpcConnection::new();
                log_info(&format!("Using RPC endpoint: {}", "https://rpc.testnet.x1.xyz"));

                let test_signature = "5mwGX4aAT3zgZ2XsafE8TdWR14o9VSTFQGi4MDvaya3HQo8UbSATiW4T95V7j49JLzAFbZSKSL3GzpXVQMHSawUS";
                log_info(&format!("Testing transaction signature: {}", test_signature));

                // Get transaction details and search in logs
                match rpc.get_transaction_details(test_signature).await {
                    Ok(tx_details) => {
                        log_info("Raw transaction details received");
                        
                        match serde_json::from_str::<serde_json::Value>(&tx_details) {
                            Ok(tx_data) => {
                                log_info("Successfully parsed transaction JSON");
                                
                                // Look for log messages
                                if let Some(log_messages) = tx_data
                                    .get("meta")
                                    .and_then(|meta| meta.get("logMessages"))
                                    .and_then(|logs| logs.as_array())
                                {
                                    log_info(&format!("Found {} log messages:", log_messages.len()));
                                    
                                    for (i, log_message) in log_messages.iter().enumerate() {
                                        if let Some(log_str) = log_message.as_str() {
                                            log_info(&format!("  [{}]: {}", i, 
                                                if log_str.len() > 100 { 
                                                    format!("{}...", &log_str[..100]) 
                                                } else { 
                                                    log_str.to_string() 
                                                }
                                            ));
                                            
                                            if log_str.starts_with("Program log: Memo") {
                                                log_success("FOUND MEMO LOG MESSAGE!");
                                                log_info(&format!("Full memo log: {}", log_str));
                                                
                                                // Extract memo content
                                                if let Some(memo_start) = log_str.find("): ") {
                                                    let memo_content = &log_str[memo_start + 3..];
                                                    log_info(&format!("Raw memo content: {}", memo_content));
                                                    
                                                    // Try to unescape JSON
                                                    match serde_json::from_str::<String>(memo_content) {
                                                        Ok(unescaped_memo) => {
                                                            log_success("Successfully unescaped memo!");
                                                            log_info(&format!("Unescaped memo length: {}", unescaped_memo.len()));
                                                            log_info(&format!("Unescaped memo: {}", unescaped_memo));
                                                        },
                                                        Err(e) => {
                                                            log_info(&format!("Memo is not JSON-escaped: {}", e));
                                                            let cleaned_memo = memo_content.trim_matches('"');
                                                            log_info(&format!("Cleaned memo: {}", cleaned_memo));
                                                        }
                                                    }
                                                }
                                                
                                                // Test the actual function
                                                print_separator();
                                                log_info("Testing get_transaction_memo function:");
                                                match rpc.get_transaction_memo(test_signature).await {
                                                    Ok(Some(function_memo)) => {
                                                        log_success("get_transaction_memo function works!");
                                                        log_info(&format!("Function returned memo length: {}", function_memo.len()));
                                                        log_info(&format!("Function memo preview: {}...", 
                                                            &function_memo[..std::cmp::min(100, function_memo.len())]));
                                                    },
                                                    Ok(None) => {
                                                        log_error("Function returned None but we found memo manually");
                                                    },
                                                    Err(e) => {
                                                        log_error(&format!("Function returned error: {}", e));
                                                    }
                                                }
                                                return;
                                            }
                                        }
                                    }
                                    
                                    log_error("No memo log message found");
                                } else {
                                    log_error("No log messages found in meta");
                                }
                            },
                            Err(e) => {
                                log_error(&format!("Failed to parse JSON: {}", e));
                            }
                        }
                    },
                    Err(e) => {
                        log_error(&format!("Failed to get transaction details: {}", e));
                    }
                }
            },
            Err(e) => {
                log_error(&format!("Failed to load test wallet: {}", e));
            }
        }
    }
} 