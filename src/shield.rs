//! Client-side special item models for ProjectE's shields.
//!
//! Upstream `PEShield` extends `ShieldItem` and `ShieldISTER` draws the vanilla
//! shield geometry with ProjectE's own texture. Horizon keeps geometry in the
//! packs: a `minecraft:special` item model names one of the renderers below,
//! which returns the shield part model. The `base` model supplies vanilla's
//! shield display transforms for first person, third person, GUI and blocking.

use mod_api::{ClientHost, ItemStack, SpecialModelPart, SpecialModelRenderer, Tag};

/// Draws one fixed shield geometry model for every stack of this item model.
pub struct ShieldModel {
    geometry: &'static str,
}

impl ShieldModel {
    pub const fn new(geometry: &'static str) -> Self {
        Self { geometry }
    }
}

impl SpecialModelRenderer for ShieldModel {
    fn extract(
        &self,
        _host: &dyn ClientHost,
        _stack: &ItemStack,
        _config: &Tag,
    ) -> Result<Vec<SpecialModelPart>, String> {
        // The resource model is authored in vanilla's shield entity-model
        // space (plate x=-6..6, y=-11..11, z=-2..-1; handle x=-1..1,
        // y=-3..3, z=-1..5). An identity part pose keeps its face winding and
        // textures outward while the special model's base supply vanilla's
        // first-person, third-person, GUI and blocking display transforms.
        Ok(vec![SpecialModelPart::new(self.geometry)])
    }
}
