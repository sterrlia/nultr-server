use once_cell::sync::Lazy;
use std::env;

macro_rules! env_lazy {
    ($name:ident, $type:ty) => {
        pub static $name: Lazy<$type> = Lazy::new(|| {
            env::var(stringify!($name))
                .expect(concat!(stringify!($name), " is not set"))
                .parse::<$type>()
                .expect(concat!(
                    stringify!($name),
                    " must be a valid ",
                    stringify!($type)
                ))
        });
    };
}

env_lazy!(DATABASE_URL, String);
env_lazy!(WS_URL, String);
env_lazy!(JWT_SECRET_KEY, String);
