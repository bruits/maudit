use colored::Colorize;
use inquire::{required, validator::Validation, Text};
use rand::seq::IndexedRandom;
use tracing::info;

mod names;
use names::{ADJECTIVE, NAME, TITLE};

pub fn start_new_project() {
    info!(name: "SKIP_FORMAT", "");
    info!(name: "SKIP_FORMAT", "  {}", "Maudit 👑".red().to_string().bold());

    let directory_name = format!("./{}", generate_directory_name());
    let name = Text::new("Where should we create the project?")
        .with_validators(&[
            Box::new(required!("This field is required")),
            Box::new(|s: &str| {
                if std::path::Path::new(&s).exists() {
                    Ok(Validation::Invalid(
                        "A directory with this name already exists".into(),
                    ))
                } else {
                    Ok(Validation::Valid)
                }
            }),
        ])
        .with_initial_value(&directory_name)
        .with_placeholder(&directory_name)
        .prompt();
}

pub fn generate_directory_name() -> String {
    let mut rng = rand::rng();

    let title = TITLE.choose(&mut rng).unwrap();
    let name = NAME.choose(&mut rng).unwrap();
    let adjective = ADJECTIVE.choose(&mut rng).unwrap();

    format!("{}-{}-{}", adjective, title, name)
}
