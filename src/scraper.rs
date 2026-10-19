use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use colored::Colorize;

use crate::getters::Getters;
use crate::instance::{InstanceFile, InstanceResolver};
use crate::mdl::{parse_mdl_textures, MODEL_VARIANTS};
use crate::mounts::SearchPathManager;
use crate::utils::{clean_sound_path, normalize_source_path, safe_copy_file};
use crate::vmf::parse_vmf;
use crate::vmt::{normalize_texture_path, parse_vmt};

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

#[allow(dead_code)]
pub struct ScraperStats {
    pub instances_count: usize,
    pub materials_count: usize,
    pub textures_count: usize,
    pub models_count: usize,
    pub sounds_count: usize,
    pub publish_files_count: usize,
    pub bsp_packed: bool,
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

    fs::create_dir_all(&output_dir)?;

    // 1. Initialize Mounts
    let search_paths = SearchPathManager::new(&options.game_path, &options.custom_mounts);
    println!("\n📂 Game Path: {}", search_paths.primary_path.display().to_string().cyan());

    let mounted: Vec<_> = search_paths.all_paths.iter().filter(|p| p.label != "Primary Game").collect();
    if !mounted.is_empty() {
        println!("🔗 Mounted directories detected ({}):", mounted.len());
        for m in &mounted {
            println!("   - {}: {}", m.label.green(), m.path.display().to_string().white());
        }
    }

    // 2. Load Getters
    let getters = Getters::load();

    // 3. Read and parse root VMF
    print!("- Parsing VMF map file... ");
    let vmf_content = fs::read_to_string(&options.vmf_path).unwrap_or_default();
    let mut root_assets = parse_vmf(&vmf_content, &getters.sound_getters);
    println!("{}", "Done!".green());

    // 4. Resolve func_instances
    let mut instance_files: Vec<InstanceFile> = Vec::new();
    if !root_assets.instances.is_empty() {
        print!("- Resolving {} func_instance(s)... ", root_assets.instances.len());
        let mut resolver = InstanceResolver::new(&search_paths, &getters.sound_getters);
        let (inst_assets, files) = resolver.process_instances(&root_assets.instances, &options.vmf_path);
        instance_files = files;

        root_assets.materials.extend(inst_assets.materials);
        root_assets.models.extend(inst_assets.models);
        root_assets.sounds.extend(inst_assets.sounds);
        root_assets.direct_vtfs.extend(inst_assets.direct_vtfs);
        println!("{}", "Done!".green());
        println!("🔄 • ✅ {} func_instance VMFs processed", instance_files.len().to_string().green());
    }

    let mut stats = ScraperStats {
        instances_count: instance_files.len(),
        materials_count: 0,
        textures_count: 0,
        models_count: 0,
        sounds_count: 0,
        publish_files_count: 0,
        bsp_packed: false,
    };

    // Shared material extractor tracker
    let mut processed_materials: HashSet<String> = HashSet::new();
    let mut processed_textures: HashSet<String> = HashSet::new();

    // Helper closure to extract a material and its referenced textures
    let extract_material = |mat_name: &str,
                            pm: &mut HashSet<String>,
                            pt: &mut HashSet<String>,
                            mat_count: &mut usize,
                            tex_count: &mut usize| {
        let mut queue = vec![normalize_source_path(mat_name)];

        while let Some(current_mat) = queue.pop() {
            let clean = if current_mat.to_lowercase().ends_with(".vmt") {
                current_mat[..current_mat.len() - 4].to_string()
            } else {
                current_mat
            };
            let lower = clean.to_lowercase();
            if pm.contains(&lower) || lower.is_empty() {
                continue;
            }
            pm.insert(lower);

            let rel_vmt = format!("materials/{}.vmt", clean);
            if let Some(resolved_vmt) = search_paths.resolve(&rel_vmt) {
                let dest_vmt = output_dir.join(&rel_vmt);
                let _ = safe_copy_file(&resolved_vmt, &dest_vmt);
                *mat_count += 1;

                if let Ok(vmt_data) = fs::read_to_string(&resolved_vmt) {
                    let vmt_map = parse_vmt(&vmt_data);

                    // Extract textures
                    for getter in &getters.texture_getters {
                        if let Some(values) = vmt_map.get(getter) {
                            for raw_val in values {
                                let norm_tex = normalize_texture_path(raw_val);
                                let tex_lower = norm_tex.to_lowercase();
                                if !pt.contains(&tex_lower) && !tex_lower.is_empty() {
                                    pt.insert(tex_lower);

                                    // Copy .vtf
                                    let rel_vtf = format!("materials/{}.vtf", norm_tex);
                                    if let Some(res_vtf) = search_paths.resolve(&rel_vtf) {
                                        let dest_vtf = output_dir.join(&rel_vtf);
                                        let _ = safe_copy_file(&res_vtf, &dest_vtf);
                                        *tex_count += 1;
                                    }

                                    // Copy .hdr.vtf if exists
                                    let rel_hdr = format!("materials/{}.hdr.vtf", norm_tex);
                                    if let Some(res_hdr) = search_paths.resolve(&rel_hdr) {
                                        let dest_hdr = output_dir.join(&rel_hdr);
                                        let _ = safe_copy_file(&res_hdr, &dest_hdr);
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
                }
            }
        }
    };

    // 5. Extract Sounds
    if options.extract_sounds {
        print!("🔊 Extracting sounds... ");
        let mut sound_set = root_assets.sounds;

        // Check soundscapes
        if root_assets.has_soundscapes {
            let sc_name = format!("scripts/soundscapes_{}.txt", map_name);
            if let Some(resolved_sc) = search_paths.resolve(&sc_name) {
                let _ = safe_copy_file(&resolved_sc, output_dir.join(&sc_name));
                if let Ok(content) = fs::read_to_string(&resolved_sc) {
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

        for snd in &sound_set {
            let rel_snd = format!("sound/{}", snd);
            if let Some(resolved) = search_paths.resolve(&rel_snd) {
                let dest = output_dir.join("sound").join(snd.to_lowercase());
                if safe_copy_file(&resolved, &dest).is_ok() {
                    stats.sounds_count += 1;
                }
            }
        }
        println!("{}", "Done!".green());
        println!("🔊 • ✅ {} sounds extracted", stats.sounds_count.to_string().green());
    }

    // 6. Extract Materials
    if options.extract_materials {
        print!("🧱 Extracting materials & textures... ");
        for mat in &root_assets.materials {
            extract_material(
                mat,
                &mut processed_materials,
                &mut processed_textures,
                &mut stats.materials_count,
                &mut stats.textures_count,
            );
        }

        // Direct VTFs (startexture)
        for vtf in &root_assets.direct_vtfs {
            let norm_tex = normalize_texture_path(vtf);
            let tex_lower = norm_tex.to_lowercase();
            if !processed_textures.contains(&tex_lower) {
                processed_textures.insert(tex_lower);
                let rel_vtf = format!("materials/{}.vtf", norm_tex);
                if let Some(res_vtf) = search_paths.resolve(&rel_vtf) {
                    let dest_vtf = output_dir.join(&rel_vtf);
                    let _ = safe_copy_file(&res_vtf, &dest_vtf);
                    stats.textures_count += 1;
                }
            }
        }

        println!("{}", "Done!".green());
        println!(
            "🧱 • ✅ {} materials & {} textures extracted",
            stats.materials_count.to_string().green(),
            stats.textures_count.to_string().green()
        );
    }

    // 7. Extract Models
    if options.extract_models {
        print!("📐 Extracting 3D models... ");
        for mdl in &root_assets.models {
            let base_no_ext = if mdl.to_lowercase().ends_with(".mdl") {
                &mdl[..mdl.len() - 4]
            } else {
                mdl.as_str()
            };

            let mut found_mdl = false;

            // Copy all companion files
            for variant in MODEL_VARIANTS {
                let rel_var = format!("{}{}", base_no_ext, variant);
                if let Some(resolved) = search_paths.resolve(&rel_var) {
                    let dest = output_dir.join(&rel_var);
                    let _ = safe_copy_file(&resolved, &dest);
                    if variant == ".mdl" {
                        found_mdl = true;
                    }
                }
            }

            if found_mdl {
                stats.models_count += 1;
                let rel_mdl = format!("{}.mdl", base_no_ext);
                if let Some(resolved_mdl) = search_paths.resolve(&rel_mdl) {
                    if let Ok(mdl_bytes) = fs::read(&resolved_mdl) {
                        if let Some(mdl_info) = parse_mdl_textures(&mdl_bytes) {
                            for dir in &mdl_info.texture_dirs {
                                for tex in &mdl_info.textures {
                                    let sub_mat = format!("{}/{}", dir.trim_end_matches('/'), tex);
                                    extract_material(
                                        &sub_mat,
                                        &mut processed_materials,
                                        &mut processed_textures,
                                        &mut stats.materials_count,
                                        &mut stats.textures_count,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
        println!("{}", "Done!".green());
        println!("📐 • ✅ {} models extracted", stats.models_count.to_string().green());
    }

    // 8. Ready to Publish
    if options.ready_to_publish {
        print!("📤 Processing auxiliary publish files & BSP packing... ");
        let companion_variants = [
            format!("maps/{}.ain", map_name),
            format!("maps/graphs/{}.ain", map_name),
            format!("maps/{}.nav", map_name),
            format!("maps/thumb/{}.png", map_name),
            format!("maps/{}_particles.txt", map_name),
        ];

        let mut files_to_pack: Vec<(String, PathBuf)> = Vec::new();

        for variant in companion_variants {
            if let Some(resolved) = search_paths.resolve(&variant) {
                let dest = output_dir.join(&variant);
                if safe_copy_file(&resolved, &dest).is_ok() {
                    stats.publish_files_count += 1;
                    files_to_pack.push((variant, resolved));
                }
            }
        }

        // Particle files
        let particles_rel = format!("maps/{}_particles.txt", map_name);
        if let Some(res_part) = search_paths.resolve(&particles_rel) {
            if let Ok(content) = fs::read_to_string(&res_part) {
                let pcf_re = regex::Regex::new(r#"(?i)"file"\s+"([^"]+)""#).unwrap();
                for cap in pcf_re.captures_iter(&content) {
                    let rel_pcf = normalize_source_path(&cap[1]);
                    if let Some(res_pcf) = search_paths.resolve(&rel_pcf) {
                        let dest_pcf = output_dir.join(&rel_pcf);
                        let _ = safe_copy_file(&res_pcf, &dest_pcf);
                        stats.publish_files_count += 1;

                        // Inspect PCF for referenced VMT
                        if let Ok(pcf_bytes) = fs::read(&res_pcf) {
                            let text = String::from_utf8_lossy(&pcf_bytes);
                            let vmt_re = regex::Regex::new(r"[\w/\.-]+\.vmt").unwrap();
                            for m in vmt_re.find_iter(&text) {
                                let vmt_ref = m.as_str().replace("materials/", "");
                                extract_material(
                                    &vmt_ref,
                                    &mut processed_materials,
                                    &mut processed_textures,
                                    &mut stats.materials_count,
                                    &mut stats.textures_count,
                                );
                            }
                        }
                    }
                }
            }
        }

        // Copy instance VMF files
        for inst in &instance_files {
            let dest_inst = output_dir.join("maps").join(&inst.relative_path);
            let _ = safe_copy_file(&inst.source_path, &dest_inst);
            stats.publish_files_count += 1;
        }

        // Copy root VMF file
        let dest_root_vmf = output_dir.join("maps").join(format!("{}.vmf", map_name));
        let _ = safe_copy_file(&options.vmf_path, &dest_root_vmf);
        stats.publish_files_count += 1;

        // BSP Packing with bspzip.exe
        let bsp_rel = format!("maps/{}.bsp", map_name);
        if let Some(resolved_bsp) = search_paths.resolve(&bsp_rel) {
            if let Some(parent) = search_paths.primary_path.parent() {
                let bspzip = parent.join("bin").join("bspzip.exe");
                if bspzip.exists() && !files_to_pack.is_empty() {
                    let zipfile_path = output_dir.join("zipfiles.txt");
                    let mut lines = Vec::new();
                    for (rel, disk) in &files_to_pack {
                        lines.push(rel.replace('\\', "/"));
                        lines.push(disk.to_string_lossy().to_string());
                    }
                    if fs::write(&zipfile_path, lines.join("\n")).is_ok() {
                        let dest_bsp = output_dir.join("maps").join(format!("{}.bsp", map_name));
                        if let Some(p) = dest_bsp.parent() {
                            let _ = fs::create_dir_all(p);
                        }
                        let status = Command::new(&bspzip)
                            .arg("-addorupdatelist")
                            .arg(&resolved_bsp)
                            .arg(&zipfile_path)
                            .arg(&dest_bsp)
                            .status();

                        let _ = fs::remove_file(&zipfile_path);
                        if let Ok(st) = status {
                            stats.bsp_packed = st.success();
                        }
                    }
                }
            }
        }

        println!("{}", "Done!".green());
        println!("📤 • ✅ {} additional files extracted", stats.publish_files_count.to_string().green());
        if stats.bsp_packed {
            println!("📦 • ✅ Files packed into BSP");
        }
    }

    println!("\n{}", "Extraction completed 🎉".green().bold());
    println!("📁 Output directory: {}", output_dir.display().to_string().cyan());
    println!("{}", "Thanks for using MapScraper 👍".white());

    Ok(stats)
}
