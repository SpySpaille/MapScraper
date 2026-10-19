use colored::Colorize;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::getters::Getters;
use crate::instance::{InstanceFile, InstanceResolver};
use crate::mdl::{parse_mdl_textures, MODEL_VARIANTS};
use crate::mounts::SearchPathManager;
use crate::paths::VirtualAssetPath;
use crate::report::{AssetKind, AssetStatus, Manifest};
use crate::utils::{clean_sound_path, normalize_source_path, safe_copy_file};
use crate::vmf::parse_vmf;
use crate::vmt::{normalize_texture_path, parse_vmt, resolve_base_material};

pub struct ScraperOptions {
    pub game_path: PathBuf,
    pub vmf_path: PathBuf,
    pub extract_materials: bool,
    pub extract_models: bool,
    pub extract_sounds: bool,
    pub ready_to_publish: bool,
    pub custom_mounts: Vec<String>,
    pub output_dir: Option<PathBuf>,
}

#[derive(Debug, Default)]
pub struct ScraperStats {
    pub instances_count: usize,
    pub materials_count: usize,
    pub textures_count: usize,
    pub models_count: usize,
    pub sounds_count: usize,
    pub publish_files_count: usize,
    pub bsp_packed: bool,
}

struct Spinner {
    stop_signal: std::sync::Arc<std::sync::atomic::AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl Spinner {
    pub fn start(message: &'static str) -> Self {
        let stop_signal = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stop_clone = stop_signal.clone();

        let handle = std::thread::spawn(move || {
            let frames = ["|", "/", "-", "\\"];
            let mut idx = 0;
            while !stop_clone.load(std::sync::atomic::Ordering::Relaxed) {
                print!(
                    "\r\x1b[90m- \x1b[37m{} \x1b[32m{}\x1b[0m   ",
                    message, frames[idx]
                );
                let _ = std::io::Write::flush(&mut std::io::stdout());
                idx = (idx + 1) % frames.len();
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            print!("\r\x1b[2K");
            let _ = std::io::Write::flush(&mut std::io::stdout());
        });

        Self {
            stop_signal,
            handle: Some(handle),
        }
    }

    pub fn stop(&mut self) {
        if let Some(handle) = self.handle.take() {
            self.stop_signal
                .store(true, std::sync::atomic::Ordering::Relaxed);
            let _ = handle.join();
        }
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        self.stop();
    }
}

pub fn run_scraper(options: ScraperOptions) -> std::io::Result<ScraperStats> {
    let map_name = options
        .vmf_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("map")
        .to_string();

    let output_dir = options
        .output_dir
        .unwrap_or_else(|| PathBuf::from(format!("./{}", map_name)));

    if output_dir.exists() {
        let _ = fs::remove_dir_all(&output_dir);
    }
    fs::create_dir_all(&output_dir)?;

    let mut manifest = Manifest::new(&map_name);

    // 1. Initialize Mounts (Loose dirs, VPKs, and GMA addons)
    let mut search_paths = SearchPathManager::new(&options.game_path, &options.custom_mounts);
    println!(
        "\n📂 Game Path: {}",
        search_paths.primary_path.display().to_string().cyan()
    );

    let mounted: Vec<_> = search_paths
        .all_paths
        .iter()
        .filter(|p| p.label != "Primary Game")
        .collect();
    if !mounted.is_empty() || !search_paths.vpks.is_empty() || !search_paths.gmas.is_empty() {
        println!("🔗 Mounted search providers:");
        for m in &mounted {
            println!(
                "   \x1b[90m├─\x1b[0m {}: {}",
                m.label.green(),
                m.path.display().to_string().white()
            );
        }
        for v in &search_paths.vpks {
            println!(
                "   \x1b[90m├─\x1b[0m {}: {}",
                v.label.green(),
                v.path.display().to_string().white()
            );
        }
        for g in &search_paths.gmas {
            println!(
                "   \x1b[90m├─\x1b[0m {}: {}",
                g.label.green(),
                g.path.display().to_string().white()
            );
        }
    }

    // Mount companion BSP pakfile if available
    let bsp_candidates = [
        options.vmf_path.with_extension("bsp"),
        search_paths
            .primary_path
            .join("maps")
            .join(format!("{}.bsp", map_name)),
    ];
    for bsp_cand in &bsp_candidates {
        if bsp_cand.exists() && bsp_cand.is_file() && search_paths.mount_bsp(bsp_cand) {
            println!(
                "   \x1b[90m├─\x1b[0m {}: {}",
                "BSP Pakfile".green(),
                bsp_cand.display().to_string().white()
            );
            break;
        }
    }

    // 2. Load Getters
    let getters = Getters::load();

    // 3. Read and parse root VMF (handles UTF-8, ANSI, Windows-1252)
    let mut spin = Spinner::start("Parsing VMF map file...");
    let vmf_bytes = fs::read(&options.vmf_path)?;
    let vmf_content = String::from_utf8_lossy(&vmf_bytes);
    let mut root_assets = parse_vmf(&vmf_content, &getters.sound_getters);
    spin.stop();
    println!("📄 • ✅ VMF map parsed");

    // 4. Resolve func_instances
    let mut instance_files: Vec<InstanceFile> = Vec::new();
    if !root_assets.instances.is_empty() {
        let mut spin = Spinner::start("Resolving func_instance(s)...");
        let mut resolver = InstanceResolver::new(&search_paths, &getters.sound_getters);
        let (inst_assets, files) =
            resolver.process_instances(&root_assets.instances, &options.vmf_path);
        instance_files = files;

        root_assets.materials.extend(inst_assets.materials);
        root_assets.models.extend(inst_assets.models);
        root_assets.sounds.extend(inst_assets.sounds);
        root_assets.direct_vtfs.extend(inst_assets.direct_vtfs);
        root_assets.detail_vbsps.extend(inst_assets.detail_vbsps);
        if inst_assets.has_soundscapes {
            root_assets.has_soundscapes = true;
        }
        spin.stop();
        println!(
            "🔄 • ✅ {} func_instance VMFs processed",
            instance_files.len().to_string().green()
        );
    }

    // Move extract_model below extract_material so it can call extract_material
    let mut processed_materials: HashSet<String> = HashSet::new();
    let mut processed_textures: HashSet<String> = HashSet::new();
    let mut processed_models: HashSet<String> = HashSet::new();

    let extract_material = |mat_name: &str,
                            pm: &mut HashSet<String>,
                            pt: &mut HashSet<String>,
                            manifest: &mut Manifest| {
        let mut queue = vec![normalize_source_path(mat_name)];

        while let Some(current_mat) = queue.pop() {
            let clean = if current_mat.to_lowercase().ends_with(".vmt") {
                current_mat[..current_mat.len() - 4].to_string()
            } else {
                current_mat.clone()
            };

            let mat_key = clean.to_lowercase();
            if pm.contains(&mat_key) || mat_key.is_empty() {
                continue;
            }
            pm.insert(mat_key.clone());

            let rel_vmt = format!("materials/{}.vmt", mat_key);
            let Ok(virt_vmt) = VirtualAssetPath::parse(&rel_vmt) else {
                continue;
            };

            let search_vmt = format!("materials/{}.vmt", clean);
            if let Some(resolved_vmt) = search_paths.resolve_asset(&search_vmt) {
                let dest_vmt = output_dir.join(&rel_vmt);
                match search_paths.extract_asset(&resolved_vmt, &dest_vmt) {
                    Ok(bytes) => {
                        manifest.record(
                            virt_vmt,
                            AssetKind::Material,
                            resolved_vmt.loose_path().map(|p| p.to_path_buf()),
                            Some(dest_vmt.clone()),
                            AssetStatus::Extracted,
                            bytes,
                            resolved_vmt.label(),
                        );
                    }
                    Err(e) => {
                        manifest.record(
                            virt_vmt,
                            AssetKind::Material,
                            resolved_vmt.loose_path().map(|p| p.to_path_buf()),
                            Some(dest_vmt),
                            AssetStatus::Failed(e.to_string()),
                            0,
                            "Material VMT copy failed",
                        );
                        continue;
                    }
                }

                if let Ok(vmt_bytes) = search_paths.read_asset(&resolved_vmt) {
                    let vmt_data = String::from_utf8_lossy(&vmt_bytes);
                    let vmt_map = parse_vmt(&vmt_data);

                    // Extract textures
                    for getter in &getters.texture_getters {
                        if let Some(values) = vmt_map.get(getter) {
                            for raw_val in values {
                                let norm_tex = normalize_texture_path(raw_val);
                                let tex_lower = norm_tex.to_lowercase();
                                if !pt.contains(&tex_lower) && !tex_lower.is_empty() {
                                    pt.insert(tex_lower.clone());

                                    // Copy .vtf
                                    let rel_vtf_search = format!("materials/{}.vtf", norm_tex);
                                    let rel_vtf_dest = format!("materials/{}.vtf", tex_lower);
                                    if let Ok(virt_vtf) = VirtualAssetPath::parse(&rel_vtf_dest) {
                                        if let Some(res_vtf) =
                                            search_paths.resolve_asset(&rel_vtf_search)
                                        {
                                            let dest_vtf = output_dir.join(&rel_vtf_dest);
                                            match search_paths.extract_asset(&res_vtf, &dest_vtf) {
                                                Ok(bytes) => {
                                                    manifest.record(
                                                        virt_vtf,
                                                        AssetKind::Texture,
                                                        res_vtf
                                                            .loose_path()
                                                            .map(|p| p.to_path_buf()),
                                                        Some(dest_vtf),
                                                        AssetStatus::Extracted,
                                                        bytes,
                                                        res_vtf.label(),
                                                    );
                                                }
                                                Err(e) => {
                                                    manifest.record(
                                                        virt_vtf,
                                                        AssetKind::Texture,
                                                        res_vtf
                                                            .loose_path()
                                                            .map(|p| p.to_path_buf()),
                                                        Some(dest_vtf),
                                                        AssetStatus::Failed(e.to_string()),
                                                        0,
                                                        "Texture VTF copy failed",
                                                    );
                                                }
                                            }
                                        } else {
                                            manifest.record(
                                                virt_vtf,
                                                AssetKind::Texture,
                                                None,
                                                None,
                                                AssetStatus::Missing,
                                                0,
                                                "Texture VTF missing in search paths",
                                            );
                                        }
                                    }

                                    // Copy .hdr.vtf if exists
                                    let rel_hdr_search = format!("materials/{}.hdr.vtf", norm_tex);
                                    let rel_hdr_dest = format!("materials/{}.hdr.vtf", tex_lower);
                                    if let Ok(virt_hdr) = VirtualAssetPath::parse(&rel_hdr_dest) {
                                        if let Some(res_hdr) =
                                            search_paths.resolve_asset(&rel_hdr_search)
                                        {
                                            let dest_hdr = output_dir.join(&rel_hdr_dest);
                                            if let Ok(bytes) =
                                                search_paths.extract_asset(&res_hdr, &dest_hdr)
                                            {
                                                manifest.record(
                                                    virt_hdr,
                                                    AssetKind::Texture,
                                                    res_hdr.loose_path().map(|p| p.to_path_buf()),
                                                    Some(dest_hdr),
                                                    AssetStatus::Extracted,
                                                    bytes,
                                                    res_hdr.label(),
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Extract linked materials ($include, $bottommaterial)
                    for getter in &getters.material_getters {
                        if let Some(values) = vmt_map.get(getter) {
                            for raw_val in values {
                                let norm_mat = normalize_texture_path(raw_val);
                                if !pm.contains(&norm_mat.to_lowercase()) {
                                    queue.push(norm_mat);
                                }
                            }
                        }
                    }

                    // If patch material or prefixed material, also resolve base material
                    if let Some(base_mat) = resolve_base_material(&clean) {
                        let base_key = base_mat.to_lowercase();
                        if !pm.contains(&base_key) {
                            queue.push(base_mat);
                        }
                    }
                }
            } else {
                let mut fallback_resolved = false;
                if let Some(base_mat) = resolve_base_material(&clean) {
                    let base_key = base_mat.to_lowercase();
                    if !pm.contains(&base_key) {
                        queue.push(base_mat);
                        fallback_resolved = true;
                    }
                }
                if !fallback_resolved {
                    manifest.record(
                        virt_vmt,
                        AssetKind::Material,
                        None,
                        None,
                        AssetStatus::Missing,
                        0,
                        "Material VMT not found in search paths",
                    );
                }
            }
        }
    };

    let extract_model = |mdl_name: &str,
                         pm: &mut HashSet<String>,
                         pt: &mut HashSet<String>,
                         pmodels: &mut HashSet<String>,
                         manifest: &mut Manifest| {
        let base_no_ext = if mdl_name.to_lowercase().ends_with(".mdl") {
            &mdl_name[..mdl_name.len() - 4]
        } else {
            mdl_name
        };

        let norm_base = normalize_source_path(base_no_ext);
        let base_lower = norm_base.to_lowercase();
        if pmodels.contains(&base_lower) || base_lower.is_empty() {
            return;
        }
        pmodels.insert(base_lower.clone());

        let mut found_mdl = false;

        for variant in MODEL_VARIANTS {
            let rel_var = format!("{}{}", norm_base, variant);
            if let Ok(virt_mdl) = VirtualAssetPath::parse(&rel_var) {
                if let Some(resolved) = search_paths.resolve_asset(&rel_var) {
                    let dest = output_dir.join(&rel_var);
                    if let Ok(bytes) = search_paths.extract_asset(&resolved, &dest) {
                        manifest.record(
                            virt_mdl,
                            AssetKind::Model,
                            resolved.loose_path().map(|p| p.to_path_buf()),
                            Some(dest),
                            AssetStatus::Extracted,
                            bytes,
                            resolved.label(),
                        );
                        if variant == ".mdl" {
                            found_mdl = true;
                        }
                    }
                }
            }
        }

        if found_mdl {
            let rel_mdl = format!("{}.mdl", norm_base);
            if let Some(resolved_mdl) = search_paths.resolve_asset(&rel_mdl) {
                if let Ok(mdl_bytes) = search_paths.read_asset(&resolved_mdl) {
                    if let Some(mdl_info) = parse_mdl_textures(&mdl_bytes) {
                        for dir in &mdl_info.texture_dirs {
                            for tex in &mdl_info.textures {
                                let sub_mat = format!("{}/{}", dir.trim_end_matches('/'), tex);
                                extract_material(&sub_mat, pm, pt, manifest);
                            }
                        }
                    }
                }
            }
        }
    };

    // 5. Extract Sounds
    if options.extract_sounds {
        let mut spin = Spinner::start("Extracting sounds...");
        let mut sound_set = root_assets.sounds;

        if root_assets.has_soundscapes {
            let sc_name = format!("scripts/soundscapes_{}.txt", map_name);
            if let Some(resolved_sc) = search_paths.resolve_asset(&sc_name) {
                let dest_sc = output_dir.join(&sc_name);
                if let Ok(bytes) = search_paths.extract_asset(&resolved_sc, &dest_sc) {
                    if let Ok(virt_sc) = VirtualAssetPath::parse(&sc_name) {
                        manifest.record(
                            virt_sc,
                            AssetKind::Script,
                            resolved_sc.loose_path().map(|p| p.to_path_buf()),
                            Some(dest_sc),
                            AssetStatus::Extracted,
                            bytes,
                            resolved_sc.label(),
                        );
                    }
                }
                if let Ok(sc_bytes) = search_paths.read_asset(&resolved_sc) {
                    let content = String::from_utf8_lossy(&sc_bytes);
                    let wave_re = regex::Regex::new(r#"(?i)"wave"\s+"([^"]+)""#).unwrap();
                    for cap in wave_re.captures_iter(&content) {
                        let clean = clean_sound_path(&cap[1]);
                        if !clean.is_empty() {
                            sound_set.insert(clean);
                        }
                    }
                }
            }
        }

        let mut sound_resolver = crate::sound::SoundscriptResolver::new();
        sound_resolver.load_from_search_paths(&search_paths);

        let mut all_waves = HashSet::new();
        for snd in &sound_set {
            let waves = sound_resolver.resolve_sound(snd);
            if waves.is_empty() {
                all_waves.insert(snd.clone());
            } else {
                for w in waves {
                    all_waves.insert(w);
                }
            }
        }

        for snd in &all_waves {
            let rel_snd = format!("sound/{}", snd);
            let dest_rel = format!("sound/{}", snd.to_lowercase());
            let Ok(virt_snd) = VirtualAssetPath::parse(&dest_rel) else {
                continue;
            };

            if let Some(resolved) = search_paths.resolve_asset(&rel_snd) {
                let dest = output_dir.join(&dest_rel);
                match search_paths.extract_asset(&resolved, &dest) {
                    Ok(bytes) => {
                        manifest.record(
                            virt_snd,
                            AssetKind::Sound,
                            resolved.loose_path().map(|p| p.to_path_buf()),
                            Some(dest),
                            AssetStatus::Extracted,
                            bytes,
                            resolved.label(),
                        );
                    }
                    Err(e) => {
                        manifest.record(
                            virt_snd,
                            AssetKind::Sound,
                            resolved.loose_path().map(|p| p.to_path_buf()),
                            Some(dest),
                            AssetStatus::Failed(e.to_string()),
                            0,
                            "Sound copy failed",
                        );
                    }
                }
            } else {
                manifest.record(
                    virt_snd,
                    AssetKind::Sound,
                    None,
                    None,
                    AssetStatus::Missing,
                    0,
                    "Sound not found in search paths",
                );
            }
        }
        spin.stop();
        let summary = manifest.summary();
        println!(
            "🔊 • ✅ {} sounds extracted",
            summary.sounds_extracted.to_string().green()
        );
    }

    // 6. Extract Materials
    if options.extract_materials {
        let mut spin = Spinner::start("Extracting materials & textures...");
        for mat in &root_assets.materials {
            extract_material(
                mat,
                &mut processed_materials,
                &mut processed_textures,
                &mut manifest,
            );
        }

        // Also extract and process map-specific patch materials from any mounted BSP pakfiles
        for bsp in &search_paths.bsps {
            let bsp_entries: Vec<String> = bsp.entries.keys().cloned().collect();
            for entry_name in bsp_entries {
                if entry_name.starts_with("materials/") && entry_name.ends_with(".vmt") {
                    let rel_mat = &entry_name["materials/".len()..entry_name.len() - 4];
                    extract_material(
                        rel_mat,
                        &mut processed_materials,
                        &mut processed_textures,
                        &mut manifest,
                    );
                }
            }
        }

        for vtf in &root_assets.direct_vtfs {
            let norm_tex = normalize_texture_path(vtf);
            let tex_lower = norm_tex.to_lowercase();
            if !processed_textures.contains(&tex_lower) {
                processed_textures.insert(tex_lower.clone());
                let rel_vtf_dest = format!("materials/{}.vtf", tex_lower);
                if let Ok(virt_vtf) = VirtualAssetPath::parse(&rel_vtf_dest) {
                    let rel_vtf = format!("materials/{}.vtf", norm_tex);
                    if let Some(res_vtf) = search_paths.resolve_asset(&rel_vtf) {
                        let dest_vtf = output_dir.join(&rel_vtf_dest);
                        if let Ok(bytes) = search_paths.extract_asset(&res_vtf, &dest_vtf) {
                            manifest.record(
                                virt_vtf,
                                AssetKind::Texture,
                                res_vtf.loose_path().map(|p| p.to_path_buf()),
                                Some(dest_vtf),
                                AssetStatus::Extracted,
                                bytes,
                                res_vtf.label(),
                            );
                        }
                    }
                }

                let rel_hdr_dest = format!("materials/{}.hdr.vtf", tex_lower);
                if let Ok(virt_hdr) = VirtualAssetPath::parse(&rel_hdr_dest) {
                    let rel_hdr = format!("materials/{}.hdr.vtf", norm_tex);
                    if let Some(res_hdr) = search_paths.resolve_asset(&rel_hdr) {
                        let dest_hdr = output_dir.join(&rel_hdr_dest);
                        if let Ok(bytes) = search_paths.extract_asset(&res_hdr, &dest_hdr) {
                            manifest.record(
                                virt_hdr,
                                AssetKind::Texture,
                                res_hdr.loose_path().map(|p| p.to_path_buf()),
                                Some(dest_hdr),
                                AssetStatus::Extracted,
                                bytes,
                                res_hdr.label(),
                            );
                        }
                    }
                }
            }
        }

        spin.stop();
        let summary = manifest.summary();
        println!(
            "🧱 • ✅ {} materials & {} textures extracted",
            summary.materials_extracted.to_string().green(),
            summary.textures_extracted.to_string().green()
        );
    }

    // 7. Extract Models
    if options.extract_models {
        let mut spin = Spinner::start("Extracting 3D models...");
        for mdl in &root_assets.models {
            extract_model(
                mdl,
                &mut processed_materials,
                &mut processed_textures,
                &mut processed_models,
                &mut manifest,
            );
        }
        spin.stop();
        let summary = manifest.summary();
        println!(
            "📐 • ✅ {} model files extracted",
            summary.models_extracted.to_string().green()
        );
    }

    let mut bsp_packed = false;

    // 8. Ready to Publish
    if options.ready_to_publish {
        let mut spin = Spinner::start("Processing auxiliary publish files & BSP packing...");
        let companion_variants = [
            format!("maps/{}.ain", map_name),
            format!("maps/graphs/{}.ain", map_name),
            format!("maps/{}.nav", map_name),
            format!("maps/thumb/{}.png", map_name),
            format!("maps/{}_particles.txt", map_name),
            format!("scripts/soundscapes_{}.txt", map_name),
        ];

        for variant in companion_variants {
            if let Ok(virt_p) = VirtualAssetPath::parse(&variant) {
                if let Some(resolved) = search_paths.resolve_asset(&variant) {
                    let dest = output_dir.join(&variant);
                    if let Ok(bytes) = search_paths.extract_asset(&resolved, &dest) {
                        manifest.record(
                            virt_p,
                            AssetKind::Auxiliary,
                            resolved.loose_path().map(|p| p.to_path_buf()),
                            Some(dest),
                            AssetStatus::Extracted,
                            bytes,
                            resolved.label(),
                        );
                    }
                }
            }
        }

        // Particle files
        let particles_rel = format!("maps/{}_particles.txt", map_name);
        if let Some(res_part) = search_paths.resolve_asset(&particles_rel) {
            if let Ok(part_bytes) = search_paths.read_asset(&res_part) {
                let content = String::from_utf8_lossy(&part_bytes);
                let pcf_re = regex::Regex::new(r#"(?i)"file"\s+"([^"]+)""#).unwrap();
                let vmt_re = regex::Regex::new(r"[\w/\.-]+\.vmt").unwrap();
                for cap in pcf_re.captures_iter(&content) {
                    let rel_pcf = normalize_source_path(&cap[1]);
                    if let Ok(virt_pcf) = VirtualAssetPath::parse(&rel_pcf) {
                        if let Some(res_pcf) = search_paths.resolve_asset(&rel_pcf) {
                            let dest_pcf = output_dir.join(&rel_pcf);
                            if let Ok(bytes) = search_paths.extract_asset(&res_pcf, &dest_pcf) {
                                manifest.record(
                                    virt_pcf,
                                    AssetKind::Particle,
                                    res_pcf.loose_path().map(|p| p.to_path_buf()),
                                    Some(dest_pcf),
                                    AssetStatus::Extracted,
                                    bytes,
                                    res_pcf.label(),
                                );
                            }

                            if let Ok(pcf_bytes) = search_paths.read_asset(&res_pcf) {
                                let text = String::from_utf8_lossy(&pcf_bytes);
                                for m in vmt_re.find_iter(&text) {
                                    let vmt_ref = m.as_str().replace("materials/", "");
                                    extract_material(
                                        &vmt_ref,
                                        &mut processed_materials,
                                        &mut processed_textures,
                                        &mut manifest,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        // Copy instance VMF files
        for inst in &instance_files {
            let rel_dest = format!("maps/{}", inst.relative_path);
            if let Ok(virt_inst) = VirtualAssetPath::parse(&rel_dest) {
                let dest_inst = output_dir.join(&rel_dest);
                if let Ok(bytes) = safe_copy_file(&inst.source_path, &dest_inst) {
                    manifest.record(
                        virt_inst,
                        AssetKind::Instance,
                        Some(inst.source_path.clone()),
                        Some(dest_inst),
                        AssetStatus::Extracted,
                        bytes,
                        "Sub-VMF func_instance",
                    );
                }
            }
        }

        // Copy root VMF file
        let root_vmf_rel = format!("maps/{}.vmf", map_name);
        if let Ok(virt_vmf) = VirtualAssetPath::parse(&root_vmf_rel) {
            let dest_root_vmf = output_dir.join(&root_vmf_rel);
            if let Ok(bytes) = safe_copy_file(&options.vmf_path, &dest_root_vmf) {
                manifest.record(
                    virt_vmf,
                    AssetKind::Other,
                    Some(options.vmf_path.clone()),
                    Some(dest_root_vmf),
                    AssetStatus::Extracted,
                    bytes,
                    "Source VMF file",
                );
            }
        }

        // Copy detail VBSP files and extract referenced models
        let vbsp_mdl_re = regex::Regex::new(r#"(?i)"model"\s+"([^"]+\.mdl)""#).unwrap();
        for vbsp_name in &root_assets.detail_vbsps {
            let rel_vbsp = normalize_source_path(vbsp_name);
            if let Ok(virt_vbsp) = VirtualAssetPath::parse(&rel_vbsp) {
                if let Some(res_vbsp) = search_paths.resolve_asset(&rel_vbsp) {
                    let dest_vbsp = output_dir.join(&rel_vbsp);
                    if let Ok(bytes) = search_paths.extract_asset(&res_vbsp, &dest_vbsp) {
                        manifest.record(
                            virt_vbsp,
                            AssetKind::Other,
                            res_vbsp.loose_path().map(|p| p.to_path_buf()),
                            Some(dest_vbsp),
                            AssetStatus::Extracted,
                            bytes,
                            "Detail VBSP file",
                        );
                    }

                    // Parse the .vbsp content to extract any foliage models referenced inside
                    if let Ok(vbsp_bytes) = search_paths.read_asset(&res_vbsp) {
                        let text = String::from_utf8_lossy(&vbsp_bytes);
                        for cap in vbsp_mdl_re.captures_iter(&text) {
                            let mdl_ref = normalize_source_path(&cap[1]);
                            if options.extract_models {
                                extract_model(
                                    &mdl_ref,
                                    &mut processed_materials,
                                    &mut processed_textures,
                                    &mut processed_models,
                                    &mut manifest,
                                );
                            }
                        }
                    }
                } else {
                    manifest.record(
                        virt_vbsp,
                        AssetKind::Other,
                        None,
                        None,
                        AssetStatus::Missing,
                        0,
                        "Detail VBSP file not found in search paths",
                    );
                }
            }
        }

        // Auxiliary BSP Packing with bspzip.exe (packing ONLY companion map files into the BSP)
        let bsp_rel = format!("maps/{}.bsp", map_name);
        if let Some(resolved_bsp) = search_paths.resolve(&bsp_rel) {
            if let Some(parent) = search_paths.primary_path.parent() {
                let bspzip = parent.join("bin").join("bspzip.exe");
                let bspzip_entries = manifest.generate_bspzip_list();

                if bspzip.exists() && !bspzip_entries.is_empty() {
                    let zipfile_path = output_dir.join("zipfiles.txt");
                    if fs::write(&zipfile_path, bspzip_entries.join("\n")).is_ok() {
                        let dest_bsp = output_dir.join("maps").join(format!("{}.bsp", map_name));
                        if let Some(dest_parent) = dest_bsp.parent() {
                            let _ = fs::create_dir_all(dest_parent);
                        }

                        let output = Command::new(&bspzip)
                            .args([
                                "-addorupdatelist",
                                &resolved_bsp.to_string_lossy(),
                                &zipfile_path.to_string_lossy(),
                                &dest_bsp.to_string_lossy(),
                            ])
                            .output();

                        if let Ok(out) = output {
                            if out.status.success() && dest_bsp.exists() {
                                bsp_packed = true;
                            }
                        }
                        let _ = fs::remove_file(&zipfile_path);
                    }
                }
            }
        }

        spin.stop();
        println!("📤 • ✅ Ready to Publish processed");
        if bsp_packed {
            println!("📦 • ✅ Auxiliary companion files packed into BSP");
        }
    }

    let manifest_file = output_dir.join("manifest.json");
    let _ = manifest.write_json(&manifest_file);

    let summary = manifest.summary();
    let stats = ScraperStats {
        instances_count: instance_files.len(),
        materials_count: summary.materials_extracted,
        textures_count: summary.textures_extracted,
        models_count: summary.models_extracted,
        sounds_count: summary.sounds_extracted,
        publish_files_count: summary.total_extracted,
        bsp_packed,
    };

    println!(
        "\n📋 Manifest written to: {}",
        manifest_file.display().to_string().cyan()
    );
    println!(
        "   Total extracted: {} assets ({:.2} MB)",
        summary.total_extracted.to_string().green(),
        summary.total_bytes_extracted as f64 / 1_048_576.0
    );

    Ok(stats)
}
