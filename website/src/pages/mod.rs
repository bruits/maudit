macro_rules! pub_mod {
    ($($mod:ident),*) => {
        $(
            mod $mod;
            pub use $mod::*;
        )*
    };
}

pub_mod!(index, docs, chat, news, contribute);

#[path = "404.rs"]
mod not_found;
pub use not_found::NotFound;
