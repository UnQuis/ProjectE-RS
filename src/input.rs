//! ProjectE's CHARGE keybind, ported to SDK 0.2.
//!
//! The reference binds V to `PEKeybind.CHARGE`; sneak turns it into uncharge.
//! Horizon has no server-side key hook, so the client's registered mapping
//! sends a tiny payload to the server, which performs the authoritative write
//! to the held stack. No EMC is involved — charge is a free integer step.

use std::sync::Arc;

use mod_api::{
    ClientHost, ClientModifiers, IPayloadContext, IPayloadHandler, KeyConflictContext,
    KeyRegistration, PayloadBytes, PayloadDeclaration, PayloadDirection, PayloadHandleError,
    PayloadId, PayloadProtocol, HandlerThread, ServerHost,
};

use crate::charge;

/// The key mapping id within this mod, and the vanilla default (V).
pub(crate) const CHARGE_KEY_ID: &str = "charge";
pub(crate) const CHARGE_KEY_DEFAULT: &str = "key.keyboard.v";
pub(crate) const CHARGE_KEY_LABEL: &str = "key.projecte.charge";

/// The serverbound channel carrying one byte: bit 0 = charge press, bit 1 = sneak.
pub(crate) const KEY_PRESS_ID: &str = "projecte:key_press";
pub(crate) const KEY_PRESS_VERSION: &str = "1";

pub(crate) fn key_registration() -> KeyRegistration {
    KeyRegistration {
        id: CHARGE_KEY_ID.to_owned(),
        label: CHARGE_KEY_LABEL.to_owned(),
        key: CHARGE_KEY_DEFAULT.to_owned(),
        modifiers: ClientModifiers::default(),
        category: String::new(),
        context: KeyConflictContext::Universal,
    }
}

pub(crate) fn key_payload_id() -> PayloadId {
    PayloadId(KEY_PRESS_ID.to_owned())
}

pub(crate) fn key_payload_declaration() -> PayloadDeclaration {
    PayloadDeclaration::new(
        key_payload_id(),
        KEY_PRESS_VERSION.to_owned(),
        PayloadProtocol::Play,
        PayloadDirection::ToServer,
        false,
        HandlerThread::Main,
    )
}

/// Bit layout of the one-byte payload.
pub(crate) fn encode_key_press(sneak: bool) -> PayloadBytes {
    PayloadBytes(vec![if sneak { 0b11 } else { 0b01 }])
}

pub(crate) fn decode_key_press(bytes: &PayloadBytes) -> Result<bool, PayloadHandleError> {
    let flag = bytes
        .0
        .first()
        .copied()
        .ok_or_else(|| PayloadHandleError::Failed("empty projecte key press".to_owned()))?;
    if flag & 0b01 == 0 {
        return Err(PayloadHandleError::Failed(
            "projecte key press has no action bit".to_owned(),
        ));
    }
    Ok(flag & 0b10 != 0)
}

/// Client hook: send the charge press to the server once per key-down.
pub(crate) fn on_mapping_press(
    host: &mut dyn ClientHost,
    mapping: &str,
    pressed: bool,
) -> bool {
    if mapping != CHARGE_KEY_ID || !pressed {
        return false;
    }
    let sneak = host.key_down(ClientModifiers::SHIFT);
    let result = host.send_payload_to_server(&key_payload_id(), encode_key_press(sneak));
    result.is_ok()
}

/// Apply an authoritative charge-key press to the player's held stack.
pub(crate) fn apply_charge_press(
    host: &mut dyn ServerHost,
    player: mod_api::PlayerRef,
    sneak: bool,
) -> bool {
    let Some(slots) = player.inventory_slots(host) else {
        return false;
    };
    let Some(index) = player.selected_slot(host) else {
        return false;
    };
    let Some((slot, mut stack)) = slots.into_iter().find(|(slot, _)| *slot as usize == index) else {
        return false;
    };
    let id = host.item_key_id(stack.item());
    let Some(maximum) = charge::maximum_charge(&id) else {
        return false;
    };
    let current = stack.get(&charge::CHARGE).unwrap_or(0);
    let next = match charge::plan_change_charge(current, maximum, sneak) {
        Ok(next) => next,
        Err(_) => return false,
    };
    if stack.set(&charge::CHARGE, next).is_err() {
        return false;
    }
    player
        .set_inventory_slot(host, slot, Some(stack))
        .is_ok()
}

/// Server payload handler, registered by `lib.rs` in `Hooks::register`.
pub(crate) struct KeyPressHandler;

impl IPayloadHandler for KeyPressHandler {
    fn handle(
        &self,
        payload: PayloadBytes,
        context: &mut dyn IPayloadContext,
    ) -> Result<(), PayloadHandleError> {
        let sneak = decode_key_press(&payload)?;
        let player = context
            .player()
            .map_err(|error| PayloadHandleError::Failed(format!("{error:?}")))?;
        let host = context
            .server_host()
            .map_err(|error| PayloadHandleError::Failed(format!("{error:?}")))?;
        apply_charge_press(host, player, sneak);
        Ok(())
    }
}

pub(crate) fn key_press_handler() -> Arc<dyn IPayloadHandler> {
    Arc::new(KeyPressHandler)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_press_payload_carries_only_the_action_and_sneak_bits() {
        assert_eq!(encode_key_press(false).0, vec![0b01]);
        assert_eq!(encode_key_press(true).0, vec![0b11]);
        assert_eq!(decode_key_press(&encode_key_press(true)), Ok(true));
        assert_eq!(decode_key_press(&encode_key_press(false)), Ok(false));
        assert!(decode_key_press(&PayloadBytes(Vec::new())).is_err());
        assert!(decode_key_press(&PayloadBytes(vec![0b00])).is_err());
    }

    #[test]
    fn the_mapping_and_channel_are_stable() {
        assert_eq!(CHARGE_KEY_ID, "charge");
        assert_eq!(CHARGE_KEY_DEFAULT, "key.keyboard.v");
        assert_eq!(key_payload_id(), PayloadId("projecte:key_press".to_owned()));
        let declaration = key_payload_declaration();
        assert_eq!(declaration.id, key_payload_id());
        assert_eq!(declaration.direction, PayloadDirection::ToServer);
        assert_eq!(declaration.protocol, PayloadProtocol::Play);
    }
}
