import json
import tempfile
import unittest
from pathlib import Path

from tools.check_gameplay import parse_emc, parse_recipes
from tools.check_resources import unique_object, validate_emc_map, validate_recipe
from tools.check_manifest import validate as validate_manifest


class ResourceChecks(unittest.TestCase):
    def test_duplicate_json_keys_are_rejected(self):
        with self.assertRaisesRegex(ValueError, "duplicate JSON key"):
            unique_object([("id", "minecraft:stone"), ("id", "minecraft:dirt")])

    def test_emc_map_rejects_negative_and_malformed_values(self):
        errors = validate_emc_map({
            "values": {"minecraft:stone": -1, "bad id": 2}, "remove": [],
        }, Path("emc.json"))
        self.assertEqual(len(errors), 2)

    def test_shaped_recipe_requires_matching_key(self):
        errors = validate_recipe({
            "type": "minecraft:crafting_shaped", "pattern": ["AA", " B"],
            "key": {"A": {"item": "minecraft:stone"}},
            "result": {"id": "projecte:dark_matter", "count": 1},
        }, Path("recipe.json"))
        self.assertTrue(any("key mismatch" in error for error in errors))

    def test_shapeless_recipe_accepts_item_and_tag_ingredients(self):
        errors = validate_recipe({
            "type": "minecraft:crafting_shapeless",
            "ingredients": [{"item": "minecraft:stone"}, {"tag": "minecraft:logs"}],
            "result": {"id": "projecte:alchemical_coal"},
        }, Path("recipe.json"))
        self.assertEqual(errors, [])


class GameplayChecks(unittest.TestCase):
    def test_emc_parser_reads_datamap_values(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            path = root / "engine/data/projecte/data_maps/item/emc.json"
            path.parent.mkdir(parents=True)
            path.write_text(json.dumps({
                "values": {
                    "minecraft:stone": 1,
                    "minecraft:dirt": 1,
                    "projecte:dark_matter": 16384,
                    "#minecraft:logs": 32,
                }
            }))
            values, errors = parse_emc(root)
        self.assertEqual(errors, [])
        self.assertEqual(values, {
            "minecraft:stone": 1, "minecraft:dirt": 1,
            "projecte:dark_matter": 16384,
        })

    def test_emc_parser_rejects_duplicate_ids(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            path = root / "engine/data/projecte/data_maps/item/emc.json"
            path.parent.mkdir(parents=True)
            path.write_text(
                '{"values":{"minecraft:stone":1,"minecraft:stone":2}}'
            )
            _, errors = parse_emc(root)
        self.assertTrue(any("duplicate JSON key" in error for error in errors))

    def test_recipe_parser_checks_pattern_key_and_catalogue(self):
        source = """
            const STONE: &str = "minecraft:stone";
            const DIRT: &str = "minecraft:dirt";
            const TABLE: &str = "projecte:table";
            const CATALYST_RECIPES: &[&str] = &["table"];
            const SHAPED: &[ShapedRecipe] = &[
                ShapedRecipe {
                    name: "table", pattern: ["SSS", "SDS", "SSS"],
                    key: &[('S', STONE), ('D', DIRT)], result: TABLE, count: 1,
                },
            ];
        """
        recipes, errors, catalysts = parse_recipes(source)
        self.assertEqual(errors, [])
        self.assertEqual(catalysts, ["table"])
        self.assertEqual(recipes[0]["result"], "projecte:table")


class ManifestChecks(unittest.TestCase):
    def test_native_manifest_and_cargo_pin_must_match(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "engine").mkdir()
            (root / ".github/workflows").mkdir(parents=True)
            revision = "a" * 40
            mixins = "b" * 40
            (root / "engine/horizon-mod.toml").write_text(
                'id="projecte"\nversion="0.1.0"\napi="0.2"\nside="both"\n'
                'capabilities=["registries"]\n[native]\ngame="26.3"\n'
                'horizon="' + revision + '"\nmixins="' + mixins + '"\n'
            )
            (root / "Cargo.toml").write_text('[workspace]\nmembers=["engine"]\n')
            (root / "engine/Cargo.toml").write_text(
                '[package]\nname="mod-projecte"\nversion="0.1.0"\n'
                '[dependencies]\nhorizon-mod-api={git="https://github.com/arismoko/horizon",rev="'
                + revision + '"}\n'
            )
            workflow = root / ".github/workflows/rust.yml"
            workflow.write_text(
                'cargo generate-lockfile --manifest-path engine/Cargo.toml\n'
                'cargo check --manifest-path engine/Cargo.toml --all-targets\n'
            )
            self.assertEqual(validate_manifest(root), [])
            workflow.write_text(workflow.read_text().replace(
                "cargo check --manifest-path engine/Cargo.toml --all-targets",
                "cargo check --workspace"
            ))
            self.assertTrue(any(
                "compile the native mod crate directly" in error
                for error in validate_manifest(root)
            ))




if __name__ == "__main__":
    unittest.main()
