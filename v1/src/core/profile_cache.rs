use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::core::rpc_profile::UserDisplayInfo;
use web_sys::window;
use log;

/// Profile cache entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileCacheEntry {
    /// User display information
    pub display_info: UserDisplayInfo,
    /// Cache timestamp in milliseconds
    pub cached_at: f64,
    /// Last validation timestamp in milliseconds - used to determine if background refresh is needed
    pub last_validated_at: f64,
}

/// Profile cache manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileCache {
    /// Cache mapping: pubkey -> ProfileCacheEntry
    cache: HashMap<String, ProfileCacheEntry>,
    /// Cache TTL in milliseconds - 30 days
    cache_ttl_ms: f64,
    /// Background refresh interval in milliseconds - 24 hours
    refresh_interval_ms: f64,
}

impl Default for ProfileCache {
    fn default() -> Self {
        Self {
            cache: HashMap::new(),
            cache_ttl_ms: 30.0 * 24.0 * 60.0 * 60.0 * 1000.0, // 30 days
            refresh_interval_ms: 24.0 * 60.0 * 60.0 * 1000.0,  // 24 hours
        }
    }
}

impl ProfileCache {
    const STORAGE_KEY: &'static str = "memo_app_profile_cache_v1";
    
    /// Create a new cache instance
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Load cache from localStorage
    pub fn load_from_storage() -> Self {
        if let Some(window) = window() {
            if let Ok(Some(storage)) = window.local_storage() {
                if let Ok(Some(cached_str)) = storage.get_item(Self::STORAGE_KEY) {
                    match serde_json::from_str::<ProfileCache>(&cached_str) {
                        Ok(cache) => {
                            log::info!("Loaded profile cache from localStorage with {} entries", cache.cache.len());
                            return cache;
                        },
                        Err(e) => {
                            log::warn!("Failed to parse profile cache from localStorage: {}", e);
                        }
                    }
                }
            }
        }
        
        log::info!("Creating new profile cache");
        Self::new()
    }
    
    /// Save cache to localStorage
    pub fn save_to_storage(&self) {
        if let Some(window) = window() {
            if let Ok(Some(storage)) = window.local_storage() {
                match serde_json::to_string(self) {
                    Ok(cache_str) => {
                        if let Err(e) = storage.set_item(Self::STORAGE_KEY, &cache_str) {
                            log::error!("Failed to save profile cache to localStorage: {:?}", e);
                        } else {
                            log::debug!("Saved profile cache with {} entries to localStorage", self.cache.len());
                        }
                    },
                    Err(e) => {
                        log::error!("Failed to serialize profile cache: {}", e);
                    }
                }
            }
        }
    }
    
    /// Get current timestamp in milliseconds
    fn get_current_timestamp() -> f64 {
        if let Some(_window) = window() {
            web_sys::js_sys::Date::now()
        } else {
            0.0
        }
    }
    
    /// Check if a cache entry is expired
    fn is_expired(&self, entry: &ProfileCacheEntry) -> bool {
        let now = Self::get_current_timestamp();
        (now - entry.cached_at) > self.cache_ttl_ms
    }
    
    /// Check if a cache entry needs background refresh
    fn needs_refresh(&self, entry: &ProfileCacheEntry) -> bool {
        let now = Self::get_current_timestamp();
        (now - entry.last_validated_at) > self.refresh_interval_ms
    }
    
    /// Get user display info from cache
    /// Returns: (Option<UserDisplayInfo>, needs_refresh)
    pub fn get(&mut self, pubkey: &str) -> (Option<UserDisplayInfo>, bool) {
        // Clean up expired entries
        if let Some(entry) = self.cache.get(pubkey) {
            if self.is_expired(entry) {
                log::debug!("Cache entry for {} is expired, removing", pubkey);
                self.cache.remove(pubkey);
                self.save_to_storage();
                return (None, false);
            }
            
            let needs_refresh = self.needs_refresh(entry);
            if needs_refresh {
                log::debug!("Cache entry for {} needs refresh ({}h since last validation)", 
                          pubkey, 
                          (Self::get_current_timestamp() - entry.last_validated_at) / (60.0 * 60.0 * 1000.0));
            }
            
            return (Some(entry.display_info.clone()), needs_refresh);
        }
        
        (None, false)
    }
    
    /// Get user display info from cache in batch
    /// Returns: (HashMap<pubkey, UserDisplayInfo>, Vec<pubkeys that need fetching>)
    pub fn get_batch(&mut self, pubkeys: &[&str]) -> (HashMap<String, UserDisplayInfo>, Vec<String>) {
        let mut cached_results = HashMap::new();
        let mut needs_fetch = Vec::new();
        let mut needs_refresh_list = Vec::new();
        
        for pubkey in pubkeys {
            let (cached_info, needs_refresh) = self.get(pubkey);
            
            match cached_info {
                Some(info) => {
                    cached_results.insert(pubkey.to_string(), info);
                    if needs_refresh {
                        needs_refresh_list.push(pubkey.to_string());
                    }
                },
                None => {
                    needs_fetch.push(pubkey.to_string());
                }
            }
        }
        
        // Prioritize pubkeys that need immediate fetching, then those that need refresh
        needs_fetch.extend(needs_refresh_list);
        
        log::debug!("Batch get: {} cached, {} need fetch/refresh", cached_results.len(), needs_fetch.len());
        
        (cached_results, needs_fetch)
    }
    
    /// Add user display info to cache
    pub fn set(&mut self, pubkey: String, display_info: UserDisplayInfo) {
        let now = Self::get_current_timestamp();
        
        let entry = ProfileCacheEntry {
            display_info,
            cached_at: now,
            last_validated_at: now,
        };
        
        self.cache.insert(pubkey.clone(), entry);
        log::debug!("Cached profile for {}", pubkey);
        
        self.save_to_storage();
    }
    
    /// Add user display info to cache in batch
    pub fn set_batch(&mut self, display_infos: Vec<UserDisplayInfo>) {
        let now = Self::get_current_timestamp();
        
        for display_info in display_infos {
            let entry = ProfileCacheEntry {
                display_info: display_info.clone(),
                cached_at: now,
                last_validated_at: now,
            };
            
            self.cache.insert(display_info.pubkey.clone(), entry);
        }
        
        log::info!("Batch cached {} profiles", self.cache.len());
        self.save_to_storage();
    }
    
    /// Update validation timestamp of a cache entry (after background refresh)
    pub fn update_validation_time(&mut self, pubkey: &str) {
        if let Some(entry) = self.cache.get_mut(pubkey) {
            entry.last_validated_at = Self::get_current_timestamp();
            log::debug!("Updated validation time for {}", pubkey);
            self.save_to_storage();
        }
    }
    
    /// Remove a user from cache
    pub fn remove(&mut self, pubkey: &str) {
        if self.cache.remove(pubkey).is_some() {
            log::debug!("Removed profile cache for {}", pubkey);
            self.save_to_storage();
        }
    }
    
    /// Clear all cache
    pub fn clear(&mut self) {
        self.cache.clear();
        log::info!("Cleared all profile cache");
        self.save_to_storage();
    }
    
    /// Clean up expired cache entries
    pub fn cleanup_expired(&mut self) -> usize {
        let before_count = self.cache.len();
        
        let expired_keys: Vec<String> = self.cache.iter()
            .filter(|(_, entry)| self.is_expired(entry))
            .map(|(key, _)| key.clone())
            .collect();
        
        for key in expired_keys {
            self.cache.remove(&key);
        }
        
        let removed_count = before_count - self.cache.len();
        
        if removed_count > 0 {
            log::info!("Cleaned up {} expired profile cache entries", removed_count);
            self.save_to_storage();
        }
        
        removed_count
    }
    
    /// Get cache statistics
    pub fn get_stats(&self) -> ProfileCacheStats {
        let _now = Self::get_current_timestamp();
        let mut expired_count = 0;
        let mut needs_refresh_count = 0;
        
        for entry in self.cache.values() {
            if self.is_expired(entry) {
                expired_count += 1;
            } else if self.needs_refresh(entry) {
                needs_refresh_count += 1;
            }
        }
        
        ProfileCacheStats {
            total_entries: self.cache.len(),
            expired_entries: expired_count,
            needs_refresh_entries: needs_refresh_count,
            cache_ttl_hours: (self.cache_ttl_ms / (60.0 * 60.0 * 1000.0)) as u32,
            refresh_interval_hours: (self.refresh_interval_ms / (60.0 * 60.0 * 1000.0)) as u32,
        }
    }
}

/// Profile cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileCacheStats {
    pub total_entries: usize,
    pub expired_entries: usize,
    pub needs_refresh_entries: usize,
    pub cache_ttl_hours: u32,
    pub refresh_interval_hours: u32,
}
