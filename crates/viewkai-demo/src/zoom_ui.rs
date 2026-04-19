use eframe::egui;
use viewkai::{
    Viewer,
    zoom::ZoomState,
};

const DISCRETE_LEVELS: [f32; 9] = [0.25, 0.50, 0.75, 1.0, 1.25, 1.5, 2.0, 3.0, 4.0];
const ZOOM_EPSILON: f32 = 0.01;

fn zoom_levels() -> [(&'static str, ZoomState); 11] {
    [
        ("25%", ZoomState::Discrete(0.25)),
        ("50%", ZoomState::Discrete(0.50)),
        ("75%", ZoomState::Discrete(0.75)),
        ("100%", ZoomState::Discrete(1.0)),
        ("125%", ZoomState::Discrete(1.25)),
        ("150%", ZoomState::Discrete(1.50)),
        ("200%", ZoomState::Discrete(2.0)),
        ("300%", ZoomState::Discrete(3.0)),
        ("400%", ZoomState::Discrete(4.0)),
        ("Fit Width", ZoomState::FitWidth),
        ("Fit Page", ZoomState::FitPage),
    ]
}

fn zoom_label(zoom: ZoomState) -> &'static str {
    match zoom {
        ZoomState::FitWidth => "Fit Width",
        ZoomState::FitPage => "Fit Page",
        ZoomState::Discrete(z) | ZoomState::Custom(z) => zoom_levels()
            .iter()
            .find(|(_, lvl)| matches!(lvl, ZoomState::Discrete(d) if (z - d).abs() < ZOOM_EPSILON))
            .map_or("Custom", |(label, _)| *label),
    }
}

pub(crate) fn step_zoom_up(current: ZoomState) -> ZoomState {
    let z = match current {
        ZoomState::Discrete(z) | ZoomState::Custom(z) => z,
        ZoomState::FitWidth | ZoomState::FitPage => 1.0,
    };
    let next = DISCRETE_LEVELS
        .iter()
        .find(|&&level| level > z + ZOOM_EPSILON)
        .copied();
    ZoomState::Discrete(next.unwrap_or(4.0))
}

pub(crate) fn step_zoom_down(current: ZoomState) -> ZoomState {
    let z = match current {
        ZoomState::Discrete(z) | ZoomState::Custom(z) => z,
        ZoomState::FitWidth | ZoomState::FitPage => 1.0,
    };
    let previous = DISCRETE_LEVELS
        .iter()
        .rev()
        .find(|&&level| level < z - ZOOM_EPSILON)
        .copied();
    ZoomState::Discrete(previous.unwrap_or(0.25))
}

pub(crate) fn zoom_toolbar_ui(ui: &mut egui::Ui, viewer: &mut Viewer) -> bool {
    let mut zoom_changed = false;

    if ui.button("−").clicked() {
        viewer.set_zoom(step_zoom_down(viewer.zoom()));
        zoom_changed = true;
    }

    egui::ComboBox::from_id_salt("zoom_combo")
        .selected_text(zoom_label(viewer.zoom()))
        .show_ui(ui, |ui| {
            for (label, zoom) in zoom_levels() {
                if ui.selectable_label(false, label).clicked() {
                    viewer.set_zoom(zoom);
                    zoom_changed = true;
                }
            }
        });

    if ui.button("+").clicked() {
        viewer.set_zoom(step_zoom_up(viewer.zoom()));
        zoom_changed = true;
    }

    zoom_changed
}
