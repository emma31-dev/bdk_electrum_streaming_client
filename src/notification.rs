//! Defines parsed Electrum server notifications.
//!
//! This module provides the [`Notification`] enum, which categorizes incoming Electrum
//! notifications based on their method type. Currently supported variants include:
//!
//! - [`Notification::Header`] for `"blockchain.headers.subscribe"`
//! - [`Notification::ScriptHash`] for `"blockchain.scripthash.subscribe"`
//! - [`Notification::OutPoint`] for `"blockchain.outpoint.subscribe"`
//! - `Notification::SpSubscribe` for `"blockchain.silentpayments.subscribe"` (requires the `frigate` feature)
//! - [`Notification::Unknown`] for unrecognized or unsupported methods
//!
//! Each variant wraps a struct that contains the deserialized payload for that notification type.
//! Use [`Notification::new`] to construct a typed [`Notification`] from a raw JSON-RPC notification.
//!
//! This is useful for higher-level consumers who want to match against known server-side events.

use serde::Deserialize;

use crate::{response, ElectrumScriptHash, ElectrumScriptStatus, RawNotification};

/// A parsed Electrum server notification.
///
/// This enum represents server-initiated messages received outside the context of a request.
/// Use [`Notification::new`] to convert a raw JSON-RPC notification into a typed variant.
///
/// Known notification types are parsed into structured variants. Unknown or unsupported types are
/// preserved as-is in [`Notification::Unknown`].
#[derive(Debug, Clone)]
pub enum Notification {
    /// A notification from `"blockchain.headers.subscribe"` indicating a new best block header.
    Header(HeaderNotification),

    /// A notification from `"blockchain.scripthash.subscribe"` indicating a change in script
    /// status.
    ScriptHash(ScriptHashNotification),

    /// A notification from `"blockchain.outpoint.subscribe"` indicating a change in outpoint
    /// status.
    OutPoint(OutPointNotification),

    /// A notification from `"blockchain.silentpayments.subscribe"` indicating a new history
    /// of transactions
    #[cfg(feature = "frigate")]
    SpSubscribe(SpNotification),

    /// A catch-all for notifications with unrecognized methods.
    ///
    /// The original [`RawNotification`] is preserved for downstream inspection.
    Unknown(UnknownNotification),
}

impl Notification {
    /// Attempts to parse a [`RawNotification`] into a typed [`Notification`] variant.
    ///
    /// Returns `Ok` with a known variant if the method is recognized, or [`Notification::Unknown`]
    /// otherwise.
    pub fn new(raw: &RawNotification) -> Result<Self, serde_json::Error> {
        let RawNotification { method, params, .. } = raw;
        match method.as_ref() {
            "blockchain.headers.subscribe" => {
                HeaderNotification::deserialize(params).map(Notification::Header)
            }
            "blockchain.scripthash.subscribe" => {
                ScriptHashNotification::deserialize(params).map(Notification::ScriptHash)
            }
            "blockchain.outpoint.subscribe" => {
                OutPointNotification::deserialize(params).map(Notification::OutPoint)
            }
            #[cfg(feature = "frigate")]
            "blockchain.silentpayments.subscribe" => {
                SpNotification::deserialize(params).map(Notification::SpSubscribe)
            }
            _ => Ok(Notification::Unknown(raw.clone())),
        }
    }
}

/// A type alias for unrecognized Electrum notifications.
///
/// Used when the method name is not handled explicitly by the client. The raw JSON-RPC
/// notification is preserved without interpretation.
pub type UnknownNotification = RawNotification;

/// A notification indicating the current best block header on the chain tip.
///
/// Corresponds to the `"blockchain.headers.subscribe"` Electrum notification method.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct HeaderNotification {
    param_0: response::HeadersSubscribeResp,
}

impl HeaderNotification {
    /// Returns the height of the new best block.
    pub fn height(&self) -> u32 {
        self.param_0.height
    }

    /// Returns a reference to the new best block header.
    pub fn header(&self) -> &bitcoin::block::Header {
        &self.param_0.header
    }
}
/// A notification indicating a change in the status of a specific script hash.
///
/// Corresponds to the `"blockchain.scripthash.subscribe"` Electrum notification method.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ScriptHashNotification {
    param_0: ElectrumScriptHash,
    param_1: Option<ElectrumScriptStatus>,
}

impl ScriptHashNotification {
    /// Returns the script hash associated with the notification.
    pub fn script_hash(&self) -> ElectrumScriptHash {
        self.param_0
    }

    /// Returns the new script status associated with the script hash.
    pub fn script_status(&self) -> Option<ElectrumScriptStatus> {
        self.param_1
    }
}

/// A notification indicating a change in the status of a specific outpoint.
///
/// Corresponds to the `"blockchain.outpoint.subscribe"` Electrum notification method.
/// The notification carries three params: `[tx_hash, txout_idx, status]`.
///
/// Available in Electrum protocol v1.7.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct OutPointNotification {
    /// The TXID of the funding transaction identifying the outpoint.
    param_0: bitcoin::Txid,
    /// The output index identifying the outpoint.
    param_1: u32,
    /// The current status of the outpoint.
    param_2: response::OutPointNotificationResp,
}

impl OutPointNotification {
    /// Returns the outpoint (txid + vout index) this notification refers to.
    pub fn outpoint(&self) -> bitcoin::OutPoint {
        bitcoin::OutPoint::new(self.param_0, self.param_1)
    }

    /// Returns the full status payload of the notification.
    pub fn status(&self) -> &response::OutPointNotificationResp {
        &self.param_2
    }

    /// Returns the funder height associated with the outpoint, if a funding tx exists.
    pub fn funder_height(&self) -> Option<i64> {
        self.param_2.funder_height
    }

    /// Returns the spender txid associated with the outpoint, if a spending tx exists.
    pub fn spender_txid(&self) -> Option<bitcoin::Txid> {
        self.param_2.spender_txid
    }

    /// Returns the spender height associated with the outpoint, if a spending tx exists.
    pub fn spender_height(&self) -> Option<i64> {
        self.param_2.spender_height
    }
}

/// An update for a Silent Payments subscription.
///
/// Corresponds to `"blockchain.silentpayments.subscribe"` Frigate Electrum notification method.
#[cfg(feature = "frigate")]
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SpNotification {
    /// Identifies the subscription to which this notification belongs.
    pub subscription: response::SpSubscribeResp,

    /// Historical scan progress from `0.0` through `1.0`.
    ///
    /// A value of `1.0` indicates that the scan is up to date.
    pub progress: f32,

    /// Transactions discovered by the scan.
    ///
    /// Confirmed transactions are ordered by block height.
    pub history: Vec<response::TxTweak>,
}
