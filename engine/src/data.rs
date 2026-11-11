//! Per-player ProjectE data.

use std::collections::BTreeSet;

use mod_api::mod_api::{
    AttachmentCodec, AttachmentError, AttachmentType, Compound, Holder, Tag,
};

/// A player's transmutation knowledge and stored EMC.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlayerData {
    pub emc: i64,
    pub learned: BTreeSet<String>,
}

fn encode_player(data: &PlayerData) -> Result<Compound, AttachmentError> {
    let mut out = Compound::new();
    out.insert("emc".into(), Tag::Long(data.emc));
    out.insert(
        "learned".into(),
        Tag::List(
            data.learned
                .iter()
                .map(|id| Tag::String(id.as_str().into()))
                .collect(),
        ),
    );
    Ok(out)
}

fn decode_player(data: &Compound) -> Result<PlayerData, AttachmentError> {
    let emc = match data.get(&"emc".into()) {
        Some(Tag::Long(v)) => *v,
        Some(Tag::Int(v)) => i64::from(*v),
        _ => 0,
    };
    let mut learned = BTreeSet::new();
    if let Some(Tag::List(list)) = data.get(&"learned".into()) {
        for tag in list {
            if let Tag::String(s) = tag {
                learned.insert(s.to_string_lossy());
            }
        }
    }
    Ok(PlayerData { emc, learned })
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
        _ => Ok(0),
    }
}

/// Legacy storage used by PR #3. Kept only to migrate existing machine buffers
/// into the block entity's NBT field when the machine first ticks after upgrade.
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

pub fn get(host: &mut dyn mod_api::mod_api::Host, uuid: u128) -> PlayerData {
    PLAYER.get(host, Holder::Player(uuid)).unwrap_or_default()
}

pub fn set(host: &mut dyn mod_api::mod_api::Host, uuid: u128, data: PlayerData) {
    let _ = PLAYER.set(host, Holder::Player(uuid), data);
}

pub fn try_add_emc(
    host: &mut dyn mod_api::mod_api::Host,
    uuid: u128,
    amount: i64,
) -> Result<i64, AttachmentError> {
    let mut data = PLAYER.get(host, Holder::Player(uuid))?;
    data.emc = data.emc.saturating_add(amount).max(0);
    let emc = data.emc;
    PLAYER.set(host, Holder::Player(uuid), data)?;
    Ok(emc)
}

pub fn add_emc(host: &mut dyn mod_api::mod_api::Host, uuid: u128, amount: i64) -> i64 {
    try_add_emc(host, uuid, amount).unwrap_or_else(|_| get(host, uuid).emc)
}

pub fn take_emc(host: &mut dyn mod_api::mod_api::Host, uuid: u128, amount: i64) -> bool {
    if amount < 0 {
        return false;
    }
    let mut data = get(host, uuid);
    if data.emc < amount {
        return false;
    }
    data.emc -= amount;
    set(host, uuid, data);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_round_trip() {
        let data = PlayerData {
            emc: 12345,
            learned: [
                "minecraft:diamond".to_string(),
                "projecte:dark_matter".to_string(),
            ]
            .into_iter()
            .collect(),
        };
        let encoded = encode_player(&data).unwrap();
        assert_eq!(decode_player(&encoded).unwrap(), data);
    }

    #[test]
    fn defaults_are_empty() {
        let data = decode_player(&Compound::new()).unwrap();
        assert_eq!(data, PlayerData::default());
    }
}
