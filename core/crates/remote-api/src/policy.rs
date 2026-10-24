//! Da quale via arriva una richiesta e cosa le e' permesso fare.
//!
//! Il trasporto si ricava dalle intestazioni che il tunnel o il proxy aggiungono:
//! chi arriva da Cloudflare non puo' toglierle, quindi non puo' farsi passare per rete locale.

use std::collections::HashSet;

use axum::http::HeaderMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Transport {
    Lan,
    Tailscale,
    Cloudflare,
}

#[derive(Clone, Debug)]
pub struct TxPolicy {
    allowed: HashSet<Transport>,
    live_tx: bool,
}

impl TxPolicy {
    pub fn new(allowed: impl IntoIterator<Item = Transport>) -> Self {
        TxPolicy { allowed: allowed.into_iter().collect(), live_tx: false }
    }

    /// Predefinito sicuro: si trasmette da rete locale e Tailscale, non da internet.
    pub fn default_safe() -> Self {
        Self::new([Transport::Lan, Transport::Tailscale])
    }

    /// Abilita o chiude la trasmissione per tutti: predefinito **chiusa**.
    /// Un Reply fa trasmettere JTDX davvero, quindi va aperta solo con una scelta esplicita.
    pub fn with_live_tx(mut self, live: bool) -> Self {
        self.live_tx = live;
        self
    }

    pub fn live_tx(&self) -> bool {
        self.live_tx
    }

    pub fn allows(&self, transport: Transport) -> bool {
        self.allowed.contains(&transport)
    }
}

pub fn transport_of(headers: &HeaderMap) -> Transport {
    if headers.contains_key("cf-ray") || headers.contains_key("cf-connecting-ip") {
        Transport::Cloudflare
    } else if headers.contains_key("tailscale-user-login") || headers.contains_key("tailscale-user-name") {
        Transport::Tailscale
    } else {
        Transport::Lan
    }
}
