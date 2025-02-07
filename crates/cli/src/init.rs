use std::{env, path::PathBuf};

use axum::extract::path;
use colored::Colorize;
use inquire::{required, validator::Validation, Confirm, Select, Text};
use spinach::{Color, Spinner};
use tracing::info;

mod names;
mod render_config;
use names::generate_directory_name;
use render_config::get_render_config;

pub fn start_new_project() {
    inquire::set_global_render_config(get_render_config());

    // Run cargo info maudit in a tmp directory to avoid catching a local version
    let cargo_search = std::process::Command::new("cargo")
        .arg("search")
        .arg("maudit")
        .args(["--limit", "1"])
        .output()
        .expect("Failed to run cargo info maudit");

    let maudit_version = if cargo_search.status.success() {
        let output = String::from_utf8_lossy(&cargo_search.stdout).to_string();
        output
            .lines()
            .next()
            .and_then(|line| {
                let start = line.find('"')?;
                let end = line[start + 1..].find('"')?;
                Some(line[start + 1..start + 1 + end].to_string())
            })
            .unwrap_or_else(|| "unknown".to_string())
    } else {
        "".to_string()
    };

    println!();
    match maudit_version.is_empty() {
        true => {
            info!(name: "SKIP_FORMAT", "👑 {} {}", "Welcome to".bold(), "Maudit".red().to_string().bold(), )
        }
        false => {
            info!(name: "SKIP_FORMAT", "👑 {} {} (v{})", "Welcome to".bold(), "Maudit".red().to_string().bold(), maudit_version)
        }
    }
    info!(name: "SKIP_FORMAT", "   {}", "Let the coronation begin!".dimmed());
    println!();

    let directory_name = format!("./{}", generate_directory_name());
    let project_path = Text::new("Where should we create the project?")
        .with_formatter(&|i| {
            if i.is_empty() {
                return directory_name.clone();
            }

            i.to_owned()
        })
        .with_validators(&[
            Box::new(|s: &str| {
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
        Ok(template) => template,
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
    std::fs::create_dir_all(&project_path).expect("Failed to create project directory");
    directory_spinner
        .text(" Created directory")
        .symbol("●")
        .color(Color::Green)
        .stop();

    if git {
        let git_spinner = Spinner::new(" Initializing git repository")
            .symbols(vec!["◐", "◓", "◑", "◒"])
            .start();
        let init_result = init_git_repo(project_path);

        match init_result {
            Ok(_) => git_spinner
                .text(" Initialized git repository")
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
}

fn init_git_repo(project_path: PathBuf) -> Result<(), String> {
    let git_init = std::process::Command::new("git")
        .arg("init")
        .arg(&project_path)
        .status()
        .map_err(|e| format!("Failed to run git init: {}", e))?
        .success();

    if !git_init {
        return Err("Failed to initialize git repository".to_string());
    }

    let git_add = std::process::Command::new("git")
        .arg("add")
        .arg("-A")
        .current_dir(&project_path)
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
        .current_dir(&project_path)
        .status()
        .map_err(|e| format!("Failed to run git commit: {}", e))?
        .success();

    if !git_commit {
        return Err("Failed to commit initial changes".to_string());
    }

    Ok(())
}

fn has_invalid_filepath_chars(s: &str) -> bool {
    s.chars().any(|c| {
        c == '/'
            || c == '\\'
            || c == ':'
            || c == '*'
            || c == '?'
            || c == '"'
            || c == '<'
            || c == '>'
            || c == '|'
    })
}
