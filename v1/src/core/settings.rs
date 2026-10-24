use serde::{Deserialize, Serialize};
use web_sys::Storage;

use super::network_config::{self, NetworkType};

const STORAGE_PREFIX: &str = "memo-app.settings.";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum RpcSelection {
    Default,
    Custom,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserSettings {
    pub rpc_selection: RpcSelection,
    pub custom_rpc_url: String,
    /// Buffer percentage to add on top of simulated CU (0-100)
    /// Final CU = simulated_cu * (1 + buffer_percentage / 100)
    pub compute_unit_buffer_percentage: u32,
    /// Compute unit price in micro-lamports (0 = no priority fee)
    pub compute_unit_price_micro_lamports: u64,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            rpc_selection: RpcSelection::Default,
            custom_rpc_url: String::new(),
            compute_unit_buffer_percentage: 1,
            compute_unit_price_micro_lamports: 0,
        }
    }
}

impl UserSettings {
    fn local_storage() -> Option<Storage> {
        web_sys::window()
            .and_then(|win| win.local_storage().ok().flatten())
    }

    fn storage_key(network_type: NetworkType) -> String {
        format!("{}{}", STORAGE_PREFIX, network_type.as_str())
    }

    pub fn load(network_type: NetworkType) -> Option<Self> {
        let storage = Self::local_storage()?;
        let value = storage
            .get_item(&Self::storage_key(network_type))
            .ok()
            .flatten()?;

        serde_json::from_str(&value).ok()
    }

    pub fn save(network_type: NetworkType, settings: &Self) -> Result<(), String> {
        let storage = Self::local_storage().ok_or_else(|| "Local storage not available".to_string())?;
        let serialized = serde_json::to_string(settings)
            .map_err(|e| format!("Failed to serialize settings: {e}"))?;

        storage
            .set_item(&Self::storage_key(network_type), &serialized)
            .map_err(|_| "Failed to write settings to local storage".to_string())
    }

    pub fn custom_rpc_endpoint(&self) -> Option<String> {
        match self.rpc_selection {
            RpcSelection::Custom => {
                let trimmed = self.custom_rpc_url.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            }
            RpcSelection::Default => None,
        }
    }

    /// Get the compute unit buffer multiplier (1.0 + buffer_percentage / 100)
    /// Returns 1.0 if buffer is 0 (no buffer)
    pub fn get_cu_buffer_multiplier(&self) -> f64 {
        if self.compute_unit_buffer_percentage == 0 {
            1.0
        } else {
            let percent = self.compute_unit_buffer_percentage.min(100) as f64;
            1.0 + percent / 100.0
        }
    }

    /// Get the compute unit price in micro-lamports
    /// Returns None if price is 0 (no priority fee)
    pub fn get_cu_price_micro_lamports(&self) -> Option<u64> {
        if self.compute_unit_price_micro_lamports == 0 {
            None
        } else {
            Some(self.compute_unit_price_micro_lamports)
        }
    }
}

pub fn load_settings_for_network(network_type: NetworkType) -> Option<UserSettings> {
    UserSettings::load(network_type)
}

pub fn save_settings_for_network(network_type: NetworkType, settings: &UserSettings) -> Result<(), String> {
    UserSettings::save(network_type, settings)
}

pub fn load_current_network_settings() -> Option<UserSettings> {
    network_config::try_get_network_config()
        .and_then(|config| UserSettings::load(config.network_type))
}

