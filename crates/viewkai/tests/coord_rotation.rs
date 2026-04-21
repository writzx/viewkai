//! Coordinate rotation helpers.

use viewkai_core::{
    PageSize, PdfPageRotation, PointsPos, inverse_rotate_point,
};

fn page_size() -> PageSize {
    PageSize {
        width_pt: 612.0,
        height_pt: 792.0,
    }
}

#[test]
fn inverse_rotate_point_none_is_identity() {
    let point = PointsPos { x: 10.0, y: 20.0 };
    assert_eq!(inverse_rotate_point(point, PdfPageRotation::None, page_size()), point);
}

#[test]
fn inverse_rotate_point_r90_maps_to_pdf_coords() {
    assert_eq!(
        inverse_rotate_point(PointsPos { x: 100.0, y: 50.0 }, PdfPageRotation::R90, page_size()),
        PointsPos { x: 50.0, y: 512.0 }
    );
}

#[test]
fn inverse_rotate_point_r180_maps_to_pdf_coords() {
    assert_eq!(
        inverse_rotate_point(PointsPos { x: 100.0, y: 50.0 }, PdfPageRotation::R180, page_size()),
        PointsPos { x: 512.0, y: 742.0 }
    );
}

#[test]
fn inverse_rotate_point_r270_maps_to_pdf_coords() {
    assert_eq!(
        inverse_rotate_point(PointsPos { x: 100.0, y: 50.0 }, PdfPageRotation::R270, page_size()),
        PointsPos { x: 742.0, y: 100.0 }
    );
}
