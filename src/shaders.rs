use std::borrow::Cow;

pub enum Shaders {
    Clouds,
}

impl Shaders {
    pub fn source(self) -> Cow<'static, str> {
        match self {
            Shaders::Clouds => format!(
                "{}\n{}",
                include_str!("shaders/math.wgsl"),
                include_str!("shaders/clouds.wgsl")
            )
            .into(),
        }
    }
}
