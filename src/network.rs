//! The networks the app can talk to, and the chosen one (kept in localStorage).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Network {
    Mainnet,
    Testnet,
}

impl Network {
    pub const ALL: [Network; 2] = [Network::Mainnet, Network::Testnet];

    pub fn name(self) -> &'static str {
        match self {
            Network::Mainnet => "Mainnet",
            Network::Testnet => "Testnet",
        }
    }

    fn key(self) -> &'static str {
        match self {
            Network::Mainnet => "mainnet",
            Network::Testnet => "testnet",
        }
    }

    pub fn rpc_url(self) -> &'static str {
        match self {
            Network::Mainnet => "https://rpc.mainnet.x1.xyz",
            Network::Testnet => "https://rpc.testnet.x1.xyz",
        }
    }

    pub fn mint_program_id(self) -> &'static str {
        match self {
            Network::Mainnet => "8iq6zqaEVcfaym2u8t939PAN5jmfPVc6Z333RuxKTTZX",
            Network::Testnet => "A31a17bhgQyRQygeZa1SybytjbCdjMpu6oPr9M3iQWzy",
        }
    }

    pub fn token_mint(self) -> &'static str {
        match self {
            Network::Mainnet => "memoX1sJsBY6od7CfQ58XooRALwnocAZen4L7mW1ick",
            Network::Testnet => "HLCoc7wNDavNMfWWw2Bwd7U7A24cesuhBSNkxZgvZm1",
        }
    }

    pub fn tx_url(self, signature: &str) -> String {
        match self {
            Network::Mainnet => format!("https://explorer.x1.xyz/tx/{}", signature),
            Network::Testnet => format!("https://explorer.testnet.x1.xyz/tx/{}", signature),
        }
    }

    fn storage() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok()?
    }

    /// The network chosen last time, Mainnet by default
    pub fn load() -> Network {
        let saved = Self::storage().and_then(|s| s.get_item("memo-network").ok().flatten());
        Self::ALL
            .into_iter()
            .find(|n| saved.as_deref() == Some(n.key()))
            .unwrap_or(Network::Mainnet)
    }

    pub fn save(self) {
        if let Some(storage) = Self::storage() {
            let _ = storage.set_item("memo-network", self.key());
        }
    }
}

pub const TOKEN_2022_PROGRAM_ID: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
