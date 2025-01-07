use std::hash::Hasher;

pub fn generate_build_id() {
    let value =
        std::hash::BuildHasher::build_hasher(&std::collections::hash_map::RandomState::new())
            .finish();

    println!("cargo:rustc-env=MAUDIT_BUILD_ID={}", value);
}
