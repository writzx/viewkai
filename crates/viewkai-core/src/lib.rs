pub const NAME: &str = "viewkai-core";

pub mod error {
    use thiserror::Error;

    #[derive(Debug, Error)]
    pub enum ViewkaiCoreError {
        #[error("viewkai-core placeholder error")]
        Placeholder,
    }
}

pub mod geometry {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
    pub struct PointsRect {
        pub min_x: f32,
        pub min_y: f32,
        pub max_x: f32,
        pub max_y: f32,
    }
}

pub mod types {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub struct PageIndex(pub u32);
}
