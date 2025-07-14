use super::rpc_base::{RpcConnection, RpcError};
use serde::{Serialize, Deserialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use js_sys::{Array, Promise, Reflect, Object};
use web_sys::window;

/// IndexedDB storage error types
#[derive(Debug)]
pub enum IndexedDBError {
    DatabaseError(String),
    SerializationError(String),
    QueryError(String),
    NotFound,
    InvalidParameter(String),
    NotSupported,
}

impl std::fmt::Display for IndexedDBError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexedDBError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            IndexedDBError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            IndexedDBError::QueryError(msg) => write!(f, "Query error: {}", msg),
            IndexedDBError::NotFound => write!(f, "Record not found"),
            IndexedDBError::InvalidParameter(msg) => write!(f, "Invalid parameter: {}", msg),
            IndexedDBError::NotSupported => write!(f, "Browser does not support IndexedDB"),
        }
    }
}

impl std::error::Error for IndexedDBError {}

/// Transaction record structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionRecord {
    /// Transaction signature (primary key)
    pub signature: String,
    /// Slot number
    pub slot: u64,
    /// Block timestamp (Unix seconds)
    pub block_time: Option<i64>,
    /// Sender address
    pub from_address: String,
    /// Receiver address  
    pub to_address: String,
    /// Contract address
    pub contract_address: String,
    /// Transaction amount
    pub amount: u64,
    /// Transaction fee
    pub fee: u64,
    /// Whether successful
    pub success: bool,
    /// Complete transaction data (JSON string)
    pub transaction_data: String,
    /// Local storage time
    pub stored_at: f64,
}

/// Query options
#[derive(Debug, Clone)]
pub struct QueryOptions {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub success_only: Option<bool>,
}

impl Default for QueryOptions {
    fn default() -> Self {
        Self {
            limit: None,
            offset: None,
            start_time: None,
            end_time: None,
            success_only: None,
        }
    }
}

/// Transaction manager that directly operates IndexedDB using JS
pub struct TransactionManager {
    db_name: String,
    db_version: u32,
    contract_address: String,
    rpc: RpcConnection,
}

impl TransactionManager {
    /// Create new transaction manager
    pub fn new(contract_address: String, rpc: RpcConnection) -> Self {
        Self {
            db_name: "contract_transactions".to_string(),
            db_version: 1,
            contract_address,
            rpc,
        }
    }

    /// Initialize database
    pub async fn initialize(&self) -> Result<(), IndexedDBError> {
        self.get_or_create_database().await?;
        Ok(())
    }

    /// Get or create database using JS eval
    async fn get_or_create_database(&self) -> Result<JsValue, IndexedDBError> {
        let window = window().ok_or(IndexedDBError::NotSupported)?;
        
        // Check IndexedDB support
        let indexeddb_check = js_sys::eval("typeof window.indexedDB !== 'undefined'")
            .map_err(|_| IndexedDBError::NotSupported)?;
        
        if !indexeddb_check.as_bool().unwrap_or(false) {
            return Err(IndexedDBError::NotSupported);
        }

        // Open database using JavaScript code
        let js_code = format!(r#"
            (function() {{
                return new Promise((resolve, reject) => {{
                    const request = window.indexedDB.open('{}', {});
                    
                    request.onerror = () => reject(request.error);
                    request.onsuccess = () => resolve(request.result);
                    
                    request.onupgradeneeded = (event) => {{
                        const db = event.target.result;
                        
                        // Create object store
                        if (!db.objectStoreNames.contains('transactions')) {{
                            const store = db.createObjectStore('transactions', {{ keyPath: 'signature' }});
                            
                            // Create indexes
                            store.createIndex('from_address_idx', 'from_address');
                            store.createIndex('to_address_idx', 'to_address');
                            store.createIndex('block_time_idx', 'block_time');
                            store.createIndex('contract_address_idx', 'contract_address');
                        }}
                    }};
                }});
            }})()
        "#, self.db_name, self.db_version);

        let promise = js_sys::eval(&js_code)
            .map_err(|e| IndexedDBError::DatabaseError(format!("Failed to execute JS code: {:?}", e)))?;

        let future = JsFuture::from(Promise::from(promise));
        future.await
            .map_err(|e| IndexedDBError::DatabaseError(format!("Failed to open database: {:?}", e)))
    }

    /// Store single transaction record
    pub async fn store_transaction(&self, transaction: TransactionRecord) -> Result<(), IndexedDBError> {
        let serialized = serde_json::to_string(&transaction)
            .map_err(|e| IndexedDBError::SerializationError(format!("Serialization failed: {:?}", e)))?;

        let js_code = format!(r#"
            (function() {{
                return new Promise(async (resolve, reject) => {{
                    try {{
                        const request = window.indexedDB.open('{}', {});
                        
                        request.onsuccess = () => {{
                            const db = request.result;
                            const tx = db.transaction(['transactions'], 'readwrite');
                            const store = tx.objectStore('transactions');
                            
                            const data = JSON.parse(`{}`);
                            const putRequest = store.put(data);
                            
                            putRequest.onsuccess = () => resolve(true);
                            putRequest.onerror = () => reject(putRequest.error);
                        }};
                        
                        request.onerror = () => reject(request.error);
                    }} catch (error) {{
                        reject(error);
                    }}
                }});
            }})()
        "#, self.db_name, self.db_version, serialized.replace('`', r#"\`"#));

        let promise = js_sys::eval(&js_code)
            .map_err(|e| IndexedDBError::DatabaseError(format!("Failed to execute storage code: {:?}", e)))?;

        let future = JsFuture::from(Promise::from(promise));
        future.await
            .map_err(|e| IndexedDBError::DatabaseError(format!("Storage failed: {:?}", e)))?;

        Ok(())
    }

    /// Store transaction records in batch
    pub async fn store_transactions_batch(&self, transactions: Vec<TransactionRecord>) -> Result<(), IndexedDBError> {
        let serialized_array: Result<Vec<String>, _> = transactions.iter()
            .map(|tx| serde_json::to_string(tx))
            .collect();
        
        let serialized_array = serialized_array
            .map_err(|e| IndexedDBError::SerializationError(format!("Batch serialization failed: {:?}", e)))?;
        
        let json_array = format!("[{}]", serialized_array.join(","));

        let js_code = format!(r#"
            (function() {{
                return new Promise(async (resolve, reject) => {{
                    try {{
                        const request = window.indexedDB.open('{}', {});
                        
                        request.onsuccess = () => {{
                            const db = request.result;
                            const tx = db.transaction(['transactions'], 'readwrite');
                            const store = tx.objectStore('transactions');
                            
                            const dataArray = {};
                            let completed = 0;
                            const total = dataArray.length;
                            
                            if (total === 0) {{
                                resolve(true);
                                return;
                            }}
                            
                            dataArray.forEach(data => {{
                                const putRequest = store.put(data);
                                putRequest.onsuccess = () => {{
                                    completed++;
                                    if (completed === total) resolve(true);
                                }};
                                putRequest.onerror = () => reject(putRequest.error);
                            }});
                        }};
                        
                        request.onerror = () => reject(request.error);
                    }} catch (error) {{
                        reject(error);
                    }}
                }});
            }})()
        "#, self.db_name, self.db_version, json_array);

        let promise = js_sys::eval(&js_code)
            .map_err(|e| IndexedDBError::DatabaseError(format!("Failed to execute batch storage code: {:?}", e)))?;

        let future = JsFuture::from(Promise::from(promise));
        future.await
            .map_err(|e| IndexedDBError::DatabaseError(format!("Batch storage failed: {:?}", e)))?;

        Ok(())
    }

    /// Get all related transactions for wallet address
    pub async fn get_transactions_for_address(&self, address: &str) -> Result<Vec<TransactionRecord>, IndexedDBError> {
        let js_code = format!(r#"
            (function() {{
                return new Promise(async (resolve, reject) => {{
                    try {{
                        const request = window.indexedDB.open('{}', {});
                        
                        request.onsuccess = () => {{
                            const db = request.result;
                            const tx = db.transaction(['transactions'], 'readonly');
                            const store = tx.objectStore('transactions');
                            
                            const getAllRequest = store.getAll();
                            
                            getAllRequest.onsuccess = () => {{
                                const allData = getAllRequest.result;
                                const filteredData = allData.filter(item => 
                                    item.from_address === '{}' || item.to_address === '{}'
                                );
                                
                                // Sort by time descending
                                filteredData.sort((a, b) => {{
                                    const timeA = a.block_time || 0;
                                    const timeB = b.block_time || 0;
                                    return timeB - timeA;
                                }});
                                
                                resolve(JSON.stringify(filteredData));
                            }};
                            
                            getAllRequest.onerror = () => reject(getAllRequest.error);
                        }};
                        
                        request.onerror = () => reject(request.error);
                    }} catch (error) {{
                        reject(error);
                    }}
                }});
            }})()
        "#, self.db_name, self.db_version, address, address);

        let promise = js_sys::eval(&js_code)
            .map_err(|e| IndexedDBError::QueryError(format!("Failed to execute query code: {:?}", e)))?;

        let future = JsFuture::from(Promise::from(promise));
        let result = future.await
            .map_err(|e| IndexedDBError::QueryError(format!("Query failed: {:?}", e)))?;

        let json_str = result.as_string()
            .ok_or_else(|| IndexedDBError::QueryError("Result is not a string".to_string()))?;

        let transactions: Vec<TransactionRecord> = serde_json::from_str(&json_str)
            .map_err(|e| IndexedDBError::SerializationError(format!("Deserialization failed: {:?}", e)))?;

        Ok(transactions)
    }

    /// Get all transaction records
    pub async fn get_all_transactions(&self) -> Result<Vec<TransactionRecord>, IndexedDBError> {
        let js_code = format!(r#"
            (function() {{
                return new Promise(async (resolve, reject) => {{
                    try {{
                        const request = window.indexedDB.open('{}', {});
                        
                        request.onsuccess = () => {{
                            const db = request.result;
                            const tx = db.transaction(['transactions'], 'readonly');
                            const store = tx.objectStore('transactions');
                            
                            const getAllRequest = store.getAll();
                            
                            getAllRequest.onsuccess = () => {{
                                const allData = getAllRequest.result;
                                
                                // Sort by time descending
                                allData.sort((a, b) => {{
                                    const timeA = a.block_time || 0;
                                    const timeB = b.block_time || 0;
                                    return timeB - timeA;
                                }});
                                
                                resolve(JSON.stringify(allData));
                            }};
                            
                            getAllRequest.onerror = () => reject(getAllRequest.error);
                        }};
                        
                        request.onerror = () => reject(request.error);
                    }} catch (error) {{
                        reject(error);
                    }}
                }});
            }})()
        "#, self.db_name, self.db_version);

        let promise = js_sys::eval(&js_code)
            .map_err(|e| IndexedDBError::QueryError(format!("Failed to execute query code: {:?}", e)))?;

        let future = JsFuture::from(Promise::from(promise));
        let result = future.await
            .map_err(|e| IndexedDBError::QueryError(format!("Query failed: {:?}", e)))?;

        let json_str = result.as_string()
            .ok_or_else(|| IndexedDBError::QueryError("Result is not a string".to_string()))?;

        let transactions: Vec<TransactionRecord> = serde_json::from_str(&json_str)
            .map_err(|e| IndexedDBError::SerializationError(format!("Deserialization failed: {:?}", e)))?;

        Ok(transactions)
    }

    /// Get transaction count
    pub async fn get_transaction_count(&self) -> Result<u32, IndexedDBError> {
        let js_code = format!(r#"
            (function() {{
                return new Promise(async (resolve, reject) => {{
                    try {{
                        const request = window.indexedDB.open('{}', {});
                        
                        request.onsuccess = () => {{
                            const db = request.result;
                            const tx = db.transaction(['transactions'], 'readonly');
                            const store = tx.objectStore('transactions');
                            
                            const countRequest = store.count();
                            
                            countRequest.onsuccess = () => {{
                                resolve(countRequest.result);
                            }};
                            
                            countRequest.onerror = () => reject(countRequest.error);
                        }};
                        
                        request.onerror = () => reject(request.error);
                    }} catch (error) {{
                        reject(error);
                    }}
                }});
            }})()
        "#, self.db_name, self.db_version);

        let promise = js_sys::eval(&js_code)
            .map_err(|e| IndexedDBError::QueryError(format!("Failed to execute count code: {:?}", e)))?;

        let future = JsFuture::from(Promise::from(promise));
        let result = future.await
            .map_err(|e| IndexedDBError::QueryError(format!("Count failed: {:?}", e)))?;

        let count = result.as_f64()
            .ok_or_else(|| IndexedDBError::QueryError("Count result is not a number".to_string()))?;

        Ok(count as u32)
    }

    /// Clear all transactions
    pub async fn clear_all_transactions(&self) -> Result<(), IndexedDBError> {
        let js_code = format!(r#"
            (function() {{
                return new Promise(async (resolve, reject) => {{
                    try {{
                        const request = window.indexedDB.open('{}', {});
                        
                        request.onsuccess = () => {{
                            const db = request.result;
                            const tx = db.transaction(['transactions'], 'readwrite');
                            const store = tx.objectStore('transactions');
                            
                            const clearRequest = store.clear();
                            
                            clearRequest.onsuccess = () => resolve(true);
                            clearRequest.onerror = () => reject(clearRequest.error);
                        }};
                        
                        request.onerror = () => reject(request.error);
                    }} catch (error) {{
                        reject(error);
                    }}
                }});
            }})()
        "#, self.db_name, self.db_version);

        let promise = js_sys::eval(&js_code)
            .map_err(|e| IndexedDBError::DatabaseError(format!("Failed to execute clear code: {:?}", e)))?;

        let future = JsFuture::from(Promise::from(promise));
        future.await
            .map_err(|e| IndexedDBError::DatabaseError(format!("Clear failed: {:?}", e)))?;

        Ok(())
    }

    /// Sync latest transactions from RPC (example implementation)
    pub async fn sync_latest_transactions(&self, limit: Option<u32>) -> Result<u32, IndexedDBError> {
        log::info!("Starting transaction sync, limit: {:?}", limit);
        
        // TODO: Implement actual sync logic
        // 1. Call self.rpc.get_signatures_for_address(&self.contract_address, limit)
        // 2. For each signature call self.rpc.get_transaction_details(signature)
        // 3. Parse transaction data to create TransactionRecord
        // 4. Call self.store_transaction(record)
        
        Ok(0) // Temporary return 0
    }
} 