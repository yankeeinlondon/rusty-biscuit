use model_id::ModelId;

#[derive(ModelId)]
pub enum BadAttribute {
    #[model_id(not_a_string)]
    InvalidVariant,
}

fn main() {}
