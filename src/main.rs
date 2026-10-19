use colored::Colorize;
use mapscraper::config::{clear_config, load_config, save_config, Config};
use mapscraper::scraper::{run_scraper, ScraperOptions};
use mapscraper::utils::clean_quotes;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

const CURRENT_VERSION: &str = "1.8.1";
const GITHUB_USER: &str = "SpySpaille";

fn print_banner() {
    let ascii = r#"
  __  __            ____                                 
 |  \/  | __ _ _ __/ ___|  ___ _ __ __ _ _ __   ___ _ __ 
 | |\/| |/ _` | '_ \___ \ / __| '__/ _` | '_ \ / _ \ '__|
 | |  | | (_| | |_) |__) | (__| | | (_| | |_) |  __/ |   
 |_|  |_|\__,_| .__/____/ \___|_|  \__,_| .__/ \___|_|   
              |_|                       |_|              
"#;
    println!("{}", ascii.green());
    println!(
        "{} {} {}",
        "Made by".white(),
        "SpySpaille".white().bold(),
        "with ❤️".red()
    );

    // Check version
    let latest = check_latest_version();
    match latest {
        Some(v) if v == CURRENT_VERSION => {
            println!("- {}: {}\n", "Version".green(), CURRENT_VERSION);
        }
        Some(v) => {
            println!(
                "- {}: {} {}\n- {}: {} {}\n",
                "Version".green(),
                CURRENT_VERSION,
                "(Outdated)".red(),
                "Latest Version".green(),
                v,
                format!("(https://github.com/{}/MapScraper)", GITHUB_USER).white()
            );
        }
        None => {
            println!("- {}: {}\n", "Version".green(), CURRENT_VERSION);
        }
    }
}

fn check_latest_version() -> Option<String> {
    let url = format!(
        "https://api.github.com/repos/{}/MapScraper/releases/latest",
        GITHUB_USER
    );
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(2))
        .build();

    let res = agent
        .get(&url)
        .set("User-Agent", "MapScraper-Rust")
        .call()
        .ok()?;
    let json: serde_json::Value = serde_json::from_reader(res.into_reader()).ok()?;
    let tag = json.get("tag_name")?.as_str()?;
    Some(tag.trim_start_matches('v').to_string())
}

fn prompt(message: &str) -> String {
    print!("{}", message);
    let _ = io::stdout().flush();
    let mut input = String::new();
    let _ = io::stdin().lock().read_line(&mut input);
    clean_quotes(input.trim())
}

fn prompt_yn(message: &str) -> bool {
    let prompt_msg = format!(
        "{} {} ",
        message,
        "[\x1b[32my\x1b[37m/\x1b[31mn\x1b[37m] >".white()
    );
    loop {
        let resp = prompt(&prompt_msg).to_lowercase();
        if resp == "y" || resp == "yes" {
            return true;
        } else if resp == "n" || resp == "no" {
            return false;
        }
        println!("  {}", "Invalid input (type 'y' or 'n')".red());
    }
}

#[allow(dead_code)]
fn prompt_yn_nested(message: &str) -> bool {
    let prompt_msg = format!(
        "  \x1b[90m└─\x1b[0m {} {} ",
        message,
        "[\x1b[32my\x1b[37m/\x1b[31mn\x1b[37m] >".white()
    );
    loop {
        let resp = prompt(&prompt_msg).to_lowercase();
        if resp == "y" || resp == "yes" {
            return true;
        } else if resp == "n" || resp == "no" {
            return false;
        }
        println!("    {}", "Invalid input (type 'y' or 'n')".red());
    }
}

fn wait_for_exit() {
    println!("\n{}", "Press Enter to close the program...".white());
    let mut buf = String::new();
    let _ = io::stdin().lock().read_line(&mut buf);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--version" || a == "-v") {
        println!("MapScraper v{}", CURRENT_VERSION);
        return;
    }
    if args.iter().any(|a| a == "--help" || a == "-h") {
        mapscraper::cli::print_help();
        return;
    }

    if let Ok(cli_args) = mapscraper::cli::parse_cli_args(&args) {
        if cli_args.is_cli_mode {
            match mapscraper::cli::to_scraper_options(cli_args) {
                Ok(options) => match run_scraper(options) {
                    Ok(_) => {
                        println!(
                            "\n{}",
                            "Extraction completed successfully 🎉".green().bold()
                        );
                        std::process::exit(0);
                    }
                    Err(e) => {
                        eprintln!("\n{} {}", "Error during extraction:".red().bold(), e);
                        std::process::exit(1);
                    }
                },
                Err(err) => {
                    eprintln!("{} {}", "CLI Error:".red().bold(), err);
                    mapscraper::cli::print_help();
                    std::process::exit(1);
                }
            }
        }
    }

    print_banner();
    println!("📜 Please enter the following information:\n");

    let mut cfg = load_config();
    let mut game_path: Option<PathBuf> = None;

    if let Some(ref saved) = cfg.gamepath {
        let reuse = prompt_yn(&format!("💾 Use last Game Path ? ({})", saved.cyan()));
        if reuse {
            let p = PathBuf::from(saved);
            if p.join("materials").exists() || p.join("gameinfo.txt").exists() {
                game_path = Some(p);
            } else {
                println!(
                    "  {}",
                    "Saved game path is no longer valid (missing materials/ or gameinfo.txt)".red()
                );
                clear_config();
                cfg.gamepath = None;
            }
        } else {
            clear_config();
            cfg.gamepath = None;
        }
    }

    if game_path.is_none() {
        loop {
            let prompt_prefix = if cfg.gamepath.is_some() {
                "  \x1b[90m└─\x1b[0m 📂 Game Path (e.g. C:\\Steam\\steamapps\\common\\GarrysMod\\garrysmod) > "
            } else {
                "📂 Game Path (e.g. C:\\Steam\\steamapps\\common\\GarrysMod\\garrysmod) > "
            };
            let input = prompt(prompt_prefix);
            let p = PathBuf::from(&input);
            if p.join("materials").exists() || p.join("gameinfo.txt").exists() {
                save_config(&Config {
                    gamepath: Some(input),
                });
                game_path = Some(p);
                break;
            }
            println!(
                "  {}",
                "Invalid game path: directory does not contain materials/ or gameinfo.txt".red()
            );
        }
    }

    let game_path = game_path.unwrap();

    let vmf_path: PathBuf = loop {
        let input = prompt("📄 VMF File (C:\\path\\to\\map.vmf) > ");
        let p = PathBuf::from(&input);
        if p.exists()
            && p.extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("vmf"))
        {
            break p;
        }
        println!("{}", "Invalid file: must be an existing .vmf file".red());
    };

    let extract_materials = prompt_yn("🧱 Extract Materials");
    let extract_models = prompt_yn("📐 Extract Models");
    let extract_sounds = prompt_yn("🔊 Extract Sounds");
    let ready_to_publish = prompt_yn("📤 Ready to Publish (all other contents & pack)");

    let options = ScraperOptions {
        game_path,
        vmf_path,
        extract_materials,
        extract_models,
        extract_sounds,
        ready_to_publish,
        custom_mounts: Vec::new(),
        output_dir: None,
    };

    if let Err(e) = run_scraper(options) {
        eprintln!("\n❌ {}: {}", "Error executing scraper".red(), e);
    }

    wait_for_exit();
}
