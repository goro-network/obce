use obce::id;

mod nested {
    #[obce::definition]
    pub trait Trait {
        #[obce(id = "named-extension-method")]
        fn extension_method(&self);
    }
}

fn main() {
    id!(nested::Trait::extension_method);
}
