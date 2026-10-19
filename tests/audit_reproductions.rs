use mapscraper::instance::InstanceResolver;
use mapscraper::mounts::SearchPathManager;
use mapscraper::paths::VirtualAssetPath;
use mapscraper::report::{AssetKind, AssetStatus, Manifest};
use mapscraper::utils::{normalize_source_path, safe_copy_file};
use mapscraper::vmf::parse_vmf;
use mapscraper::vmt::parse_vmt;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

/// Verify that bspzip packing list includes ONLY auxiliary companion files
/// (.nav, .ain, thumbnail, particles, soundscapes) and strictly excludes
/// models, textures, materials, and sounds so custom content remains loose in the addon folder.
#[test]
fn test_bsp_pack_only_includes_auxiliary_companion_files_not_models_or_materials() {
    let mut manifest = Manifest::new("test_map");

    let mat_virt = VirtualAssetPath::parse("materials/models/props/crate.vmt").unwrap();
    let tex_virt = VirtualAssetPath::parse("materials/models/props/crate.vtf").unwrap();
    let mdl_virt = VirtualAssetPath::parse("models/props/crate.mdl").unwrap();
    let snd_virt = VirtualAssetPath::parse("sound/weapons/explode3.wav").unwrap();
    let nav_virt = VirtualAssetPath::parse("maps/test_map.nav").unwrap();

    let dummy_file = PathBuf::from("Cargo.toml"); // existing file on disk

    manifest.record(
        mat_virt,
        AssetKind::Material,
        None,
        Some(dummy_file.clone()),
        AssetStatus::Extracted,
        100,
        "VMT",
    );
    manifest.record(
        tex_virt,
        AssetKind::Texture,
        None,
        Some(dummy_file.clone()),
        AssetStatus::Extracted,
        1000,
        "VTF",
    );
    manifest.record(
        mdl_virt,
        AssetKind::Model,
        None,
        Some(dummy_file.clone()),
        AssetStatus::Extracted,
        500,
        "MDL",
    );
    manifest.record(
        snd_virt,
        AssetKind::Sound,
        None,
        Some(dummy_file.clone()),
        AssetStatus::Extracted,
        200,
        "WAV",
    );
    manifest.record(
        nav_virt,
        AssetKind::Auxiliary,
        None,
        Some(dummy_file.clone()),
        AssetStatus::Extracted,
        50,
        "NAV",
    );

    let zip_entries = manifest.generate_bspzip_list();

    // Verify nav companion is present in bspzip entries
    assert!(
        zip_entries.iter().any(|e| e == "maps/test_map.nav"),
        "Missing nav in BSP zip list"
    );

    // Verify custom content (materials, textures, models, sounds) is NOT in BSP zip list
    assert!(
        !zip_entries
            .iter()
            .any(|e| e == "materials/models/props/crate.vmt"),
        "Materials must NOT be packed into the BSP pakfile"
    );
    assert!(
        !zip_entries
            .iter()
            .any(|e| e == "materials/models/props/crate.vtf"),
        "Textures must NOT be packed into the BSP pakfile"
    );
    assert!(
        !zip_entries.iter().any(|e| e == "models/props/crate.mdl"),
        "Models must NOT be packed into the BSP pakfile"
    );
    assert!(
        !zip_entries
            .iter()
            .any(|e| e == "sound/weapons/explode3.wav"),
        "Sounds must NOT be packed into the BSP pakfile"
    );

    let summary = manifest.summary();
    assert_eq!(summary.materials_extracted, 1);
    assert_eq!(summary.textures_extracted, 1);
    assert_eq!(summary.models_extracted, 1);
    assert_eq!(summary.sounds_extracted, 1);
    assert_eq!(summary.auxiliary_extracted, 1);
}

/// P1-11 Reproduction: strip_vmt_comments strips '//' even when inside double quotes!
#[test]
fn test_reproduce_p1_11_vmt_comment_in_quoted_string() {
    let content = fs::read_to_string("tests/fixtures/vmt/quote_comment.vmt")
        .expect("Failed to read quote_comment.vmt fixture");

    let map = parse_vmt(&content);
    let desc = map
        .get("description")
        .and_then(|v| v.first())
        .cloned()
        .unwrap_or_default();

    // In 1.8.1: strip_vmt_comments strips everything after "//" without checking if it's inside quotes!
    // So the description becomes truncated: "Texture with "
    assert_eq!(
        desc, "Texture with // not a comment inside string",
        "P1-11 FAILURE REPRODUCED: strip_vmt_comments erroneously stripped '//' inside double quotes!"
    );
}

/// P0-6 Reproduction: VMF regex stops at first closing brace '}'
/// When an entity contains an inner block (such as editor { ... }),
/// fields placed after that inner block (e.g. "file" in func_instance)
/// are truncated and ignored!
#[test]
fn test_reproduce_p0_6_vmf_nested_block_truncation() {
    let content = fs::read_to_string("tests/fixtures/vmf/nested_blocks.vmf")
        .expect("Failed to read nested_blocks.vmf fixture");

    let sound_getters = HashSet::new();
    let parsed = parse_vmf(&content, &sound_getters);

    // Because of the regex entity\s*\{([^}]+)\}, it stopped at the end of editor { ... }
    // Thus instances was NOT found in 1.8.1!
    // We assert that the instance SHOULD be discovered.
    assert!(
        parsed.instances.iter().any(|i| i.contains("instances/test_sub.vmf")),
        "P0-6 FAILURE REPRODUCED: parse_vmf failed to find func_instance due to nested editor block truncation!"
    );
}

/// P0-7 Reproduction: Soundscript event names without file extensions
/// ambient_generic with "message" "Weapon_AR2.Single" is ignored
/// because 1.8.1 only keeps .wav, .mp3, .ogg!
#[test]
fn test_reproduce_p0_7_soundscript_names_ignored() {
    let content = fs::read_to_string("tests/fixtures/vmf/soundscripts.vmf")
        .expect("Failed to read soundscripts.vmf fixture");

    let mut sound_getters = HashSet::new();
    sound_getters.insert("message".to_string());

    let parsed = parse_vmf(&content, &sound_getters);

    // In 1.8.1: "Weapon_AR2.Single" is dropped, only "doors/door_metal_large_open1.wav" is kept.
    assert!(
        parsed.sounds.contains("Weapon_AR2.Single") || parsed.sounds.contains("weapon_ar2.single"),
        "P0-7 FAILURE REPRODUCED: parse_vmf ignored soundscript event name 'Weapon_AR2.Single' because it lacks a file extension!"
    );
}

/// P0-10 Reproduction: normalize_source_path does not sanitize ../ directory traversal
#[test]
fn test_reproduce_p0_10_path_traversal() {
    let malicious_path = "../../Windows/System32/drivers/etc/hosts";
    let normalized = normalize_source_path(malicious_path);

    assert!(
        !normalized.contains(".."),
        "P0-10 FAILURE REPRODUCED: normalize_source_path allowed '..' path traversal: {}",
        normalized
    );
}

/// P0-5 Reproduction: safe_copy_file returns Ok(()) when source does not exist
#[test]
fn test_reproduce_p0_5_safe_copy_file_masks_missing_source() {
    let non_existent_src = PathBuf::from("tests/fixtures/non_existent_file_12345.vtf");
    let dest = PathBuf::from("tests/fixtures/dest_dummy.vtf");

    let result = safe_copy_file(&non_existent_src, &dest);
    assert!(
        result.is_err(),
        "P0-5 FAILURE REPRODUCED: safe_copy_file returned Ok(()) for non-existent source file!"
    );
}

/// P1-14: InstanceResolver propagates has_soundscapes from child instances
#[test]
fn test_reproduce_p1_14_instance_soundscape_propagation() {
    let parent_path = PathBuf::from("tests/fixtures/vmf/instance_parent.vmf");
    let content = fs::read_to_string(&parent_path).unwrap();
    let sound_getters = HashSet::new();
    let parsed = parse_vmf(&content, &sound_getters);

    let search_paths = SearchPathManager::new(&PathBuf::from("tests/fixtures/vmf"), &[]);
    let mut resolver = InstanceResolver::new(&search_paths, &sound_getters);
    let (merged, _inst_files) = resolver.process_instances(&parsed.instances, &parent_path);

    assert!(
        merged.has_soundscapes,
        "P1-14 FAILURE REPRODUCED: InstanceResolver did not detect or propagate soundscapes in child instances!"
    );
}

/// P0-3 & P0-4: Test GMA archive resolution and Source engine SearchPaths macro evaluation
#[test]
fn test_vfs_gma_and_searchpath_macros() {
    use std::io::Write;

    let temp_dir = std::env::temp_dir().join("mapscraper_vfs_test");
    let _ = fs::remove_dir_all(&temp_dir);
    let game_dir = temp_dir.join("garrysmod");
    let addons_dir = game_dir.join("addons");
    fs::create_dir_all(&addons_dir).unwrap();

    // Create a mock GMA file in garrysmod/addons/test_pack.gma
    let gma_path = addons_dir.join("test_pack.gma");
    let mut f = fs::File::create(&gma_path).unwrap();
    f.write_all(b"GMAD").unwrap();
    f.write_all(&[3u8]).unwrap();
    f.write_all(&[0u8; 16]).unwrap();
    f.write_all(b"\0").unwrap();
    f.write_all(b"Pack\0").unwrap();
    f.write_all(b"Desc\0").unwrap();
    f.write_all(b"Author\0").unwrap();
    f.write_all(&1i32.to_le_bytes()).unwrap();

    // Entry: materials/gma_pack/wood.vmt
    f.write_all(&1u32.to_le_bytes()).unwrap();
    f.write_all(b"materials/gma_pack/wood.vmt\0").unwrap();
    let data = b"\"VertexLitGeneric\"\n{\n\t\"$basetexture\" \"gma_pack/wood\"\n}";
    f.write_all(&(data.len() as u64).to_le_bytes()).unwrap();
    f.write_all(&0u32.to_le_bytes()).unwrap();
    f.write_all(&0u32.to_le_bytes()).unwrap(); // End of entries
    f.write_all(data).unwrap();
    drop(f);

    // Create gameinfo.txt with |gameinfo_path| macro
    let gameinfo_content = r#"
        "GameInfo"
        {
            "game" "Garry's Mod"
            "FileSystem"
            {
                "SearchPaths"
                {
                    "game" "|gameinfo_path|addons/*"
                }
            }
        }
    "#;
    fs::write(game_dir.join("gameinfo.txt"), gameinfo_content).unwrap();

    let spm = SearchPathManager::new(&game_dir, &[]);
    assert!(
        !spm.gmas.is_empty(),
        "GMA archive should be discovered in addons directory"
    );

    let resolved = spm.resolve_asset("materials/gma_pack/wood.vmt");
    assert!(resolved.is_some(), "Asset inside GMA must be resolved");

    let read_bytes = spm.read_asset(&resolved.unwrap()).unwrap();
    assert_eq!(read_bytes, data);

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_non_utf8_instance_parsing() {
    let temp_dir = std::env::temp_dir().join(format!("mapscraper_ansi_test_{}", std::process::id()));
    let _ = fs::create_dir_all(&temp_dir);

    // Root VMF references an instance
    let root_vmf = temp_dir.join("root.vmf");
    let root_content = r#"
        entity
        {
            "id" "1"
            "classname" "func_instance"
            "file" "instance_ansi.vmf"
        }
    "#;
    fs::write(&root_vmf, root_content).unwrap();

    // Instance VMF containing Windows-1252 / ANSI bytes (e.g. 0xE9 for 'é' which is invalid UTF-8)
    let instance_vmf = temp_dir.join("instance_ansi.vmf");
    let mut instance_bytes = Vec::new();
    instance_bytes.extend_from_slice(b"// Auteur: R\xe9mi (Windows-1252 ANSI non-UTF8 comment)\n");
    instance_bytes.extend_from_slice(br#"
        solid
        {
            side
            {
                "material" "NATURE/BLEND_GROUND01_TO_GRAVEL"
            }
        }
    "#);
    fs::write(&instance_vmf, instance_bytes).unwrap();

    let spm = SearchPathManager::new(&temp_dir, &[]);
    let sound_getters = HashSet::new();
    let mut resolver = InstanceResolver::new(&spm, &sound_getters);
    let mut initial_instances = HashSet::new();
    initial_instances.insert("instance_ansi.vmf".to_string());

    let (merged_assets, instance_files) = resolver.process_instances(&initial_instances, &root_vmf);

    assert_eq!(instance_files.len(), 1, "Non-UTF8 instance VMF must NOT be skipped");
    assert!(
        merged_assets
            .materials
            .iter()
            .any(|m| m.to_lowercase() == "nature/blend_ground01_to_gravel"),
        "Material from non-UTF8 instance must be successfully extracted"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_bsp_pakfile_and_wvt_patch_resolution() {
    // 1. Verify WVT patch base resolution
    let patch_path = "maps/custom_map/terrain/blend_ground01_to_gravel_wvt_patch";
    let base = mapscraper::vmt::resolve_base_material(patch_path);
    assert_eq!(
        base.as_deref(),
        Some("terrain/blend_ground01_to_gravel"),
        "WVT patch must resolve to its master base blend material"
    );
}

#[test]
fn test_detail_vbsp_parsing_and_resolution() {
    let vmf = r#"
        world {
            "classname" "worldspawn"
            "detailmaterial" "detail/detailsprites"
            "detailvbsp" "detail_custom_foliage.vbsp"
            "skyname" "sky_dust"
        }
    "#;
    let sound_getters = HashSet::new();
    let assets = parse_vmf(vmf, &sound_getters);

    assert!(
        assets.detail_vbsps.contains("detail_custom_foliage.vbsp"),
        "VMF parser must extract detailvbsp key from worldspawn"
    );
}


