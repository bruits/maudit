use std::{
    path::{Path, PathBuf},
    process::Stdio,
};

use colored::Colorize;
use flate2::read::GzDecoder;
use inquire::{validator::Validation, Confirm, Select, Text};
use quanta::Instant;
use rand::seq::IndexedRandom;
use spinach::{Color, Spinner};
use toml_edit::DocumentMut;
use tracing::{debug, error, info};

mod names;
mod render_config;
use names::generate_directory_name;
use render_config::get_render_config;

use crate::logging::format_elapsed_time;

const REPO_TAR_URL: &str = "https://api.github.com/repos/bruits/maudit/tarball/main";

const INTROS: [&str; 6] = [
    "Let the coronation begin.",
    "The coronation shall begin.",
    "A new era begins.",
    "A new chapter unfolds.",
    "A reign begins anew.",
    "History is made today.",
];

pub fn start_new_project(dry_run: &bool) {
    if *dry_run {
        debug!("Dry run enabled");
    }

    inquire::set_global_render_config(get_render_config());

    let cargo_search = std::process::Command::new("cargo")
        .arg("search")
        .arg("maudit")
        .args(["--limit", "1"])
        .current_dir(std::env::temp_dir()) // `cargo search` sometimes can fail in certain directories, so we use a temp dir
        .output()
        .expect("Failed to run cargo info maudit");

    let maudit_version = if cargo_search.status.success() {
        let output = String::from_utf8_lossy(&cargo_search.stdout).to_string();
        format!(
            "(v{})",
            output
                .lines()
                .next()
                .and_then(|line| {
                    let start = line.find('"')?;
                    let end = line[start + 1..].find('"')?;
                    Some(line[start + 1..start + 1 + end].to_string())
                })
                .unwrap_or_else(|| "unknown".to_string())
        )
    } else {
        "".to_string()
    };

    println!();
    match maudit_version.is_empty() {
        true => {
            info!(name: "SKIP_FORMAT", "👑 {} {}!", "Welcome to".bold(), "Maudit".red().to_string().bold(), )
        }
        false => {
            info!(name: "SKIP_FORMAT", "👑 {} {}! {}", "Welcome to".bold(), "Maudit".red().to_string().bold(), maudit_version.dimmed())
        }
    }

    let rng = &mut rand::rng();
    let intro = INTROS.choose(rng).unwrap();
    info!(name: "SKIP_FORMAT", "   {}", intro.dimmed());
    println!();

    let directory_name = format!("./{}", generate_directory_name(rng));
    let project_path = Text::new("Where should we create the project?")
        .with_formatter(&|i| {
            if i.is_empty() {
                return directory_name.clone();
            }

            i.to_owned()
        })
        .with_validators(&[
            Box::new(|s: &str| {
                // Don't check if the directory already exists if the user wants to use the current directory
                if s == "." {
                    return Ok(Validation::Valid);
                }

                if std::path::Path::new(&s).exists() {
                    Ok(Validation::Invalid(
                        "A directory with this name already exists".into(),
                    ))
                } else {
                    Ok(Validation::Valid)
                }
            }),
            Box::new(|s: &str| {
                if has_invalid_filepath_chars(s) {
                    Ok(Validation::Invalid(
                        "The directory name contains invalid characters".into(),
                    ))
                } else {
                    Ok(Validation::Valid)
                }
            }),
        ])
        .with_placeholder(&directory_name)
        .prompt();

    let project_path = match project_path {
        Ok(path) => {
            let path = if path.is_empty() {
                directory_name
            } else {
                path
            };

            PathBuf::from(path)
        }
        Err(_) => {
            println!();
            return;
        }
    };

    let templates: Vec<&str> = vec!["Blog", "Basics", "Empty"];
    let template = Select::new("Which template would you like to use?", templates).prompt();

    let template = match template {
        Ok(template) => template.to_ascii_lowercase(),
        Err(_) => {
            println!();
            return;
        }
    };

    let git = Confirm::new("Do you want to initialize a git repository?")
        .with_default(true)
        .prompt();

    let git = match git {
        Ok(git) => git,
        Err(_) => {
            println!();
            return;
        }
    };

    // Do the steps
    println!();

    // Create the project directory
    let directory_spinner = Spinner::new(" Creating directory")
        .symbols(vec!["◐", "◓", "◑", "◒"])
        .start();

    let start_time = Instant::now();
    if !dry_run {
        std::fs::create_dir_all(&project_path).expect("Failed to create project directory");
    }
    let elasped_time = format_elapsed_time(start_time.elapsed(), &Default::default());

    directory_spinner
        .text(&format!(" Created directory {}", elasped_time))
        .symbol("●")
        .color(Color::Green)
        .stop();

    let template_spinner = Spinner::new(" Downloading template")
        .symbols(vec!["◐", "◓", "◑", "◒"])
        .start();

    let start_time = Instant::now();
    if !dry_run {
        download_and_unpack_template(&template, &project_path)
            .expect("Failed to download template");
    }
    let elasped_time = format_elapsed_time(start_time.elapsed(), &Default::default());

    template_spinner
        .text(&format!(" Downloaded template {}", elasped_time))
        .symbol("●")
        .color(Color::Green)
        .stop();

    if git {
        let git_spinner = Spinner::new(" Initializing git repository")
            .symbols(vec!["◐", "◓", "◑", "◒"])
            .start();

        let start_time = Instant::now();

        let init_result = if !dry_run {
            init_git_repo(&project_path, dry_run)
        } else {
            Ok(())
        };

        let elasped_time = format_elapsed_time(start_time.elapsed(), &Default::default());

        match init_result {
            Ok(_) => git_spinner
                .text(&format!(" Initialized git repository {}", elasped_time))
                .symbol("●")
                .color(Color::Green)
                .stop(),
            Err(e) => {
                git_spinner
                    .text(" Failed to initialize git repository")
                    .failure();
                eprintln!("{}", e);
            }
        }
    }

    println!();

    info!(name: "SKIP_FORMAT", "👑 {} {}! Next steps:", "Project created".bold(), "successfully".green().to_string().bold());
    println!();

    let enter_directory = if project_path != PathBuf::from(".") {
        format!(
            "1. Run {} to enter your project's directory.\n2. ",
            format!("cd {}", project_path.display())
                .bold()
                .bright_blue()
                .underline()
        )
    } else {
        "   ".to_string()
    };

    info!(
        name: "SKIP_FORMAT",
        "{}Run {} to start the development server, {} to stop it.",
        enter_directory,
        "maudit dev".bold().bright_blue().underline(),
        "CTRL+C".bright_blue()
    );
    println!();

    info!(name: "SKIP_FORMAT", "   Visit {} for more information on using Maudit.", "https://maudit.org/docs".bold().bright_magenta().underline());
    info!(name: "SKIP_FORMAT", "   Need a hand? Find us at {}.", "https://maudit.org/chat".bold().bright_magenta().underline());
}

fn download_and_unpack_template(
    template: &str,
    project_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let tarball = ureq::get(REPO_TAR_URL)
        .call()
        .map_err(|e| format!("Failed to download template: {}", e))?;

    if !tarball.status().is_success() {
        return Err("Failed to download template".into());
    }

    let (_, mut body) = tarball.into_parts();

    let archive = GzDecoder::new(body.as_reader());

    // Uncomment to test with a local tarball
    //let archive = std::fs::File::open("project.tar").unwrap();

    let mut archive = tar::Archive::new(archive);

    let example_path = format!("examples/{}", template);
    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.to_string_lossy().to_string();

        if let Some(index) = path.find(&example_path).map(|i| i + example_path.len() + 1) {
            let dest_path = project_path.join(&path[index..]);
            entry.unpack(dest_path)?;
        }
    }

    // Edit the Cargo.toml file
    let cargo_toml_path = project_path.join("Cargo.toml");
    match std::fs::read_to_string(&cargo_toml_path) {
        Ok(content) => {
            let mut cargo_toml = content.parse::<DocumentMut>().expect("invalid doc");

            let project_path = project_path
                .canonicalize()
                .expect("Failed to canonicalize project path");
            if let Some(project_name) = project_path.file_name().and_then(|name| name.to_str()) {
                cargo_toml["package"]["name"] = toml_edit::value(project_name);

                if let toml_edit::Item::Value(v) =
                    &cargo_toml["package"]["metadata"]["maudit"]["intended_version"]
                {
                    cargo_toml["dependencies"]["maudit"] = toml_edit::value(v.clone());
                }

                cargo_toml["package"]["metadata"] = toml_edit::Item::None;

                if let Err(e) = std::fs::write(&cargo_toml_path, cargo_toml.to_string()) {
                    error!("Failed to write Cargo.toml file: {}", e);
                }
            } else {
                error!("Failed to determine project name from path");
            }
        }
        Err(e) => {
            error!("Failed to read Cargo.toml file: {}", e);
        }
    }

    Ok(())
}

fn init_git_repo(project_path: &PathBuf, dry_run: &bool) -> Result<(), String> {
    if !dry_run {
        let git_init = std::process::Command::new("git")
            .arg("init")
            .arg(project_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| format!("Failed to run git init: {}", e))?
            .success();

        if !git_init {
            return Err("Failed to initialize git repository".to_string());
        }

        let git_add = std::process::Command::new("git")
            .arg("add")
            .arg("-A")
            .current_dir(project_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| format!("Failed to run git add: {}", e))?
            .success();

        if !git_add {
            return Err("Failed to add initial changes".to_string());
        }

        let git_commit = std::process::Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg("Initial commit")
            .current_dir(project_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| format!("Failed to run git commit: {}", e))?
            .success();

        if !git_commit {
            return Err("Failed to commit initial changes".to_string());
        }
    }

    Ok(())
}

fn has_invalid_filepath_chars(s: &str) -> bool {
    s.chars().any(|c| {
        c == '\\'
            || c == ':'
            || c == '*'
            || c == '?'
            || c == '"'
            || c == '<'
            || c == '>'
            || c == '|'
    })
}
