use std::borrow::Cow;

pub enum Shaders {
    Skybox,
    Fallback,
}

impl Shaders {
    pub fn source(self) -> Cow<'static, str> {
        match self {
            Shaders::Skybox => format!(
                "{}\n{}",
                include_str!("shaders/common.wgsl"),
                include_str!("shaders/skybox.wgsl")
            )
            .into(),
            Self::Fallback => include_str!("shaders/fallback.wgsl").into(),
        }
    }

    pub fn source_fs(self) -> Cow<'static, str> {
        match self {
            Shaders::Skybox => format!(
                "{}\n{}",
                std::fs::read_to_string("src/shaders/common.wgsl").unwrap_or_default(),
                std::fs::read_to_string("src/shaders/skybox.wgsl").unwrap_or_default()
            )
            .into(),
            Shaders::Fallback => include_str!("shaders/fallback.wgsl").into(),
        }
    }
}
