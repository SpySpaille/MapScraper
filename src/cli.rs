use crate::scraper::ScraperOptions;
use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct CliArgs {
    pub game_path: Option<PathBuf>,
    pub vmf_path: Option<PathBuf>,
    pub output_dir: Option<PathBuf>,
    pub custom_mounts: Vec<String>,
    pub extract_materials: Option<bool>,
    pub extract_models: Option<bool>,
    pub extract_sounds: Option<bool>,
    pub ready_to_publish: Option<bool>,
    pub is_cli_mode: bool,
}

pub fn parse_cli_args(args: &[String]) -> Result<CliArgs, String> {
    let mut cli = CliArgs::default();
    let mut i = 1;

    while i < args.len() {
        match args[i].as_str() {
            "--game" | "-g" => {
                i += 1;
                if i >= args.len() {
                    return Err("Missing argument for --game".to_string());
                }
                cli.game_path = Some(PathBuf::from(&args[i]));
                cli.is_cli_mode = true;
            }
            "--vmf" | "-m" => {
                i += 1;
                if i >= args.len() {
                    return Err("Missing argument for --vmf".to_string());
                }
                cli.vmf_path = Some(PathBuf::from(&args[i]));
                cli.is_cli_mode = true;
            }
            "--output" | "-o" => {
                i += 1;
                if i >= args.len() {
                    return Err("Missing argument for --output".to_string());
                }
                cli.output_dir = Some(PathBuf::from(&args[i]));
                cli.is_cli_mode = true;
            }
            "--mount" => {
                i += 1;
                if i >= args.len() {
                    return Err("Missing argument for --mount".to_string());
                }
                cli.custom_mounts.push(args[i].clone());
                cli.is_cli_mode = true;
            }
            "--materials" => {
                cli.extract_materials = Some(true);
                cli.is_cli_mode = true;
            }
            "--no-materials" => {
                cli.extract_materials = Some(false);
                cli.is_cli_mode = true;
            }
            "--models" => {
                cli.extract_models = Some(true);
                cli.is_cli_mode = true;
            }
            "--no-models" => {
                cli.extract_models = Some(false);
                cli.is_cli_mode = true;
            }
            "--sounds" => {
                cli.extract_sounds = Some(true);
                cli.is_cli_mode = true;
            }
            "--no-sounds" => {
                cli.extract_sounds = Some(false);
                cli.is_cli_mode = true;
            }
            "--publish" | "-p" => {
                cli.ready_to_publish = Some(true);
                cli.is_cli_mode = true;
            }
            _ => {}
        }
        i += 1;
    }

    Ok(cli)
}

pub fn to_scraper_options(cli: CliArgs) -> Result<ScraperOptions, String> {
    let game_path = cli
        .game_path
        .ok_or("Game path (--game) is required in CLI mode")?;
    let vmf_path = cli
        .vmf_path
        .ok_or("VMF map path (--vmf) is required in CLI mode")?;

    if !game_path.exists() {
        return Err(format!(
            "Specified game path does not exist: {}",
            game_path.display()
        ));
    }
    if !vmf_path.exists() {
        return Err(format!(
            "Specified VMF file does not exist: {}",
            vmf_path.display()
        ));
    }

    Ok(ScraperOptions {
        game_path,
        vmf_path,
        extract_materials: cli.extract_materials.unwrap_or(true),
        extract_models: cli.extract_models.unwrap_or(true),
        extract_sounds: cli.extract_sounds.unwrap_or(true),
        ready_to_publish: cli.ready_to_publish.unwrap_or(false),
        custom_mounts: cli.custom_mounts,
        output_dir: cli.output_dir,
    })
}

pub fn print_help() {
    println!(
        r#"
MapScraper - Source Engine Content Scraper

USAGE:
    MapScraper.exe [OPTIONS]

OPTIONS:
    -g, --game <PATH>       Path to game root directory (e.g. C:\Steam\...\garrysmod)
    -m, --vmf <PATH>        Path to map VMF file
    -o, --output <PATH>     Custom output directory (default: ./<mapname>)
        --mount <PATH>      Custom mount path (can be specified multiple times)
        --materials         Extract materials and textures (default: true)
        --no-materials      Do not extract materials
        --models            Extract 3D models and materials (default: true)
        --no-models         Do not extract models
        --sounds            Extract sounds (default: true)
        --no-sounds         Do not extract sounds
    -p, --publish           Ready to publish (extract auxiliary files and pack BSP)
    -h, --help              Print help information
    -v, --version           Print version information
"#
    );
}
