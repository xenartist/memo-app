use tauri::{
    plugin::{Builder, TauriPlugin, PluginHandle},
    Runtime, AppHandle, Manager,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedVaultResponse {
    #[serde(flatten)]
    pub data: serde_json::Value,
}

// Commands that bridge to Android plugin
#[tauri::command]
async fn is_available<R: Runtime>(app: AppHandle<R>) -> Result<SeedVaultResponse, String> {
    #[cfg(mobile)]
    {
        let plugin = app.state::<PluginHandle<R>>();
        plugin.run_mobile_plugin("isAvailable", ())
            .map_err(|e| format!("Failed to check availability: {}", e))
            .map(|data| SeedVaultResponse { data })
    }
    #[cfg(not(mobile))]
    {
        let _ = app;
        Err("Seed Vault is only available on mobile".to_string())
    }
}

#[tauri::command]
async fn authorize_seed<R: Runtime>(app: AppHandle<R>) -> Result<SeedVaultResponse, String> {
    #[cfg(mobile)]
    {
        let plugin = app.state::<PluginHandle<R>>();
        plugin.run_mobile_plugin("authorizeSeed", ())
            .map_err(|e| format!("Failed to authorize seed: {}", e))
            .map(|data| SeedVaultResponse { data })
    }
    #[cfg(not(mobile))]
    {
        let _ = app;
        Err("Seed Vault is only available on mobile".to_string())
    }
}

#[tauri::command]
async fn get_authorized_seeds<R: Runtime>(app: AppHandle<R>) -> Result<SeedVaultResponse, String> {
    #[cfg(mobile)]
    {
        let plugin = app.state::<PluginHandle<R>>();
        plugin.run_mobile_plugin("getAuthorizedSeeds", ())
            .map_err(|e| format!("Failed to get authorized seeds: {}", e))
            .map(|data| SeedVaultResponse { data })
    }
    #[cfg(not(mobile))]
    {
        let _ = app;
        Err("Seed Vault is only available on mobile".to_string())
    }
}

#[derive(Serialize, Deserialize)]
struct GetAccountsPayload {
    #[serde(rename = "authToken")]
    auth_token: i64,
}

#[tauri::command]
async fn get_accounts<R: Runtime>(app: AppHandle<R>, auth_token: i64) -> Result<SeedVaultResponse, String> {
    #[cfg(mobile)]
    {
        let plugin = app.state::<PluginHandle<R>>();
        let payload = GetAccountsPayload { auth_token };
        plugin.run_mobile_plugin("getAccounts", payload)
            .map_err(|e| format!("Failed to get accounts: {}", e))
            .map(|data| SeedVaultResponse { data })
    }
    #[cfg(not(mobile))]
    {
        let _ = (app, auth_token);
        Err("Seed Vault is only available on mobile".to_string())
    }
}

#[derive(Serialize, Deserialize)]
struct SignTransactionPayload {
    #[serde(rename = "authToken")]
    auth_token: i64,
    transaction: String,
    #[serde(rename = "derivationPath", skip_serializing_if = "Option::is_none")]
    derivation_path: Option<String>,
}

#[tauri::command]
async fn sign_transaction<R: Runtime>(
    app: AppHandle<R>,
    auth_token: i64,
    transaction: String,
    derivation_path: Option<String>,
) -> Result<SeedVaultResponse, String> {
    #[cfg(mobile)]
    {
        let plugin = app.state::<PluginHandle<R>>();
        let payload = SignTransactionPayload {
            auth_token,
            transaction,
            derivation_path,
        };
        plugin.run_mobile_plugin("signTransaction", payload)
            .map_err(|e| format!("Failed to sign transaction: {}", e))
            .map(|data| SeedVaultResponse { data })
    }
    #[cfg(not(mobile))]
    {
        let _ = (app, auth_token, transaction, derivation_path);
        Err("Seed Vault is only available on mobile".to_string())
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::<R>::new("seed-vault")
        .invoke_handler(tauri::generate_handler![
            is_available,
            authorize_seed,
            get_authorized_seeds,
            get_accounts,
            sign_transaction
        ])
        .build()
}
