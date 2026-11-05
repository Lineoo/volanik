use std::borrow::Cow;

pub enum Shaders {
    Skybox,
}

impl Shaders {
    pub fn source(self) -> Cow<'static, str> {
        match self {
            Shaders::Skybox => format!(
                "{}\n{}",
                include_str!("shaders/math.wgsl"),
                include_str!("shaders/skybox.wgsl")
            )
            .into(),
        }
    }
}
