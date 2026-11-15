//! Tooling crate (`sr_tools`) for importing Slime Rancher localization,
//! world/balance JSON dumps, local texture/audio/model manifests, and verifying `.sav` files.

pub mod assets_manifest;
pub mod localization;
pub mod world_dump;

pub use assets_manifest::AssetManifest;
pub use localization::{
    import_decompile_localization, parse_properties, LocalizationSet, ToolsError, ToolsResult,
    LOCALIZATION_BUNDLES,
};
pub use world_dump::{PlacedObjectDump, SpawnerDump, Vec3Dto, WorldDump};

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn step11_parse_properties_multiline_and_escapes() {
        let sample = r#"
# Comment line
! Another comment
t.pedia = Slimepedia
m.body.welcome = Hello:\n\
    Line 2\n\
    Key\=Value
l.hello = Welcome, {0}!
"#;
        let parsed = parse_properties(sample);
        assert_eq!(parsed.get("t.pedia").map(String::as_str), Some("Slimepedia"));
        assert_eq!(
            parsed.get("m.body.welcome").map(String::as_str),
            Some("Hello:\nLine 2\nKey=Value")
        );
        assert_eq!(
            parsed.get("l.hello").map(String::as_str),
            Some("Welcome, {0}!")
        );
    }

    #[test]
    fn step11_localization_set_lookup_and_format() {
        let dir = std::env::temp_dir().join(format!("sr_tools_loc_{}", std::process::id()));
        let text_dir = dir.join("decompile").join("en").join("TextAsset");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&text_dir).unwrap();
        std::fs::write(
            text_dir.join("ui"),
            "t.pedia = Slimepedia\nl.version = v {0}\n",
        )
        .unwrap();
        std::fs::write(text_dir.join("actor"), "l.pink_slime = Pink Slime\n").unwrap();

        let out_dir = dir.join("out");
        let logs = import_decompile_localization(&dir, &out_dir).unwrap();
        assert_eq!(logs.len(), 1);

        let loaded = LocalizationSet::load_json(&out_dir.join("localization_en.json")).unwrap();
        assert_eq!(loaded.get("ui.t.pedia"), Some("Slimepedia"));
        assert_eq!(loaded.get("l.pink_slime"), Some("Pink Slime"));
        assert_eq!(loaded.format("l.version", &["1.4.4"]), "v 1.4.4");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn step11_asset_manifest_scan_and_world_dump_roundtrip() {
        let dir = std::env::temp_dir().join(format!("sr_tools_assets_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("textures")).unwrap();
        std::fs::create_dir_all(dir.join("audio")).unwrap();
        std::fs::create_dir_all(dir.join("models")).unwrap();
        std::fs::create_dir_all(dir.join("localization")).unwrap();

        std::fs::write(dir.join("textures").join("SlimePink.png"), b"png").unwrap();
        std::fs::write(dir.join("audio").join("vac_shoot.ogg"), b"ogg").unwrap();
        std::fs::write(dir.join("models").join("slime_pink.glb"), b"glb").unwrap();
        std::fs::write(dir.join("localization").join("localization_ru.json"), b"{}").unwrap();

        let manifest = AssetManifest::scan(&dir).unwrap();
        assert_eq!(
            manifest.textures.get("slimepink").map(String::as_str),
            Some("textures/SlimePink.png")
        );
        assert_eq!(
            manifest.audio.get("vac_shoot").map(String::as_str),
            Some("audio/vac_shoot.ogg")
        );
        assert_eq!(
            manifest.models.get("slime_pink").map(String::as_str),
            Some("models/slime_pink.glb")
        );
        assert_eq!(
            manifest.localization.get("ru").map(String::as_str),
            Some("localization/localization_ru.json")
        );

        let mut dump = WorldDump::default();
        dump.land_plots.push(PlacedObjectDump {
            id: "plot001".to_string(),
            kind: "LandPlotLocation".to_string(),
            pos: Vec3Dto {
                x: 10.0,
                y: 2.0,
                z: -15.0,
            },
            rot_euler_deg: Vec3Dto::default(),
            zone: "RANCH".to_string(),
        });
        let dump_path: PathBuf = dir.join("world_dump.json");
        dump.save_json(&dump_path).unwrap();
        let reloaded = WorldDump::load_json(&dump_path).unwrap();
        assert_eq!(reloaded, dump);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn step11_embedded_localization_ru_and_en() {
        let ru = LocalizationSet::embedded("ru").unwrap();
        let en = LocalizationSet::embedded("en").unwrap();

        assert!(ru.total_entries() > 2500);
        assert!(en.total_entries() > 2500);
        assert_eq!(ru.get("ui.t.pedia"), Some("Слаймопедия"));
        assert_eq!(en.get("ui.t.pedia"), Some("Slimepedia"));
        assert_eq!(ru.get("actor.l.beet_veggie"), Some("Сердцевёкла"));
        assert_eq!(en.get("actor.l.beet_veggie"), Some("Heart Beet"));
    }
}
