//! Per-player ProjectE data.
//!
//! Every attachment operation is fallible and every balance change is checked.
//! Callers must handle the returned error: a swallowed attachment failure would
//! let a purchase succeed without debiting EMC, or credit a deposit that never
//! reached storage. Malformed saved data is rejected instead of being replaced
//! by a default value.

use std::collections::BTreeSet;

use mod_api::{AttachmentCodec, AttachmentError, AttachmentType, Compound, Holder, Tag};

/// A player's transmutation knowledge and stored EMC.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlayerData {
    /// Server-authoritative, non-negative balance.
    pub emc: i64,
    /// Fully qualified item ids the player has learned for transmutation.
    pub learned: BTreeSet<String>,
    /// The item pinned in the transmutation table's lock slot, if any.
    /// An id must be fully qualified; `None` means the lock is cleared.
    pub lock: Option<String>,
    /// Tome of Knowledge: every item with an EMC value is known.
    pub full_knowledge: bool,
}

/// The Tome of Knowledge. It has no EMC of its own but grants full knowledge.
pub const TOME_ID: &str = "projecte:tome";

/// A data-operation failure: either the attachment store refused the operation
/// or the request itself was invalid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataError {
    /// The attachment store could not read or write the value.
    Attachment(AttachmentError),
    /// A negative amount is not a valid EMC delta; use a withdrawal instead.
    NegativeAmount,
    /// Adding the amount would overflow the balance type.
    BalanceOverflow,
    /// The player does not have enough EMC for a withdrawal.
    InsufficientEmc { required: i64, available: i64 },
}

impl From<AttachmentError> for DataError {
    fn from(error: AttachmentError) -> Self {
        Self::Attachment(error)
    }
}

/// Validate a fully qualified item id before it becomes knowledge.
///
/// Saved ids go through this too: accepting `""`, `" mineral"`, or an
/// unqualified `diamond` would make the learned set inconsistent with the ids
/// the EMC resolver and the registries use.
fn valid_item_id(id: &str) -> bool {
    let Some((namespace, path)) = id.split_once(':') else {
        return false;
    };
    if namespace.is_empty() || path.is_empty() || path.contains(':') {
        return false;
    }
    let namespace_ok = namespace
        .bytes()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'.' | b'-'));
    let path_ok = path.bytes().all(|b| {
        b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'.' | b'-' | b'/')
    });
    namespace_ok && path_ok
}

fn encode_player(data: &PlayerData) -> Result<Compound, AttachmentError> {
    if data.emc < 0 {
        return Err(AttachmentError::Encode);
    }
    let mut learned = Vec::with_capacity(data.learned.len());
    for id in &data.learned {
        if !valid_item_id(id) {
            return Err(AttachmentError::Encode);
        }
        learned.push(Tag::String(id.as_str().into()));
    }
    let mut out = Compound::new();
    out.insert("emc".into(), Tag::Long(data.emc));
    out.insert("learned".into(), Tag::List(learned));
    if let Some(lock) = &data.lock {
        if !valid_item_id(lock) {
            return Err(AttachmentError::Encode);
        }
        out.insert("lock".into(), Tag::String(lock.as_str().into()));
    }
    out.insert(
        "full_knowledge".into(),
        Tag::Byte(if data.full_knowledge { 1 } else { 0 }),
    );
    Ok(out)
}

fn decode_player(data: &Compound) -> Result<PlayerData, AttachmentError> {
    let emc = match data.get(&"emc".into()) {
        Some(Tag::Long(v)) => *v,
        Some(Tag::Int(v)) => i64::from(*v),
        // A saved compound without a balance is corrupt. Defaulting here would
        // silently erase a player's EMC on a codec regression.
        _ => return Err(AttachmentError::Decode),
    };
    if emc < 0 {
        return Err(AttachmentError::Decode);
    }
    let Some(Tag::List(list)) = data.get(&"learned".into()) else {
        return Err(AttachmentError::Decode);
    };
    let mut learned = BTreeSet::new();
    for tag in list {
        let Tag::String(s) = tag else {
            return Err(AttachmentError::Decode);
        };
        let id = s.to_string_lossy();
        if !valid_item_id(&id) || !learned.insert(id) {
            return Err(AttachmentError::Decode);
        }
    }
    // The lock was added after the first saved format. An absent key is a
    // valid legacy "no lock" state; a present key with a wrong shape or id is
    // corruption and must not be silently dropped.
    let lock = match data.get(&"lock".into()) {
        None => None,
        Some(Tag::String(s)) => {
            let id = s.to_string_lossy();
            if !valid_item_id(&id) {
                return Err(AttachmentError::Decode);
            }
            Some(id)
        }
        Some(_) => return Err(AttachmentError::Decode),
    };
    // Added with the Tome; an older save predates it, so absence means false.
    let full_knowledge = match data.get(&"full_knowledge".into()) {
        None => false,
        Some(Tag::Byte(value)) => *value == 1,
        Some(_) => return Err(AttachmentError::Decode),
    };
    Ok(PlayerData {
        emc,
        learned,
        lock,
        full_knowledge,
    })
}

pub static PLAYER: AttachmentType<PlayerData> = AttachmentType {
    namespace: "projecte",
    name: "player",
    default: PlayerData::default,
    codec: Some(AttachmentCodec {
        encode: encode_player,
        decode: decode_player,
    }),
    copy_on_death: true,
    sync: false,
};

fn encode_i64(value: &i64) -> Result<Compound, AttachmentError> {
    let mut out = Compound::new();
    out.insert("value".into(), Tag::Long(*value));
    Ok(out)
}

fn decode_i64(data: &Compound) -> Result<i64, AttachmentError> {
    match data.get(&"value".into()) {
        Some(Tag::Long(v)) => Ok(*v),
        Some(Tag::Int(v)) => Ok(i64::from(*v)),
        _ => Err(AttachmentError::Decode),
    }
}

/// Legacy storage from the old machine-buffer path. No production path writes
/// it any more; the codec stays strict so migration cannot invent a value.
pub static BUFFER: AttachmentType<i64> = AttachmentType {
    namespace: "projecte",
    name: "buffer",
    default: || 0,
    codec: Some(AttachmentCodec {
        encode: encode_i64,
        decode: decode_i64,
    }),
    copy_on_death: false,
    sync: false,
};

impl PlayerData {
    /// Whether this player can transmute `id`. Full knowledge covers every
    /// item with an EMC value; otherwise the exact id must have been learned.
    pub fn has_knowledge(&self, id: &str) -> bool {
        self.full_knowledge || self.learned.contains(id)
    }

    /// Learn one item. Returns true only when the learned set changed.
    ///
    /// The Tome is the one exception to the EMC requirement: using it sets
    /// `full_knowledge` and records the tome itself. A blacklisted or valueless
    /// item is ignored, and a full-knowledge player already knows everything.
    pub fn learn(
        &mut self,
        id: &str,
        blacklist: &BTreeSet<String>,
        has_value: impl Fn(&str) -> bool,
    ) -> bool {
        if id == TOME_ID {
            let inserted = self.learned.insert(id.to_owned());
            let flag_changed = !self.full_knowledge;
            self.full_knowledge = true;
            return inserted || flag_changed;
        }
        if !valid_item_id(id) || !has_value(id) || crate::emc::is_learn_blacklisted(id, blacklist) {
            return false;
        }
        if self.full_knowledge {
            return false;
        }
        self.learned.insert(id.to_owned())
    }

    /// Apply the Tome's effect: all valued items become known.
    pub fn grant_full_knowledge(&mut self) -> bool {
        let changed = self.learned.insert(TOME_ID.to_owned()) || !self.full_knowledge;
        self.full_knowledge = true;
        changed
    }

    /// Forget everything, including the Tome flag.
    pub fn clear_knowledge(&mut self) {
        self.learned.clear();
        self.full_knowledge = false;
    }
}

/// Drop learned ids that no longer carry an EMC value (e.g. after a data-map
/// reload), returning what was removed. Pure: the caller performs the write.
pub fn prune_known(
    known: &BTreeSet<String>,
    has_value: impl Fn(&str) -> bool,
) -> (BTreeSet<String>, Vec<String>) {
    let mut retained = BTreeSet::new();
    let mut removed = Vec::new();
    for id in known {
        if has_value(id) {
            retained.insert(id.clone());
        } else {
            removed.push(id.clone());
        }
    }
    (retained, removed)
}

/// Read the player's data. The attachment read may insert the default and may
/// fail; callers must not treat a failure as "empty data".
pub fn get(
    host: &mut dyn mod_api::Host,
    uuid: u128,
) -> Result<PlayerData, AttachmentError> {
    PLAYER.get(host, Holder::Player(uuid))
}

/// Write the player's data. `Ok` means the value really reached storage.
pub fn set(
    host: &mut dyn mod_api::Host,
    uuid: u128,
    data: PlayerData,
) -> Result<(), AttachmentError> {
    PLAYER.set(host, Holder::Player(uuid), data)
}

/// Change the balance by a signed amount, rejecting negative results and
/// overflow instead of clamping them. Returns the new balance.
pub fn add_emc(
    host: &mut dyn mod_api::Host,
    uuid: u128,
    amount: i64,
) -> Result<i64, DataError> {
    if amount == i64::MIN {
        return Err(DataError::BalanceOverflow);
    }
    let mut data = get(host, uuid)?;
    let next = if amount < 0 {
        let required = amount.checked_neg().ok_or(DataError::BalanceOverflow)?;
        data.emc.checked_sub(required).ok_or_else(|| {
            DataError::InsufficientEmc {
                required,
                available: data.emc,
            }
        })?
    } else {
        data.emc.checked_add(amount).ok_or(DataError::BalanceOverflow)?
    };
    data.emc = next;
    set(host, uuid, data)?;
    Ok(next)
}

/// Alias kept for the machine module's existing call site. It has exactly the
/// same fallible, checked semantics as [`add_emc`] — no error is swallowed.
pub fn try_add_emc(
    host: &mut dyn mod_api::Host,
    uuid: u128,
    amount: i64,
) -> Result<i64, DataError> {
    add_emc(host, uuid, amount)
}

/// Withdraw EMC only when the whole amount is available. The balance can never
/// go negative, and a failed write is reported to the caller.
pub fn take_emc(
    host: &mut dyn mod_api::Host,
    uuid: u128,
    amount: i64,
) -> Result<i64, DataError> {
    if amount < 0 {
        return Err(DataError::NegativeAmount);
    }
    let mut data = get(host, uuid)?;
    if data.emc < amount {
        return Err(DataError::InsufficientEmc {
            required: amount,
            available: data.emc,
        });
    }
    data.emc -= amount;
    set(host, uuid, data)?;
    Ok(amount)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> PlayerData {
        PlayerData {
            emc: 12345,
            learned: [
                "minecraft:diamond".to_string(),
                "projecte:dark_matter".to_string(),
            ]
            .into_iter()
            .collect(),
            lock: Some("minecraft:diamond".to_string()),
            full_knowledge: false,
        }
    }

    #[test]
    fn player_round_trip() {
        let data = sample();
        let encoded = encode_player(&data).unwrap();
        assert_eq!(decode_player(&encoded).unwrap(), data);
    }

    #[test]
    fn lock_round_trips_and_a_missing_lock_is_valid_legacy_data() {
        let mut data = sample();
        data.lock = None;
        let encoded = encode_player(&data).unwrap();
        assert_eq!(encoded.get(&"lock".into()), None);
        assert_eq!(decode_player(&encoded).unwrap(), data);

        // A save written before the lock existed has only emc and learned.
        let mut legacy = Compound::new();
        legacy.insert("emc".into(), Tag::Long(7));
        legacy.insert("learned".into(), Tag::List(Vec::new()));
        let decoded = decode_player(&legacy).unwrap();
        assert_eq!(decoded.lock, None);
        assert_eq!(decoded.emc, 7);
    }

    #[test]
    fn malformed_locks_are_rejected() {
        let mut compound = Compound::new();
        compound.insert("emc".into(), Tag::Long(0));
        compound.insert("learned".into(), Tag::List(Vec::new()));
        compound.insert("lock".into(), Tag::String("diamond".into()));
        assert_eq!(decode_player(&compound), Err(AttachmentError::Decode));

        let mut compound = Compound::new();
        compound.insert("emc".into(), Tag::Long(0));
        compound.insert("learned".into(), Tag::List(Vec::new()));
        compound.insert("lock".into(), Tag::Int(1));
        assert_eq!(decode_player(&compound), Err(AttachmentError::Decode));

        let mut data = sample();
        data.lock = Some("bad id".to_string());
        assert_eq!(encode_player(&data), Err(AttachmentError::Encode));
    }

    #[test]
    fn empty_saved_compound_is_rejected() {
        assert_eq!(decode_player(&Compound::new()), Err(AttachmentError::Decode));
    }

    #[test]
    fn missing_or_wrong_balance_is_rejected() {
        let mut compound = Compound::new();
        compound.insert("learned".into(), Tag::List(Vec::new()));
        assert_eq!(decode_player(&compound), Err(AttachmentError::Decode));

        let mut compound = Compound::new();
        compound.insert("emc".into(), Tag::String("12".into()));
        compound.insert("learned".into(), Tag::List(Vec::new()));
        assert_eq!(decode_player(&compound), Err(AttachmentError::Decode));
    }

    #[test]
    fn negative_saved_balance_is_rejected() {
        let mut compound = Compound::new();
        compound.insert("emc".into(), Tag::Long(-1));
        compound.insert("learned".into(), Tag::List(Vec::new()));
        assert_eq!(decode_player(&compound), Err(AttachmentError::Decode));
    }

    #[test]
    fn malformed_learned_entries_are_rejected() {
        for bad in ["diamond", "", "minecraft:", ":diamond", "minecraft:Bad Id"] {
            let mut compound = Compound::new();
            compound.insert("emc".into(), Tag::Long(0));
            compound.insert("learned".into(), Tag::List(vec![Tag::String(bad.into())]));
            assert_eq!(
                decode_player(&compound),
                Err(AttachmentError::Decode),
                "accepted {bad:?}"
            );
        }
    }

    #[test]
    fn non_string_and_duplicate_learned_entries_are_rejected() {
        let mut compound = Compound::new();
        compound.insert("emc".into(), Tag::Long(0));
        compound.insert("learned".into(), Tag::List(vec![Tag::Int(1)]));
        assert_eq!(decode_player(&compound), Err(AttachmentError::Decode));

        let mut compound = Compound::new();
        compound.insert("emc".into(), Tag::Long(0));
        compound.insert(
            "learned".into(),
            Tag::List(vec![
                Tag::String("minecraft:diamond".into()),
                Tag::String("minecraft:diamond".into()),
            ]),
        );
        assert_eq!(decode_player(&compound), Err(AttachmentError::Decode));
    }

    #[test]
    fn encoding_refuses_invalid_data() {
        let mut data = sample();
        data.emc = -1;
        assert_eq!(encode_player(&data), Err(AttachmentError::Encode));

        let mut data = sample();
        data.learned.insert("bad id".to_string());
        assert_eq!(encode_player(&data), Err(AttachmentError::Encode));
    }

    #[test]
    fn legacy_buffer_codec_is_strict() {
        assert_eq!(decode_i64(&Compound::new()), Err(AttachmentError::Decode));
        let mut compound = Compound::new();
        compound.insert("value".into(), Tag::Long(42));
        assert_eq!(decode_i64(&compound), Ok(42));
    }

    #[test]
    fn item_id_validation_covers_boundaries() {
        assert!(valid_item_id("minecraft:diamond"));
        assert!(valid_item_id("projecte:dark_matter"));
        assert!(valid_item_id("c:ingots/iron"));
        assert!(!valid_item_id("diamond"));
        assert!(!valid_item_id("minecraft:"));
        assert!(!valid_item_id(":diamond"));
        assert!(!valid_item_id("minecraft:a:b"));
        assert!(!valid_item_id("Minecraft:diamond"));
        assert!(!valid_item_id("minecraft:a b"));
    }

    #[test]
    fn full_knowledge_round_trips_and_legacy_saves_default_to_false() {
        let mut data = sample();
        data.full_knowledge = true;
        let encoded = encode_player(&data).unwrap();
        assert_eq!(decode_player(&encoded).unwrap().full_knowledge, true);

        // A save written before the Tome has no full_knowledge key.
        let mut legacy = Compound::new();
        legacy.insert("emc".into(), Tag::Long(0));
        legacy.insert("learned".into(), Tag::List(Vec::new()));
        assert_eq!(decode_player(&legacy).unwrap().full_knowledge, false);

        let mut wrong_type = legacy;
        wrong_type.insert("full_knowledge".into(), Tag::String("yes".into()));
        assert_eq!(decode_player(&wrong_type), Err(AttachmentError::Decode));
    }

    #[test]
    fn learning_respects_value_and_blacklist() {
        let mut data = PlayerData::default();
        let has_value = |id: &str| crate::emc::value(id) > 0;
        let blacklist: BTreeSet<String> = ["minecraft:diamond".to_string()].into_iter().collect();

        assert!(data.learn("minecraft:diamond", &blacklist, has_value) == false);
        assert!(data.learn("minecraft:not_an_item", &BTreeSet::new(), has_value) == false);
        assert!(data.learn("minecraft:iron_ingot", &BTreeSet::new(), has_value));
        assert!(data.learn("minecraft:iron_ingot", &BTreeSet::new(), has_value) == false);
        assert!(data.has_knowledge("minecraft:iron_ingot"));
        assert!(!data.has_knowledge("minecraft:diamond"));
    }

    #[test]
    fn tome_grants_full_knowledge_and_can_be_cleared() {
        let mut data = PlayerData::default();
        assert!(data.learn(TOME_ID, &BTreeSet::new(), |_| false));
        assert!(data.full_knowledge);
        assert!(data.has_knowledge("minecraft:diamond"));
        assert!(data.learn("minecraft:diamond", &BTreeSet::new(), |_| true) == false);
        assert!(data.grant_full_knowledge() == false, "already granted");

        data.clear_knowledge();
        assert!(!data.full_knowledge);
        assert!(data.learned.is_empty());
        assert!(!data.has_knowledge("minecraft:diamond"));
    }

    #[test]
    fn pruning_drops_only_ids_without_emc() {
        let known: BTreeSet<String> = [
            "minecraft:diamond".to_string(),
            "minecraft:stone".to_string(),
            "example:unknown".to_string(),
        ]
        .into_iter()
        .collect();
        let (retained, removed) = prune_known(&known, |id| crate::emc::value(id) > 0);
        assert!(retained.contains("minecraft:diamond"));
        assert!(retained.contains("minecraft:stone"));
        assert_eq!(removed, vec!["example:unknown".to_string()]);
        assert_eq!(retained.len() + removed.len(), known.len());

        let (same, none) = prune_known(&known, |_| true);
        assert_eq!(same, known);
        assert!(none.is_empty());
    }
}
