
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DotShape {
    #[default]
    Circle,
    Ring,
    Cross,
}

#[derive(Clone, Debug)]
pub struct DotStyle {
    pub color: [f32; 4],
    pub radius_px: f32,
    pub shape: DotShape,
    pub label: Option<String>,
    pub selected: bool,
    pub visible: bool,
}

impl Default for DotStyle {
    fn default() -> Self {
        Self {
            color: [1.0, 1.0, 1.0, 1.0],
            radius_px: 3.8,
            shape: DotShape::Circle,
            label: None,
            selected: false,
            visible: true,
        }
    }
}


#[derive(Clone, Debug)]
pub struct DdotStyle {
    pub color: [f32; 4],
    pub radius_px: f32,
    pub shape: DotShape,
    pub label: Option<String>,
    pub selected: bool,
    pub visible: bool,
}

impl Default for DdotStyle {
    fn default() -> Self {
        Self {
            color: [1.0, 1.0, 1.0, 1.0],
            radius_px: 3.8,
            shape: DotShape::Circle,
            label: None,
            selected: false,
            visible: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct LineStyle {
    pub color: [f32; 4],
    pub width_px: f32,
    pub label: Option<String>,
    pub selected: bool,
    pub visible: bool,
}

impl Default for LineStyle {
    fn default() -> Self {
        Self {
            color: [1.0, 1.0, 1.0, 1.0],
            width_px: 1.6,
            label: None,
            selected: false,
            visible: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ConicStyle {
    pub color: [f32; 4],
    pub width_px: f32,
    pub label: Option<String>,
    pub selected: bool,
    pub visible: bool,
}

impl Default for ConicStyle {
    fn default() -> Self {
        Self {
            color: [1.0, 1.0, 1.0, 1.0],
            width_px: 1.6,
            label: None,
            selected: false,
            visible: true,
        }
    }
}
